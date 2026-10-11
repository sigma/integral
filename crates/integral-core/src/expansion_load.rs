//! Loading Expansions into the Expansion slots.
//!
//! An RQ1 at `0F 00 30 00` whose size field carries the four target slot
//! values starts a load. The device answers `0F 00 30 01` when it starts
//! and `0F 00 30 02` when it is done (~5 s per Expansion). It cuts all sound
//! and ignores RQ1s until done.
//!
//! [`LoadMachine`] drives one load without doing I/O: the host sends what
//! [`LoadMachine::next_action`] asks for and feeds every DT1 back through
//! [`LoadMachine::on_dt1`].
//!
//! Reference: `docs/midi/99-undocumented.md`, section 5.

use crate::address::{Address, DataSize};
use crate::expansion_slots::{self, ExpansionSlots, SLOT_COUNT};
use crate::expansions::{self, Expansion};
use crate::sysex;

/// Address of the load request.
const LOAD_REQUEST_ADDR: Address = Address::new(0x0F, 0x00, 0x30, 0x00);
/// Address of the "load started" reply.
const LOAD_STARTED_ADDR: Address = Address::new(0x0F, 0x00, 0x30, 0x01);
/// Address of the "load done" reply.
const LOAD_DONE_ADDR: Address = Address::new(0x0F, 0x00, 0x30, 0x02);

/// Interval between slot reads once the load has started (ms).
pub const POLL_INTERVAL_MS: f64 = 500.0;
/// Time without any answer after which the load is declared timed out (ms).
pub const TIMEOUT_MS: f64 = 60_000.0;

/// Build the RQ1 that loads `target` into the slots.
pub fn build_load_request(device_id: u8, target: &ExpansionSlots) -> Vec<u8> {
    let [a, b, c, d] = target.values();
    sysex::build_rq1(device_id, &LOAD_REQUEST_ADDR, &DataSize::new(a, b, c, d))
}

/// One slot whose content a load changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotChange {
    /// Slot index (0 = A).
    pub slot: usize,
    /// What it holds now (`None` = OFF).
    pub from: Option<&'static Expansion>,
    /// What it will hold (`None` = OFF).
    pub to: Option<&'static Expansion>,
}

/// What a load would change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadPlan {
    /// Slots that change, in slot order. Empty for a no-op.
    pub changes: Vec<SlotChange>,
    /// Parts (1-based) on a loaded Expansion that the load unloads.
    pub silenced_parts: Vec<u8>,
}

/// Plan loading `target` over `current`.
///
/// `part_banks` holds each Part's tone bank (MSB, LSB), Part 1 first.
pub fn plan(
    current: &ExpansionSlots,
    target: &ExpansionSlots,
    part_banks: &[(u8, u8)],
) -> LoadPlan {
    let changes = (0..SLOT_COUNT)
        .filter(|&slot| current.0[slot] != target.0[slot])
        .map(|slot| SlotChange {
            slot,
            from: current.0[slot],
            to: target.0[slot],
        })
        .collect();
    let silenced_parts = (1..)
        .zip(part_banks)
        .filter(|(_, (msb, lsb))| {
            expansions::expansion_of_bank(*msb, *lsb)
                .is_some_and(|e| current.contains(e) && !target.contains(e))
        })
        .map(|(part, _)| part)
        .collect();
    LoadPlan {
        changes,
        silenced_parts,
    }
}

/// How a load ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadOutcome {
    /// The target slots were already loaded; nothing was sent.
    NoOp,
    /// The slots now hold the target.
    Loaded(ExpansionSlots),
    /// The device read back other slots than the target.
    Failed(ExpansionSlots),
    /// No answer of any kind for [`TIMEOUT_MS`].
    TimedOut,
}

/// What the host should do next.
#[derive(Debug, Clone, PartialEq)]
pub enum LoadAction {
    /// Send these bytes now (the load request).
    Send(Vec<u8>),
    /// Send these bytes now (a slot read).
    Poll(Vec<u8>),
    /// Nothing to send; ask again after this many ms or on the next DT1.
    Wait(f64),
    /// The load is over.
    Done(LoadOutcome),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    Pending,
    AwaitingAck,
    Polling { last_poll: f64 },
    Done(LoadOutcome),
}

/// Drives one Expansion load. Pure: time comes from the host.
#[derive(Debug, Clone)]
pub struct LoadMachine {
    device_id: u8,
    target: ExpansionSlots,
    phase: Phase,
    last_answer: f64,
}

