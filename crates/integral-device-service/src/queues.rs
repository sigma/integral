//! The client work waiting for the device's queue: one queue per client,
//! served in turn.
//!
//! A client's items keep their order: the service takes the first item of one
//! client's queue at a time, the client served least recently first, so
//! sixteen plugin instances that each push a tone fill in together instead of
//! one after another, a client that has been idle (a fader that starts to
//! move) is served at the next turn, and a write a client queues after its
//! push is applied after the push. Between clients there is no order, except
//! that a write replaces the earlier write it covers, whoever wrote it
//! ([`coalesce`], the rule of the device's own queue).

use std::collections::{BTreeMap, VecDeque};
use std::ops::Range;

use integral_core::device::{Coalesce, coalesce};

use crate::protocol::{ClientId, Request};

/// What a queued item does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Work {
    /// A checked request.
    Request(Request),
    /// The end of a client's [`Request::Push`]: tell it the push was handed
    /// to the device's queue.
    PushDone,
    /// Request the next block of the baseline being read for `part`: queued
    /// by the service itself, in front of the items of the client whose
    /// select started it, so that client's later items wait for the tone.
    /// Ignored if the reading it was for is gone (`generation` differs).
    ReadBaseline {
        /// The Part.
        part: u8,
        /// Which reading it was queued for.
        generation: u64,
    },
}

/// A checked item of a client waiting for the device's queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pending {
    /// Who asked, and is announced as the writer.
    pub client: ClientId,
    /// What to do.
    pub work: Work,
    /// The addresses it writes, if it is a write that may be replaced by a
    /// later one (never for an item of a push).
    pub span: Option<Range<u32>>,
    /// Part of a push: not replaced, and nothing queued after it is moved
    /// before it.
    pub push: bool,
    /// Sent even if the session knows the device has the value: a push, but
    /// not a write to a tone that the push selected earlier and the service
    /// has read back.
    pub force: bool,
}

/// A queued item and its place in arrival order.
struct Item {
    pending: Pending,
    /// Grows with arrival across all clients; a queue is sorted by it.
    seq: i64,
}

/// Every client's queue.
#[derive(Default)]
pub(crate) struct Queues {
    queues: BTreeMap<ClientId, VecDeque<Item>>,
    /// When each client was last served (a tick), for the ones that ever were.
    served: BTreeMap<ClientId, u64>,
    tick: u64,
    next_seq: i64,
    /// The seq the next item put in front takes; below every other.
    front_seq: i64,
}