impl LoadMachine {
    /// Prepare loading `target` over `current`.
    pub fn new(device_id: u8, current: ExpansionSlots, target: ExpansionSlots) -> Self {
        let phase = if current == target {
            Phase::Done(LoadOutcome::NoOp)
        } else {
            Phase::Pending
        };
        Self {
            device_id,
            target,
            phase,
            last_answer: 0.0,
        }
    }

    /// The slots being loaded.
    pub fn target(&self) -> &ExpansionSlots {
        &self.target
    }

    /// The outcome, once the load is over.
    pub fn outcome(&self) -> Option<LoadOutcome> {
        match self.phase {
            Phase::Done(outcome) => Some(outcome),
            _ => None,
        }
    }

    /// The next step at monotonic time `now_ms`.
    pub fn next_action(&mut self, now_ms: f64) -> LoadAction {
        let since_answer = now_ms - self.last_answer;
        match self.phase {
            Phase::Done(outcome) => LoadAction::Done(outcome),
            Phase::Pending => {
                self.phase = Phase::AwaitingAck;
                self.last_answer = now_ms;
                LoadAction::Send(build_load_request(self.device_id, &self.target))
            }
            _ if since_answer >= TIMEOUT_MS => {
                self.phase = Phase::Done(LoadOutcome::TimedOut);
                LoadAction::Done(LoadOutcome::TimedOut)
            }
            Phase::AwaitingAck => LoadAction::Wait(TIMEOUT_MS - since_answer),
            Phase::Polling { last_poll } => {
                let since_poll = now_ms - last_poll;
                if since_poll >= POLL_INTERVAL_MS {
                    self.phase = Phase::Polling { last_poll: now_ms };
                    LoadAction::Poll(expansion_slots::build_read_request(self.device_id))
                } else {
                    LoadAction::Wait((POLL_INTERVAL_MS - since_poll).min(TIMEOUT_MS - since_answer))
                }
            }
        }
    }

    /// Feed an incoming DT1. Returns `true` if it belonged to the load.
    ///
    /// Slot reads answered before the "started" ack are stale and consumed
    /// without effect.
    pub fn on_dt1(&mut self, address: &Address, data: &[u8], now_ms: f64) -> bool {
        if matches!(self.phase, Phase::Pending | Phase::Done(_)) {
            return false;
        }
        if *address == LOAD_DONE_ADDR {
            self.phase = Phase::Done(LoadOutcome::Loaded(self.target));
            return true;
        }
        if *address == LOAD_STARTED_ADDR {
            if self.phase == Phase::AwaitingAck {
                self.last_answer = now_ms;
                self.phase = Phase::Polling { last_poll: now_ms };
            }
            return true;
        }
        let Some(reply) = expansion_slots::parse_read_reply(address, data) else {
            return false;
        };
        if let (Phase::Polling { .. }, Ok(slots)) = (self.phase, reply) {
            self.phase = Phase::Done(if slots == self.target {
                LoadOutcome::Loaded(slots)
            } else {
                LoadOutcome::Failed(slots)
            });
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expansion_slots::SLOT_READ_ADDR;

    fn slots(names: [&str; SLOT_COUNT]) -> ExpansionSlots {
        ExpansionSlots(names.map(expansions::by_name))
    }

    fn current() -> ExpansionSlots {
        slots(["SRX-03", "SRX-05", "SRX-07", "SRX-08"])
    }

    fn target() -> ExpansionSlots {
        slots(["SRX-01", "SRX-05", "ExSN3", "SRX-08"])
    }

    /// Start a load at t=0 and ack it at t=70.
    fn started() -> LoadMachine {
        let mut m = LoadMachine::new(0x10, current(), target());
        assert!(matches!(m.next_action(0.0), LoadAction::Send(_)));
        assert_eq!(m.next_action(10.0), LoadAction::Wait(TIMEOUT_MS - 10.0));
        assert!(m.on_dt1(&LOAD_STARTED_ADDR, &[0; 4], 70.0));
        m
    }

    #[test]
    fn builds_documented_load_request() {
        let target = slots(["SRX-03", "SRX-05", "SRX-07", "SRX-01"]);
        assert_eq!(
            build_load_request(0x10, &target),
            [
                0xF0, 0x41, 0x10, 0x00, 0x00, 0x64, 0x11, 0x0F, 0x00, 0x30, 0x00, 0x03, 0x05, 0x07,
                0x01, 0x31, 0xF7
            ]
        );
    }

    #[test]
    fn expcm_load_sends_13_in_every_slot() {
        let target = slots(["ExPCM"; 4]);
        let mut m = LoadMachine::new(0x10, current(), target);
        let LoadAction::Send(bytes) = m.next_action(0.0) else {
            panic!("expected Send");
        };
        assert_eq!(bytes[11..15], [0x13; 4]);
        m.on_dt1(&LOAD_STARTED_ADDR, &[0; 4], 70.0);
        m.next_action(570.0);
        m.on_dt1(&SLOT_READ_ADDR, &[0x13, 0x14, 0x15, 0x16], 20_000.0);
        assert_eq!(m.outcome(), Some(LoadOutcome::Loaded(target)));
    }

    #[test]
    fn plan_lists_changes_and_silenced_parts() {
        // Part 1 on SRX-03 (unloaded), Part 2 on SRX-05 (kept), Part 3 on
        // a preset, Part 4 on ExSN3 (not loaded before).
        let parts = [(93, 2), (93, 4), (89, 64), (89, 98)];
        let p = plan(&current(), &target(), &parts);
        assert_eq!(
            p.changes,
            [
                SlotChange {
                    slot: 0,
                    from: expansions::by_name("SRX-03"),
                    to: expansions::by_name("SRX-01"),
                },
                SlotChange {
                    slot: 2,
                    from: expansions::by_name("SRX-07"),
                    to: expansions::by_name("ExSN3"),
                },
            ]
        );
        assert_eq!(p.silenced_parts, [1]);
    }

    #[test]
    fn noop_sends_nothing() {
        let mut m = LoadMachine::new(0x10, current(), current());
        assert_eq!(m.next_action(0.0), LoadAction::Done(LoadOutcome::NoOp));
        assert!(plan(&current(), &current(), &[]).changes.is_empty());
    }

    #[test]
    fn success_via_done_dt1() {
        let mut m = started();
        assert_eq!(m.next_action(100.0), LoadAction::Wait(470.0));
        assert!(matches!(m.next_action(570.0), LoadAction::Poll(_)));
        assert!(m.on_dt1(&LOAD_DONE_ADDR, &[0; 4], 5_000.0));
        assert_eq!(
            m.next_action(5_000.0),
            LoadAction::Done(LoadOutcome::Loaded(target()))
        );
    }

    #[test]
    fn success_via_poll_with_done_dt1_lost() {
        let mut m = started();
        assert!(matches!(m.next_action(570.0), LoadAction::Poll(_)));
        assert!(matches!(m.next_action(1_070.0), LoadAction::Poll(_)));
        assert!(m.on_dt1(&SLOT_READ_ADDR, &target().values(), 5_100.0));
        assert_eq!(m.outcome(), Some(LoadOutcome::Loaded(target())));
    }

    #[test]
    fn failure_when_read_answers_other_slots() {
        let mut m = started();
        m.next_action(570.0);
        let actual = slots(["SRX-01", "SRX-05", "SRX-07", "SRX-08"]);
        m.on_dt1(&SLOT_READ_ADDR, &actual.values(), 5_100.0);
        assert_eq!(m.outcome(), Some(LoadOutcome::Failed(actual)));
    }

    #[test]
    fn stale_read_before_ack_is_ignored() {
        let mut m = LoadMachine::new(0x10, current(), target());
        m.next_action(0.0);
        assert!(m.on_dt1(&SLOT_READ_ADDR, &current().values(), 5.0));
        assert_eq!(m.outcome(), None);
    }

    #[test]
    fn times_out_without_answers() {
        let mut m = LoadMachine::new(0x10, current(), target());
        m.next_action(0.0);
        assert_eq!(m.next_action(59_999.0), LoadAction::Wait(1.0));
        assert_eq!(
            m.next_action(60_000.0),
            LoadAction::Done(LoadOutcome::TimedOut)
        );

        // After the ack, polls get no answer either.
        let mut m = started();
        let mut now = 570.0;
        while let LoadAction::Poll(_) | LoadAction::Wait(_) = m.next_action(now) {
            now += 500.0;
        }
        assert_eq!(m.outcome(), Some(LoadOutcome::TimedOut));
        assert!(now >= 70.0 + TIMEOUT_MS);
    }
}