impl Queues {
    /// Whether nothing waits.
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.queues.is_empty()
    }

    /// Take everything, a client's items together, in the order they would
    /// have been served within it.
    pub(crate) fn drain(&mut self) -> Vec<Pending> {
        std::mem::take(&mut self.queues)
            .into_values()
            .flatten()
            .map(|item| item.pending)
            .collect()
    }

    /// The next item: the first of the queue of the client served least
    /// recently (ties by client ID) among those `blocked` does not hold.
    pub(crate) fn pop(&mut self, blocked: impl Fn(ClientId) -> bool) -> Option<Pending> {
        let next = self
            .queues
            .keys()
            .copied()
            .filter(|client| !blocked(*client))
            .min_by_key(|client| (self.served.get(client).copied().unwrap_or(0), *client))?;
        self.tick += 1;
        self.served.insert(next, self.tick);
        let queue = self.queues.get_mut(&next)?;
        let item = queue.pop_front()?;
        if queue.is_empty() {
            self.queues.remove(&next);
        }
        Some(item.pending)
    }

    /// Forget how long ago a client was served (it went away for good).
    pub(crate) fn forget(&mut self, client: ClientId) {
        self.served.remove(&client);
    }

    /// Queue `pending` in front of its client's other items.
    pub(crate) fn push_front(&mut self, pending: Pending) {
        // Below every seq in use, so a queue stays sorted by it (the binary
        // searches that find an item rely on that).
        self.front_seq -= 1;
        let seq = self.front_seq;
        self.queues
            .entry(pending.client)
            .or_default()
            .push_front(Item { pending, seq });
    }

    /// Queue `pending` behind its client's earlier items.
    pub(crate) fn push(&mut self, pending: Pending) {
        let seq = self.take_seq();
        self.queues
            .entry(pending.client)
            .or_default()
            .push_back(Item { pending, seq });
    }

    /// Queue a write (or a message that is not one), letting it replace the
    /// earlier writes it covers, or a tone select the client's last item
    /// selected again: looking back over every client's items from
    /// the newest, it passes unrelated ones and stops at an overlap, a
    /// select, a message that is not a write, or an item of a push.
    ///
    /// The surviving write takes the place of the oldest replaced item of its
    /// own client, so a fader dragged faster than the device takes writes
    /// keeps its turn. If the replaced items are all other clients', they are
    /// dropped and the write goes to the end of its own queue: it never
    /// moves into another client's queue, where it could pass an earlier
    /// item of its writer. The cost is that a write that takes over another
    /// client's pending write queues behind its own client's items, so it
    /// may go out later than the write it replaced would have.
    pub(crate) fn enqueue(&mut self, pending: Pending) {
        if let Work::Request(Request::ChangeTone { part, .. }) = &pending.work
            && let Some(last) = self
                .queues
                .get_mut(&pending.client)
                .and_then(VecDeque::back_mut)
            && !last.pending.push
            && matches!(&last.pending.work, Work::Request(Request::ChangeTone { part: p, .. }) if p == part)
        {
            // Not yet applied, so the earlier select never reaches the
            // device: five quick selects on one Part cost one.
            last.pending = pending;
            return;
        }
        let Some(new) = pending.span.clone() else {
            return self.push(pending);
        };
        let replaced = self.covered(&new);
        let own = pending.client;
        let slot = replaced.iter().rev().find(|(client, _)| *client == own);
        let slot = slot.map(|(_, seq)| *seq);
        for (client, seq) in replaced {
            if Some(seq) != slot {
                self.remove(client, seq);
            }
        }
        match slot.and_then(|seq| self.find(own, seq)) {
            Some(item) => item.pending = pending,
            None => self.push(pending),
        }
    }

    fn take_seq(&mut self) -> i64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        seq
    }

    /// The items a write of `new` replaces, newest first, as (client, seq).
    fn covered(&self, new: &Range<u32>) -> Vec<(ClientId, i64)> {
        // A cursor per queue, each at the end of what it has not yet shown.
        let mut cursors: Vec<(ClientId, &VecDeque<Item>, usize)> = self
            .queues
            .iter()
            .map(|(client, queue)| (*client, queue, queue.len()))
            .collect();
        let mut replaced = Vec::new();
        loop {
            let Some(cursor) = cursors
                .iter_mut()
                .filter(|(_, _, left)| *left > 0)
                .max_by_key(|(_, queue, left)| queue[*left - 1].seq)
            else {
                break;
            };
            cursor.2 -= 1;
            let item = &cursor.1[cursor.2];
            if item.pending.push {
                break;
            }
            match coalesce(new, item.pending.span.as_ref()) {
                Coalesce::Replace => replaced.push((cursor.0, item.seq)),
                Coalesce::PassOver => {}
                Coalesce::Stop => break,
            }
        }
        replaced
    }

    fn find(&mut self, client: ClientId, seq: i64) -> Option<&mut Item> {
        let queue = self.queues.get_mut(&client)?;
        let at = queue.binary_search_by_key(&seq, |item| item.seq).ok()?;
        queue.get_mut(at)
    }

    fn remove(&mut self, client: ClientId, seq: i64) {
        let Some(queue) = self.queues.get_mut(&client) else {
            return;
        };
        if let Ok(at) = queue.binary_search_by_key(&seq, |item| item.seq) {
            queue.remove(at);
        }
        if queue.is_empty() {
            self.queues.remove(&client);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(client: u64, span: Range<u32>, tag: u8) -> Pending {
        Pending {
            client: ClientId(client),
            work: Work::Request(Request::Raw { bytes: vec![tag] }),
            span: Some(span),
            push: false,
            force: false,
        }
    }

    fn tag(pending: &Pending) -> u8 {
        match &pending.work {
            Work::Request(Request::Raw { bytes }) => bytes[0],
            _ => panic!("not a test item"),
        }
    }

    fn drain(queues: &mut Queues) -> Vec<(u64, u8)> {
        std::iter::from_fn(|| queues.pop(|_| false))
            .map(|p| (p.client.0, tag(&p)))
            .collect()
    }

    #[test]
    fn clients_are_served_in_turn_each_in_its_own_order() {
        let mut queues = Queues::default();
        for i in 0..3 {
            queues.enqueue(write(1, i * 10..i * 10 + 1, i as u8));
        }
        for i in 0..2 {
            queues.enqueue(write(2, 100 + i..101 + i, 10 + i as u8));
        }
        assert_eq!(
            drain(&mut queues),
            [(1, 0), (2, 10), (1, 1), (2, 11), (1, 2)]
        );
        assert!(queues.is_empty());
    }

    #[test]
    fn a_client_that_was_idle_is_served_before_one_that_was_just_served() {
        let mut queues = Queues::default();
        queues.enqueue(write(5, 0..1, 0));
        queues.enqueue(write(5, 10..11, 1));
        assert_eq!(queues.pop(|_| false).map(|p| p.client.0), Some(5));
        queues.enqueue(write(2, 20..21, 2));
        // 5 was just served and 2 never was.
        assert_eq!(drain(&mut queues), [(2, 2), (5, 1)]);
    }

    #[test]
    fn a_write_replaces_the_earlier_one_it_covers_in_place() {
        let mut queues = Queues::default();
        queues.enqueue(write(1, 0..1, 0));
        queues.enqueue(write(1, 10..11, 1));
        queues.enqueue(write(1, 0..1, 2));
        assert_eq!(drain(&mut queues), [(1, 2), (1, 1)]);
    }

    #[test]
    fn another_clients_write_is_replaced_and_the_new_one_stays_in_its_own_queue() {
        let mut queues = Queues::default();
        queues.enqueue(write(1, 0..1, 0));
        queues.enqueue(write(2, 50..51, 1));
        queues.enqueue(write(2, 0..1, 2)); // covers client 1's
        assert_eq!(drain(&mut queues), [(2, 1), (2, 2)]);
    }

    #[test]
    fn a_write_replaces_what_it_covers_but_not_across_an_overlap() {
        let mut queues = Queues::default();
        queues.enqueue(write(1, 0..3, 0));
        queues.enqueue(write(2, 1..2, 1));
        // Covers client 2's write; client 1's first write overlaps it
        // without being covered, so the look back stops there.
        queues.enqueue(write(1, 1..2, 2));
        assert_eq!(drain(&mut queues), [(1, 0), (1, 2)]);
    }

    #[test]
    fn nothing_is_replaced_across_an_item_of_a_push() {
        let mut queues = Queues::default();
        queues.enqueue(write(2, 0..1, 0));
        queues.push(Pending {
            push: true,
            span: None,
            ..write(1, 0..1, 1)
        });
        queues.enqueue(write(1, 0..1, 2));
        assert_eq!(drain(&mut queues), [(1, 1), (2, 0), (1, 2)]);
    }

    #[test]
    fn an_item_that_is_not_a_write_stops_the_look_back() {
        let mut queues = Queues::default();
        queues.enqueue(write(1, 0..1, 0));
        queues.enqueue(Pending {
            span: None,
            ..write(1, 0..1, 1)
        });
        queues.enqueue(write(1, 0..1, 2));
        assert_eq!(drain(&mut queues), [(1, 0), (1, 1), (1, 2)]);
    }

    fn select(client: u64, part: u8, pc: u8) -> Pending {
        Pending {
            client: ClientId(client),
            work: Work::Request(Request::ChangeTone {
                part,
                msb: 87,
                lsb: 64,
                pc,
            }),
            span: None,
            push: false,
            force: false,
        }
    }

    #[test]
    fn a_select_replaces_the_one_queued_just_before_for_the_same_part() {
        let mut queues = Queues::default();
        for pc in 0..5 {
            queues.enqueue(select(1, 2, pc));
        }
        queues.enqueue(select(1, 3, 0));
        queues.enqueue(select(1, 2, 9));
        let pcs: Vec<_> = queues
            .drain()
            .into_iter()
            .map(|p| match p.work {
                Work::Request(Request::ChangeTone { part, pc, .. }) => (part, pc),
                _ => panic!(),
            })
            .collect();
        assert_eq!(pcs, [(2, 4), (3, 0), (2, 9)]);
    }

    #[test]
    fn items_pushed_to_the_front_go_first_and_a_blocked_client_is_skipped() {
        let mut queues = Queues::default();
        queues.enqueue(write(1, 0..1, 0));
        queues.enqueue(write(2, 10..11, 1));
        queues.push_front(write(1, 20..21, 2));
        assert_eq!(queues.pop(|c| c == ClientId(2)).map(|p| tag(&p)), Some(2));
        assert_eq!(queues.pop(|c| c == ClientId(1)).map(|p| tag(&p)), Some(1));
        assert!(queues.pop(|c| c == ClientId(1)).is_none());
        assert_eq!(queues.pop(|_| false).map(|p| tag(&p)), Some(0));
    }
}
