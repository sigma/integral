//! The recall: pushing the project's state to the device (ADR 0009).
//!
//! # When
//!
//! After the host has restored the instance's state, never before, and once
//! for each device session and each restore. [`Recall::owes`] says whether a
//! push is due: the instance holds a state it trusts, it can be pushed, and
//! the session the link is in has not had its push for the state it holds.
//!
//! - **Trusted** is a state the host restored ([`Instance::was_restored`],
//!   never [`Instance::is_settled`], which a fresh instance is too), or the
//!   one the instance adopted from the device and has held since: the
//!   project, the host's state, always wins, so a device that comes back in a
//!   new session gets it. Only a brand-new instance's first session adopts.
//! - **Pushable** is a Studio Set plugin, or a Tone or Kit plugin that saved a
//!   tone. A restored Tone or Kit instance without one adopts from the start.
//! - A socket drop within a session does not change its ID, so it pushes
//!   nothing; a new session (a link drop, a power cycle, a service restart)
//!   does. Every restore the host makes counts as a new state
//!   ([`Instance::restores`]): a preset load or a revert in the middle of a
//!   session is pushed too.
//! - A tone the editor picked that reaches the device once a push is owed
//!   ([`Recall::owes_only_the_session`]) stands if the push is owed only for
//!   a new session: it is the user's latest word, so it is adopted and the
//!   session gets no push. If a restore made the push owed, the restore wins.
//!   One that reaches the Part during the push makes it start over, its
//!   selection first, so that its values do not land on the pick's tone.
//!
//! The follower starts the push after the instance's first read of the
//! session has settled it, and reads in the meantime keep the project's
//! values ([`Follower`](super::follow::Follower)). The push is queued with
//! `Push`, which the service never filters, and it carries the values the
//! host holds when it is queued.
//!
//! # What
//!
//! - **The Studio Set plugin** pushes every parameter it has, in full: Common,
//!   Chorus, Reverb, Master EQ, Motional Surround, each Part's mixer fields,
//!   Part EQ and Drum COMP+EQ, as DT1s over runs of adjacent fields. The
//!   Part tone-select bytes (`00 06` to `00 08` of a Part) are never in it,
//!   and never bridged: they belong to the Tone or Kit plugin bound to the
//!   Part, so the pushes of different instances do not depend on their
//!   order.
//! - **A Tone or Kit plugin** selects the saved tone first
//!   ([`Request::ChangeTone`]), then sends its fields:
//!   - a **User** tone in full, because the slot may hold another tone by now;
//!   - a **Preset or Expansion** tone as a difference from the service's
//!     baseline of it ([`Request::Baseline`], the tone as the device
//!     reported it right after the selection): one DT1 per run of fields that
//!     differ, or the full runs if they are fewer DT1s. An untouched preset
//!     is the selection alone. If the baseline names another tone, the
//!     selection is sent again; with no baseline (the device does not read
//!     the tone back) the full runs go;
//!   - a tone of a bank the device does not let one edit (GM2, ExPCM) is the
//!     selection alone.
//!
//!   A Kit plugin bound to a key does not select (the Part's kit belongs to the
//!   plugin bound to the whole kit) and sends the key's fields in full. The
//!   whole kit's selection reloads every key, so when another writer's tone
//!   selection reaches the Part after the key plugin has pushed, and the Part
//!   holds the kit the plugin saved, it pushes again
//!   ([`Recall::kit_reloaded`]) instead of adopting the reloaded kit.
//!
//! Only fields that act now are sent: a slot of an effect type other than the
//! one the host holds for the effect would overwrite another type's slot.
//!
//! ## Runs
//!
//! A DT1 can span bytes that no host parameter owns, if the plugin knows what
//! to write there: for a Preset or Expansion tone the bytes of the baseline
//! (the device already holds them), and for any push a byte proven on hardware
//! to ignore a write ([`UserMemorySpec::is_reserved`]: two bytes of the Studio
//! Set Common), written as 0. Anything else ends a run: the other `<*>` bytes
//! of the MIDI Implementation are not known to ignore writes (some take
//! them), and the plugin writes nothing it does not own. Always the Part
//! tone-select bytes end a run. After an effect type changes, the bytes of
//! effect slots are not bridged either, because the slots then mean
//! something else.
//!
//! ## Where the difference is made
//!
//! In the plugin, from the service's `Baseline` answer. The device queue
//! holds one item at a time, so nothing downstream can merge the survivors of
//! a field-by-field filter: they would each cost a DT1, more than the full
//! runs for a lightly edited preset. The runs can only be formed where the
//! whole set of differences is known, and the baseline is the data the
//! service already keeps for that. The plugin therefore sends the selection,
//! asks for the baseline (the service has read it back by then, in this
//! client's turns), and queues one push of runs.
//!
//! # Status
//!
//! The instance reports [`Notice::Recalling`] from the start of the push
//! until the service says it has handed the last of it to the device's queue
//! ([`Message::Pushed`]). While it is under way no read applies the device's
//! values (they are the preset's, or the old project's), and writes of host
//! changes wait ([`Instance::disarm`]): they are kept and written when the
//! push is out. A push the service refuses is tried again, a few times.
//!
//! No part of this runs on the audio thread.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use integral_core::address::Address;
use integral_core::device_spec::{INTEGRA7, USER_MEMORY, UserMemorySpec};
use integral_core::host_params::{Binding, PluginKind};
use integral_core::params::{
    self, TONE_NAME_SIZE, TONE_SELECTION_SIZE, is_editable_bank, part, tone_type_from_bank_msb,
};
use integral_core::sysex;
use integral_core::tone_banks::is_fixed_tone;
use integral_device_service::SessionId;
use integral_device_service::protocol::{BaselineRun, Message, Request, ToneSelect};

use super::bank::ParamBank;
use super::follow::Wire;
use super::instance::Instance;
use super::notice::Notice;
use super::tone::{SavedTone, is_name_byte};

/// How long the recall waits for the service's answer to `Baseline` before it
/// pushes the full runs. The service reads a tone back a block at a time, at
/// 40 ms a message, so a cold answer takes about two seconds.
pub const BASELINE_WAIT: Duration = Duration::from_secs(20);

/// How long after a push is queued the recall waits for the service to say
/// it has handed it over (`Pushed`) before it gives up on the session.
/// Sixteen instances pushing whole tones take about twenty seconds between
/// them.
pub const PUSH_WAIT: Duration = Duration::from_secs(60);

/// How many times a refused push is tried again before the session is given
/// up on.
const MAX_FAILURES: u32 = 3;

/// How many times the selection is sent again when the baseline turns out to
/// be of another tone.
const MAX_RESELECTS: u32 = 2;

/// What a recall asks of the world around it.
pub struct Context<'a> {
    /// The instance being recalled.
    pub instance: &'a Instance,
    /// Where it is bound.
    pub binding: Binding,
    /// The device ID the requests carry; the service replaces it.
    pub device_id: u8,
    /// The connection.
    pub wire: &'a dyn Wire,
}

/// One DT1's worth of consecutive bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// Where the first byte goes.
    pub address: Address,
    /// The bytes, at most [`sysex::MAX_DT1_DATA`].
    pub data: Vec<u8>,
}

/// A host parameter's bytes at their address.
struct Field {
    /// The parameter's position in the bank.
    index: usize,
    start: u32,
    bytes: Vec<u8>,
}

/// The fields a recall of the bank sends, in address order: those that live
/// where the Binding points and act now, except the Part tone-select bytes,
/// which a Tone or Kit plugin alone owns.
fn fields(bank: &ParamBank, binding: &Binding) -> Vec<Field> {
    let mut out: Vec<Field> = (0..bank.len())
        .filter(|i| bank.placed(*i, binding) && bank.host_active(*i))
        .filter_map(|index| {
            let host = &bank.list()[index].0;
            let start = host.device_address(binding).ok()?.value();
            (!touches_a_select(start, host.width as u32)).then(|| Field {
                index,
                start,
                bytes: host.encode(bank.value(index)),
            })
        })
        .collect();
    out.sort_by_key(|f| f.start);
    out
}

/// Whether `len` bytes from `start` overlap a Part's tone-select bytes.
fn touches_a_select(start: u32, len: u32) -> bool {
    (0..INTEGRA7.part_count).any(|p| {
        let select = params::part_address(p, part::TONE_BANK_MSB).value();
        start < select + TONE_SELECTION_SIZE.value() && select < start + len
    })
}

/// The bytes of a baseline, by linear address.
fn baseline_bytes(baseline: &[BaselineRun]) -> HashMap<u32, u8> {
    let mut known = HashMap::new();
    for run in baseline {
        let start = Address::new(
            run.address[0],
            run.address[1],
            run.address[2],
            run.address[3],
        )
        .value();
        for (k, byte) in run.bytes.iter().enumerate() {
            known.insert(start + k as u32, *byte);
        }
    }
    known
}

/// What a DT1 may carry at an address no field of the push owns: the byte the
/// device already holds (the baseline's) or one it ignores (reserved, 0).
struct Filler<'a> {
    known: Option<HashMap<u32, u8>>,
    spec: Option<&'a UserMemorySpec>,
    part: u8,
    /// Addresses not to bridge.
    forbidden: HashSet<u32>,
}

impl<'a> Filler<'a> {
    /// The filler for the bank's plugin bound to `binding`, over `baseline`
    /// if the device has been read.
    fn new(bank: &ParamBank, binding: &Binding, baseline: Option<&[BaselineRun]>) -> Self {
        let known = baseline.map(baseline_bytes);
        let mut forbidden = HashSet::new();
        // Once an effect's type changes, what the baseline holds in its slots
        // is another type's.
        if let Some(known) = &known
            && type_changed(bank, binding, known)
        {
            for i in (0..bank.len()).filter(|i| bank.list()[*i].0.effect.is_some()) {
                if let Ok(at) = bank.list()[i].0.device_address(binding) {
                    let width = bank.list()[i].0.width as u32;
                    forbidden.extend(at.value()..at.value() + width);
                }
            }
        }
        Self {
            known,
            spec: USER_MEMORY
                .iter()
                .find(|s| s.tone_type == bank.kind().tone_type()),
            part: binding.part.unwrap_or(0),
            forbidden,
        }
    }

    fn byte(&self, at: u32) -> Option<u8> {
        if self.forbidden.contains(&at) || touches_a_select(at, 1) {
            return None;
        }
        self.known
            .as_ref()
            .and_then(|known| known.get(&at).copied())
            .or_else(|| {
                self.spec
                    .is_some_and(|s| s.is_reserved(self.part, at))
                    .then_some(0)
            })
    }
}

/// Whether an effect type the host holds is not the baseline's.
fn type_changed(bank: &ParamBank, binding: &Binding, known: &HashMap<u32, u8>) -> bool {
    fields(bank, binding)
        .iter()
        .any(|f| bank.is_holder(f.index) && differs(f, known))
}

/// Whether the baseline lacks the field's bytes or holds others.
fn differs(field: &Field, known: &HashMap<u32, u8>) -> bool {
    field
        .bytes
        .iter()
        .enumerate()
        .any(|(k, byte)| known.get(&(field.start + k as u32)) != Some(byte))
}

/// DT1s over the `fields` (sorted), joining two when the bytes between them
/// can be filled, and never splitting a field.
fn runs_of(fields: impl IntoIterator<Item = Field>, filler: &Filler<'_>) -> Vec<Run> {
    let mut runs: Vec<(u32, Vec<u8>)> = Vec::new();
    for field in fields {
        if let Some((start, data)) = runs.last_mut() {
            let end = *start + data.len() as u32;
            if field.start >= end {
                let gap = field.start - end;
                let fill: Option<Vec<u8>> = (0..gap).map(|k| filler.byte(end + k)).collect();
                if let Some(fill) = fill
                    && data.len() + fill.len() + field.bytes.len() <= sysex::MAX_DT1_DATA
                {
                    data.extend(fill);
                    data.extend(field.bytes);
                    continue;
                }
            }
        }
        runs.push((field.start, field.bytes));
    }
    runs.into_iter()
        .map(|(start, data)| Run {
            address: Address::from_value(start).expect("a field has an address"),
            data,
        })
        .collect()
}

/// Everything the recall of an instance bound to `binding` sends, as DT1s
/// over runs of adjacent fields. With a `baseline` the bytes between fields
/// are filled from it where it has them.
pub fn full_runs(
    bank: &ParamBank,
    binding: &Binding,
    baseline: Option<&[BaselineRun]>,
) -> Vec<Run> {
    runs_of(fields(bank, binding), &Filler::new(bank, binding, baseline))
}

/// The fields of the recall that differ from `baseline`, as DT1s over runs
/// that are filled from it. A field the baseline does not wholly cover
/// differs, and so does every slot of an effect whose type differs.
pub fn diff_runs(bank: &ParamBank, binding: &Binding, baseline: &[BaselineRun]) -> Vec<Run> {
    let known = baseline_bytes(baseline);
    let all = fields(bank, binding);
    let changed: HashSet<usize> = all
        .iter()
        .filter(|f| bank.is_holder(f.index) && differs(f, &known))
        .map(|f| f.index)
        .collect();
    let kept = all.into_iter().filter(|f| {
        differs(f, &known)
            || bank
                .type_slot(f.index)
                .is_some_and(|holder| changed.contains(&holder))
    });
    runs_of(kept, &Filler::new(bank, binding, Some(baseline)))
}

/// `diff`, unless `full` takes fewer DT1s.
pub fn cheaper(full: Vec<Run>, diff: Vec<Run>) -> Vec<Run> {
    if full.len() < diff.len() { full } else { diff }
}

/// The address and length of the name of the tone in the Part `binding`
/// names, for a plugin of `kind`.
pub fn name_block(kind: PluginKind, binding: &Binding) -> Option<(u32, u32)> {
    Some((
        params::tone_name_address(binding.part?, kind.tone_type()?).value(),
        TONE_NAME_SIZE.value(),
    ))
}

/// The tone the device holds in the Part `binding` names, from the bytes a
/// read got (`byte` gives the byte at a linear address, if it arrived), when
/// it is a tone of the plugin's type.
fn read_tone(
    instance: &Instance,
    binding: &Binding,
    byte: &dyn Fn(u32) -> Option<u8>,
) -> Option<(u8, u8, u8)> {
    let kind = instance.kind();
    kind.tone_type()?;
    let (select, _) = params::tone_select_block(binding.part?);
    let select = select.value();
    let (msb, lsb, pc) = (byte(select)?, byte(select + 1)?, byte(select + 2)?);
    (tone_type_from_bank_msb(msb) == kind.tone_type() && lsb < 0x80 && pc < 0x80)
        .then_some((msb, lsb, pc))
}

/// Save the tone the device holds in the Part `binding` names, from the bytes
/// a read got, when it is a tone of the plugin's type. Else the saved tone
/// stays: a Part that holds another type is no reason to forget the
/// instance's own.
pub fn capture(instance: &Instance, binding: &Binding, byte: &dyn Fn(u32) -> Option<u8>) {
    let name: Option<String> = name_block(instance.kind(), binding).and_then(|(at, len)| {
        (0..len)
            .map(|k| byte(at + k).map(|b| if is_name_byte(b) { char::from(b) } else { '?' }))
            .collect()
    });
    if let Some(tone) = read_tone(instance, binding, byte) {
        save_tone(instance, tone, name.as_deref());
    } else if let Some(name) = name
        && !select_was_read(instance, binding, byte)
        && let Some(saved) = instance.tone.get()
    {
        // The probe and the service tell the tone; this read brings its name.
        instance
            .tone
            .set(Some(SavedTone::new(saved.msb, saved.lsb, saved.pc, &name)));
    }
}

/// Whether the read has the Part's tone-select bytes at all.
fn select_was_read(
    instance: &Instance,
    binding: &Binding,
    byte: &dyn Fn(u32) -> Option<u8>,
) -> bool {
    let Some(part) = binding
        .part
        .filter(|_| instance.kind().tone_type().is_some())
    else {
        return false;
    };
    byte(params::tone_select_block(part).0.value()).is_some()
}

/// Save `tone` (Bank Select MSB, LSB and Program Change) as the instance's,
/// if it is of the plugin's type, with `name` if the device was read for it,
/// else the one saved for the same tone.
pub fn save_tone(instance: &Instance, tone: (u8, u8, u8), name: Option<&str>) {
    let (msb, lsb, pc) = tone;
    if tone_type_from_bank_msb(msb) != instance.kind().tone_type() || lsb >= 0x80 || pc >= 0x80 {
        return;
    }
    let name = name.map(str::to_owned).unwrap_or_else(|| {
        // Not read: keep the name while the tone is the same.
        instance
            .tone
            .get()
            .filter(|t| (t.msb, t.lsb, t.pc) == (msb, lsb, pc))
            .map(|t| t.name)
            .unwrap_or_default()
    });
    instance.tone.set(Some(SavedTone::new(msb, lsb, pc, &name)));
}

/// The state a push is for: a device session, and which restore of the
/// instance's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stamp {
    session: SessionId,
    restores: u64,
}

impl Stamp {
    fn of(instance: &Instance, session: Option<SessionId>) -> Option<Self> {
        session.map(|session| Self {
            session,
            restores: instance.restores(),
        })
    }
}

/// What the recall is waiting for.
#[derive(Debug)]
enum Phase {
    /// Nothing is under way.
    Idle,
    /// The tone is selected and the baseline asked for.
    Baseline { tone: SavedTone, until: Instant },
    /// All of the push is queued; waiting for the service to hand it over.
    Finishing { until: Instant },
}

/// The recall of one instance: which state has had its push, and the push
/// under way.
#[derive(Debug)]
pub struct Recall {
    /// The state whose push is done (or given up).
    pushed: Option<Stamp>,
    /// The state the push under way is for.
    running: Option<Stamp>,
    /// The instance holds the state it adopted from the device.
    adopted: bool,
    /// The restore whose state was abandoned: the user bound the instance to
    /// another target, which that state was not saved for.
    abandoned: Option<u64>,
    phase: Phase,
    /// Pushes handed to the service that it has not answered `Pushed` for.
    outstanding: u32,
    /// `Pushed` answers still to come for pushes that were abandoned after
    /// events were lost: they are not the answers to the next push.
    stale_pushed: u32,
    /// Pushes the service refused in a row.
    failures: u32,
    /// Times the selection was sent again for the baseline of another tone.
    reselects: u32,
    /// The Part's tone must be the saved one for the push under way or due
    /// (a key's plugin pushing again after a tone change).
    check_select: bool,
    /// A tone change reached the Part while a key's plugin was pushing.
    again: bool,
}

impl Default for Recall {
    fn default() -> Self {
        Self {
            pushed: None,
            running: None,
            adopted: false,
            abandoned: None,
            phase: Phase::Idle,
            outstanding: 0,
            stale_pushed: 0,
            failures: 0,
            reselects: 0,
            check_select: false,
            again: false,
        }
    }
}

impl Recall {
    /// Whether `instance` has a push due in `session`: see the module docs.
    pub fn owes(&self, instance: &Instance, session: Option<SessionId>) -> bool {
        Stamp::of(instance, session)
            .is_some_and(|stamp| self.holds_state(instance) && self.pushed != Some(stamp))
    }

    /// Whether the push owed in `session` is owed only because the session is
    /// new: no state the host restored is still to be pushed (the last
    /// restore's state has had a push in an earlier session, or the instance
    /// holds a state it adopted and was never restored).
    pub fn owes_only_the_session(&self, instance: &Instance, session: Option<SessionId>) -> bool {
        let restore_owed = instance.was_restored()
            && self.pushed.map(|pushed| pushed.restores) != Some(instance.restores());
        self.owes(instance, session) && !restore_owed
    }

    /// Whether the instance holds a state it trusts and can push.
    fn holds_state(&self, instance: &Instance) -> bool {
        let pushable = instance.kind() == PluginKind::StudioSet || instance.tone.get().is_some();
        let restored = instance.was_restored() && self.abandoned != Some(instance.restores());
        pushable && (restored || self.adopted)
    }

    /// The instance has taken what the device holds as its state in
    /// `session`: it has nothing to push there, and holds a state to push in
    /// the next.
    pub fn adopted(&mut self, instance: &Instance, session: Option<SessionId>) {
        self.adopted = true;
        if !self.active() {
            self.pushed = Stamp::of(instance, session);
        }
    }

    /// Whether a push is under way.
    pub fn active(&self) -> bool {
        !matches!(self.phase, Phase::Idle)
    }

    /// Whether the next read must tell that the Part still holds the saved
    /// tone ([`select_matches`](Self::select_matches)).
    pub fn checking(&self) -> bool {
        self.check_select
    }

    /// How long until the recall next has something to do on its own.
    pub fn next_wake(&self, now: Instant) -> Option<Duration> {
        match self.phase {
            Phase::Idle => None,
            Phase::Baseline { until, .. } | Phase::Finishing { until } => {
                Some(until.saturating_duration_since(now))
            }
        }
    }

    /// Another writer's tone change reached the Part of a key's plugin. The
    /// whole kit's selection reloads every key, wiping what the plugin
    /// pushed, so if it has pushed in this state (or is pushing), it is owed
    /// a push again, provided the Part still holds the kit it saved (the next
    /// read checks). Returns whether it is.
    pub fn kit_reloaded(
        &mut self,
        instance: &Instance,
        binding: &Binding,
        session: Option<SessionId>,
    ) -> bool {
        let Some(stamp) = Stamp::of(instance, session) else {
            return false;
        };
        if binding.key.is_none() || !self.holds_state(instance) {
            return false;
        }
        if self.active() {
            self.again = true;
            return true;
        }
        if self.pushed != Some(stamp) {
            // Owed already, or adopting: nothing was wiped.
            return false;
        }
        self.pushed = None;
        self.check_select = true;
        true
    }

    /// The read for a check came back: `byte` gives what it read. Returns
    /// whether the Part holds the saved tone. If not, the state gives up its
    /// push (the instance adopts what the device holds).
    pub fn select_matches(
        &mut self,
        instance: &Instance,
        binding: &Binding,
        session: Option<SessionId>,
        byte: &dyn Fn(u32) -> Option<u8>,
    ) -> bool {
        self.check_select = false;
        let saved = instance.tone.get().map(|t| (t.msb, t.lsb, t.pc));
        let matches = read_tone(instance, binding, byte).is_some_and(|read| Some(read) == saved);
        if !matches {
            self.pushed = Stamp::of(instance, session);
        }
        matches
    }

    /// Start the push for `session`, if it is due and none is under way.
    /// Returns whether it started.
    pub fn begin(&mut self, cx: &Context<'_>, session: Option<SessionId>, now: Instant) -> bool {
        if self.active()
            || !self.owes(cx.instance, session)
            || !cx.instance.kind().accepts(&cx.binding)
        {
            return false;
        }
        self.running = Stamp::of(cx.instance, session);
        self.outstanding = 0;
        self.reselects = 0;
        self.again = false;
        self.check_select = false;
        cx.instance.notices.set(Notice::Recalling, true);
        // Host edits wait until the push is out, then go after it.
        cx.instance.disarm();
        // Nothing but the fields goes with a key's plugin.
        let tone =
            cx.instance.tone.get().filter(|_| {
                cx.instance.kind() != PluginKind::StudioSet && cx.binding.key.is_none()
            });
        let Some(tone) = tone else {
            let runs = full_runs(&cx.instance.bank, &cx.binding, None);
            self.push(cx, Vec::new(), runs, now);
            return true;
        };
        let select = select_request(&cx.binding, &tone);
        // The instance's own selection: the Part holds the plugin's type from
        // here on, and the writes after it are ordered behind it.
        cx.instance.selected(tone.msb);
        if is_fixed_tone(tone.msb, tone.lsb) {
            // Until the type slots are pushed no effect slot is written.
            cx.instance.bank.forget_types();
            self.select_and_ask(cx, select, tone, now);
        } else if is_editable_bank(tone.msb) {
            let runs = full_runs(&cx.instance.bank, &cx.binding, None);
            self.push(cx, vec![select], runs, now);
        } else {
            self.push(cx, vec![select], Vec::new(), now);
        }
        true
    }

    /// Take a message of the service: `Pushed`, or the baseline asked for.
    pub fn on_message(&mut self, cx: &Context<'_>, message: &Message, now: Instant) {
        match message {
            Message::Pushed => {
                if self.stale_pushed > 0 {
                    self.stale_pushed -= 1;
                    return;
                }
                self.outstanding = self.outstanding.saturating_sub(1);
                self.complete_if_out(cx.instance);
            }
            Message::Baseline {
                binding,
                tone,
                runs,
            } if binding.part == cx.binding.part => self.on_baseline(cx, *tone, runs, now),
            _ => {}
        }
    }

    /// The service refused a request of the push (the push is not queued, or
    /// the baseline could not be asked for): clear the status and try again,
    /// a few times, then give up on the state.
    pub fn refused(&mut self, instance: &Instance) {
        if !self.active() {
            return;
        }
        self.failures += 1;
        let give_up = self.failures >= MAX_FAILURES;
        let stamp = self.running;
        self.abort(instance);
        if give_up {
            self.pushed = stamp;
            self.failures = 0;
            instance.arm();
        }
    }

    /// Act on the time `now`: a baseline that did not come, a push that was
    /// never reported out.
    pub fn tick(&mut self, cx: &Context<'_>, now: Instant) {
        match &self.phase {
            Phase::Baseline { until, .. } if *until <= now => {
                let runs = full_runs(&cx.instance.bank, &cx.binding, None);
                self.push(cx, Vec::new(), runs, now);
            }
            Phase::Finishing { until } if *until <= now => self.complete(cx.instance),
            _ => {}
        }
    }

    /// Stop the push under way (the link or the connection is gone): its state
    /// is not marked, so it is pushed again when the instance is back, in the
    /// same session or in a new one.
    pub fn abort(&mut self, instance: &Instance) {
        if self.active() {
            instance.notices.set(Notice::Recalling, false);
        }
        self.phase = Phase::Idle;
        self.running = None;
        self.outstanding = 0;
        self.stale_pushed = 0;
        self.again = false;
    }

    /// Stop the push under way because events were lost, `Pushed` or the
    /// baseline perhaps among them. The service still answers the pushes it
    /// has, and those answers are not the next push's.
    pub fn abort_lost(&mut self, instance: &Instance) {
        let owed = self.outstanding + self.stale_pushed;
        self.abort(instance);
        self.stale_pushed = owed;
    }

    /// Stop the push under way for good: the user bound the instance to
    /// another target, which the project's state was not saved for. The
    /// state is marked pushed in `session` whether or not a push was under
    /// way.
    pub fn abandon(&mut self, instance: &Instance, session: Option<SessionId>) {
        self.abort(instance);
        self.abandoned = Some(instance.restores());
        // What it held is not the new target's: it adopts that.
        self.adopted = false;
        self.pushed = Stamp::of(instance, session);
    }

    fn on_baseline(
        &mut self,
        cx: &Context<'_>,
        answer: Option<ToneSelect>,
        baseline: &[BaselineRun],
        now: Instant,
    ) {
        let Phase::Baseline { tone, .. } = &self.phase else {
            return;
        };
        let tone = tone.clone();
        let bank = &cx.instance.bank;
        match answer {
            Some(a) if (a.msb, a.lsb, a.pc) == (tone.msb, tone.lsb, tone.pc) => {
                let full = full_runs(bank, &cx.binding, Some(baseline));
                let runs = cheaper(full, diff_runs(bank, &cx.binding, baseline));
                self.push(cx, Vec::new(), runs, now);
            }
            Some(_) if self.reselects < MAX_RESELECTS => {
                // Another tone is on the Part (someone selected over ours):
                // ours is not there to write to.
                self.reselects += 1;
                let select = select_request(&cx.binding, &tone);
                cx.instance.selected(tone.msb);
                self.select_and_ask(cx, select, tone, now);
            }
            _ => {
                let runs = full_runs(bank, &cx.binding, None);
                self.push(cx, Vec::new(), runs, now);
            }
        }
    }

    /// Select the tone and ask for its baseline.
    fn select_and_ask(&mut self, cx: &Context<'_>, select: Request, tone: SavedTone, now: Instant) {
        self.send(cx, vec![select]);
        cx.wire.baseline(
            cx.binding
                .part
                .expect("a Tone or Kit plugin is bound to a Part"),
        );
        self.phase = Phase::Baseline {
            tone,
            until: now + BASELINE_WAIT,
        };
    }

    /// Queue `first` and then `runs` as one push, and wait for it to be out.
    fn push(&mut self, cx: &Context<'_>, first: Vec<Request>, runs: Vec<Run>, now: Instant) {
        // The effect types the project holds are the device's from here on.
        let bank = &cx.instance.bank;
        for i in (0..bank.len()).filter(|i| bank.is_holder(*i) && bank.placed(*i, &cx.binding)) {
            bank.set_device_type(i, bank.value(i));
        }
        let mut requests = first;
        requests.extend(runs.iter().map(|run| Request::Raw {
            bytes: sysex::build_dt1(cx.device_id, &run.address, &run.data),
        }));
        if !requests.is_empty() {
            self.send(cx, requests);
        }
        self.phase = Phase::Finishing {
            until: now + PUSH_WAIT,
        };
        self.complete_if_out(cx.instance);
    }

    fn send(&mut self, cx: &Context<'_>, requests: Vec<Request>) {
        cx.wire.push(requests);
        self.outstanding += 1;
    }

    fn complete_if_out(&mut self, instance: &Instance) {
        if matches!(self.phase, Phase::Finishing { .. }) && self.outstanding == 0 {
            self.complete(instance);
        }
    }

    /// The push is out, or given up: this state has had its push.
    fn complete(&mut self, instance: &Instance) {
        let again = self.again;
        self.pushed = self.running;
        self.phase = Phase::Idle;
        self.running = None;
        self.outstanding = 0;
        self.failures = 0;
        self.again = false;
        instance.notices.set(Notice::Recalling, false);
        instance.arm();
        if again && self.pushed.is_some() {
            // A kit was reloaded while the key's fields went out.
            self.pushed = None;
            self.check_select = true;
        }
    }
}

/// The request that selects `tone` on the Part `binding` names.
fn select_request(binding: &Binding, tone: &SavedTone) -> Request {
    Request::ChangeTone {
        part: binding
            .part
            .expect("a Tone or Kit plugin is bound to a Part"),
        msb: tone.msb,
        lsb: tone.lsb,
        pc: tone.pc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An instance of `kind` bound to `binding`, whose host holds the
    /// defaults.
    fn instance(kind: PluginKind, binding: Binding) -> Instance {
        let instance = Instance::new(kind);
        instance.binding.set(Some(binding));
        instance
    }

    fn binding_of(kind: PluginKind) -> Binding {
        match kind {
            PluginKind::StudioSet => Binding::ANY,
            _ => Binding::of_part(3),
        }
    }

    /// The baseline a device would report for an instance that holds its
    /// defaults: each field's bytes, as the service reports them (one run
    /// per field).
    fn baseline_of(bank: &ParamBank, binding: &Binding) -> Vec<BaselineRun> {
        fields(bank, binding)
            .into_iter()
            .map(|f| BaselineRun {
                address: *Address::from_value(f.start).unwrap().as_bytes(),
                bytes: f.bytes,
            })
            .collect()
    }

    fn start_of(run: &BaselineRun) -> u32 {
        Address::new(
            run.address[0],
            run.address[1],
            run.address[2],
            run.address[3],
        )
        .value()
    }

    #[test]
    fn runs_never_split_a_field_or_pass_a_dt1_and_keep_every_byte_once() {
        for kind in PluginKind::ALL {
            let binding = binding_of(kind);
            let bank = instance(kind, binding).bank;
            let runs = full_runs(&bank, &binding, None);
            let wanted: usize = fields(&bank, &binding).iter().map(|f| f.bytes.len()).sum();
            let sent: usize = runs.iter().map(|r| r.data.len()).sum();
            // Reserved bytes between fields are written too.
            assert!(sent >= wanted, "{kind:?}");
            assert!(runs.iter().all(|r| r.data.len() <= sysex::MAX_DT1_DATA));
            assert!(
                runs.len() * 2 < bank.len(),
                "{kind:?}: {} DT1s for {} parameters",
                runs.len(),
                bank.len()
            );
            for pair in runs.windows(2) {
                let end = pair[0].address.value() + pair[0].data.len() as u32;
                assert!(end <= pair[1].address.value(), "{kind:?}: in order");
            }
            // Every field is where it belongs in a run.
            for f in fields(&bank, &binding) {
                let run = runs
                    .iter()
                    .find(|r| {
                        r.address.value() <= f.start
                            && f.start + f.bytes.len() as u32
                                <= r.address.value() + r.data.len() as u32
                    })
                    .expect("in a run");
                let at = (f.start - run.address.value()) as usize;
                assert_eq!(run.data[at..at + f.bytes.len()], f.bytes[..], "{kind:?}");
            }
        }
    }

    #[test]
    fn the_studio_set_fills_only_proven_bytes_and_never_a_parts_tone_select() {
        let bank = instance(PluginKind::StudioSet, Binding::ANY).bank;
        let runs = full_runs(&bank, &Binding::ANY, None);
        assert!(!runs.is_empty());
        let fields = fields(&bank, &Binding::ANY);
        let studio = USER_MEMORY.iter().find(|s| s.tone_type.is_none()).unwrap();
        for run in &runs {
            let (start, end) = (
                run.address.value(),
                run.address.value() + run.data.len() as u32,
            );
            for at in start..end {
                let owned = fields
                    .iter()
                    .any(|f| f.start <= at && at < f.start + f.bytes.len() as u32);
                assert!(owned || studio.is_reserved(0, at), "{at:#x} is nobody's");
                if !owned {
                    assert_eq!(run.data[(at - start) as usize], 0, "reserved goes as 0");
                }
            }
        }
        for p in 0..INTEGRA7.part_count {
            let select = params::part_address(p, part::TONE_BANK_MSB).value();
            for run in &runs {
                let (start, end) = (
                    run.address.value(),
                    run.address.value() + run.data.len() as u32,
                );
                assert!(
                    end <= select || select + TONE_SELECTION_SIZE.value() <= start,
                    "part {p}: a run {start:#x}..{end:#x} holds the select at {select:#x}"
                );
            }
        }
        // Filling never makes more runs than the fields' own.
        let unfilled = runs_of(
            fields_again(&bank),
            &Filler {
                known: None,
                spec: None,
                part: 0,
                forbidden: HashSet::new(),
            },
        );
        assert!(runs.len() <= unfilled.len());

        // Of the Common's `<*>` bytes only the proven ones are filled.
        let filler = Filler::new(&bank, &Binding::ANY, None);
        let common = |offset: u8| {
            integral_core::address::STUDIO_SET
                .offset([0, 0, 0, offset])
                .value()
        };
        assert_eq!(filler.byte(common(0x17)), Some(0));
        assert_eq!(filler.byte(common(0x38)), Some(0));
        for untrusted in [0x10, 0x14, 0x4B] {
            assert_eq!(filler.byte(common(untrusted)), None, "{untrusted:#x}");
        }
    }

    fn fields_again(bank: &ParamBank) -> Vec<Field> {
        fields(bank, &Binding::ANY)
    }

    #[test]
    fn an_untouched_preset_has_no_difference_and_a_changed_field_is_one_run() {
        let binding = Binding::of_part(0);
        let instance = instance(PluginKind::PcmSynth, binding);
        let bank = &instance.bank;
        let baseline = baseline_of(bank, &binding);
        assert!(diff_runs(bank, &binding, &baseline).is_empty());

        // Two adjacent fields.
        let list = fields(bank, &binding);
        let adjacent = list
            .windows(2)
            .position(|w| w[0].start + w[0].bytes.len() as u32 == w[1].start)
            .expect("neighbours");
        let mut changed = baseline.clone();
        for run in &mut changed {
            let start = start_of(run);
            if start == list[adjacent].start || start == list[adjacent + 1].start {
                run.bytes[0] ^= 1;
            }
        }
        let diff = diff_runs(bank, &binding, &changed);
        assert_eq!(diff.len(), 1, "adjacent fields are one DT1");
        assert_eq!(diff[0].address.value(), list[adjacent].start);
    }

    #[test]
    fn a_field_the_baseline_does_not_cover_is_sent() {
        let binding = Binding::of_part(0);
        let instance = instance(PluginKind::PcmSynth, binding);
        let bank = &instance.bank;
        let mut baseline = baseline_of(bank, &binding);
        let dropped = baseline.remove(5);
        let diff = diff_runs(bank, &binding, &baseline);
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].address.value(), start_of(&dropped));
    }

    #[test]
    fn two_changes_a_gap_apart_are_one_dt1_over_the_baselines_bytes() {
        let binding = Binding::of_part(0);
        let instance = instance(PluginKind::PcmSynth, binding);
        let bank = &instance.bank;
        // A baseline that covers everything between the fields, as a read of
        // whole blocks does: 0x55 where no field is.
        let list = fields(bank, &binding);
        let mut covered = baseline_of(bank, &binding);
        for w in list.windows(2) {
            let end = w[0].start + w[0].bytes.len() as u32;
            if w[1].start > end && w[1].start - end <= 8 {
                covered.push(BaselineRun {
                    address: *Address::from_value(end).unwrap().as_bytes(),
                    bytes: vec![0x55; (w[1].start - end) as usize],
                });
            }
        }
        // Change the first field and one a few fields on, with a gap between.
        let gap_at = list
            .windows(2)
            .position(|w| {
                w[1].start > w[0].start + w[0].bytes.len() as u32
                    && w[1].start - (w[0].start + w[0].bytes.len() as u32) <= 8
            })
            .expect("a gap");
        let (a, b) = (&list[gap_at], &list[gap_at + 1]);
        for run in &mut covered {
            if start_of(run) == a.start || start_of(run) == b.start {
                run.bytes[0] ^= 1;
            }
        }
        let diff = diff_runs(bank, &binding, &covered);
        assert_eq!(diff.len(), 1, "joined: {diff:?}");
        let data = &diff[0].data;
        let gap = (b.start - (a.start + a.bytes.len() as u32)) as usize;
        let between = &data[a.bytes.len()..a.bytes.len() + gap];
        assert!(between.iter().all(|x| *x == 0x55), "the baseline's bytes");
    }

    #[test]
    fn after_an_effect_type_changes_all_its_slots_differ_and_none_is_bridged() {
        let binding = Binding::of_part(0);
        let instance = instance(PluginKind::PcmSynth, binding);
        let bank = &instance.bank;
        // The effect the host holds: one with slots, whose type is set so
        // that its slots are active.
        let slot = (0..bank.len())
            .find(|i| bank.list()[*i].0.effect.is_some() && bank.placed(*i, &binding))
            .expect("a slot");
        let holder = bank.type_slot(slot).unwrap();
        let effect = bank.list()[slot].0.effect.unwrap();
        bank.host_changed(holder, i32::from(effect.type_id));
        let baseline = baseline_of(bank, &binding);
        let slots_of_type = |bank: &ParamBank| {
            fields(bank, &binding)
                .iter()
                .filter(|f| bank.type_slot(f.index) == Some(holder))
                .map(|f| f.start)
                .collect::<Vec<_>>()
        };
        let slots = slots_of_type(bank);
        assert!(!slots.is_empty());
        assert!(
            diff_runs(bank, &binding, &baseline).is_empty(),
            "same type, same slots: nothing"
        );
        // The preset's effect is another type: its type byte differs and its
        // slots hold that type's values, which happen to equal ours.
        let mut other = baseline.clone();
        let holder_at = {
            let h = &bank.list()[holder].0;
            h.device_address(&binding).unwrap().value()
        };
        for run in &mut other {
            if start_of(run) == holder_at {
                run.bytes[0] ^= 1;
            }
        }
        let diff = diff_runs(bank, &binding, &other);
        let sent: HashSet<u32> = diff
            .iter()
            .flat_map(|r| r.address.value()..r.address.value() + r.data.len() as u32)
            .collect();
        for at in slots {
            assert!(sent.contains(&at), "slot {at:#x} is sent");
        }
    }

    #[test]
    fn the_full_runs_win_when_they_are_fewer_dt1s() {
        let run = |at: u32| Run {
            address: Address::from_value(at).unwrap(),
            data: vec![0],
        };
        let full = vec![run(0), run(10)];
        let few = vec![run(1)];
        let many = vec![run(1), run(3), run(5)];
        assert_eq!(cheaper(full.clone(), few.clone()), few);
        assert_eq!(cheaper(full.clone(), many.clone()), full);
        assert_eq!(cheaper(full.clone(), vec![]), vec![]);
        assert_eq!(cheaper(vec![run(0)], vec![run(1)]), vec![run(1)], "a tie");
    }

    #[test]
    fn an_effect_slot_of_another_type_than_the_hosts_is_not_sent() {
        let binding = Binding::of_part(0);
        let instance = instance(PluginKind::PcmSynth, binding);
        let bank = &instance.bank;
        let slot = (0..bank.len())
            .find(|i| bank.list()[*i].0.effect.is_some() && bank.placed(*i, &binding))
            .expect("an effect slot");
        let holder = bank.type_slot(slot).expect("its type");
        let effect = bank.list()[slot].0.effect.expect("an effect");
        let address = bank.list()[slot]
            .0
            .device_address(&binding)
            .unwrap()
            .value();
        let at = |bank: &ParamBank| {
            fields(bank, &binding)
                .iter()
                .filter(|f| f.start == address)
                .count()
        };

        bank.host_changed(holder, i32::from(effect.type_id));
        assert!(bank.host_active(slot));
        assert_eq!(at(bank), 1, "the slot of the host's type");
        bank.host_changed(holder, i32::from(effect.type_id) + 1);
        assert!(!bank.host_active(slot));
        assert!(at(bank) <= 1, "never two slots at one address");
    }

    #[test]
    fn the_name_is_read_from_the_tone_of_each_kind_and_kept_while_the_tone_is_the_same() {
        let kinds = [
            (PluginKind::PcmSynth, 87u8),
            (PluginKind::SnSynth, 95),
            (PluginKind::SnAcoustic, 89),
            (PluginKind::SnDrum, 88),
            (PluginKind::PcmDrum, 86),
        ];
        for (kind, msb) in kinds {
            let binding = Binding::of_part(1);
            let instance = instance(kind, binding);
            let select = params::tone_select_block(1).0.value();
            let (name_at, name_len) = name_block(kind, &binding).unwrap();
            assert_eq!(name_len, 12);
            let mut memory: HashMap<u32, u8> = HashMap::new();
            for (k, b) in [msb, 64, 9].iter().enumerate() {
                memory.insert(select + k as u32, *b);
            }
            for (k, b) in b"Soft Strings".iter().enumerate() {
                memory.insert(name_at + k as u32, *b);
            }
            capture(&instance, &binding, &|at| memory.get(&at).copied());
            assert_eq!(
                instance.tone.get(),
                Some(SavedTone::new(msb, 64, 9, "Soft Strings")),
                "{kind:?}"
            );
            // The name did not arrive: the same tone keeps it, another does
            // not.
            for k in 0..name_len {
                memory.remove(&(name_at + k));
            }
            capture(&instance, &binding, &|at| memory.get(&at).copied());
            assert_eq!(instance.tone.get().unwrap().name, "Soft Strings");
            memory.insert(select + 2, 10);
            capture(&instance, &binding, &|at| memory.get(&at).copied());
            assert_eq!(
                instance.tone.get(),
                Some(SavedTone::new(msb, 64, 10, "")),
                "another tone has no known name"
            );
            // A Part that holds another type leaves the saved tone as it was.
            memory.insert(select, 0);
            capture(&instance, &binding, &|at| memory.get(&at).copied());
            assert_eq!(instance.tone.get().unwrap().pc, 10);
        }
    }

    #[test]
    fn a_push_is_due_once_per_state_for_an_instance_that_trusts_one() {
        let studio = instance(PluginKind::StudioSet, Binding::ANY);
        let tone = instance(PluginKind::PcmSynth, Binding::of_part(0));
        let (one, two) = (Some(SessionId(1)), Some(SessionId(2)));
        let mut recall = Recall::default();
        assert!(!recall.owes(&studio, one), "a fresh instance adopts");
        studio.mark_restored();
        tone.mark_restored();
        assert!(recall.owes(&studio, one));
        assert!(!recall.owes(&studio, None), "no session, no push");
        assert!(!recall.owes(&tone, one), "no saved tone, nothing to select");
        tone.tone.set(Some(SavedTone::new(87, 64, 0, "")));
        assert!(recall.owes(&tone, two));
        // A restore in the middle of a session is a new state.
        recall.pushed = Stamp::of(&tone, two);
        assert!(!recall.owes(&tone, two));
        tone.mark_restored();
        assert!(recall.owes(&tone, two));
    }

    #[test]
    fn a_fresh_instance_adopts_in_its_first_session_and_pushes_in_the_next() {
        let tone = instance(PluginKind::PcmSynth, Binding::of_part(0));
        let (one, two) = (Some(SessionId(1)), Some(SessionId(2)));
        let mut recall = Recall::default();
        assert!(!recall.owes(&tone, one));
        // It adopts a tone from the device.
        tone.tone.set(Some(SavedTone::new(87, 64, 0, "")));
        assert!(!recall.owes(&tone, one), "not trusted before it adopts");
        recall.adopted(&tone, one);
        assert!(!recall.owes(&tone, one), "its own session is settled");
        assert!(recall.owes(&tone, two), "the project wins in a new one");
    }

    #[test]
    fn a_rebind_marks_the_session_so_the_old_project_is_not_pushed_to_the_new_target() {
        let tone = instance(PluginKind::PcmSynth, Binding::of_part(0));
        tone.tone.set(Some(SavedTone::new(87, 64, 0, "")));
        tone.mark_restored();
        let one = Some(SessionId(1));
        let mut recall = Recall::default();
        assert!(recall.owes(&tone, one));
        recall.abandon(&tone, one);
        assert!(!recall.owes(&tone, one));
        assert!(
            !recall.owes(&tone, Some(SessionId(2))),
            "not even in another session: the instance adopts the new target first"
        );
        recall.adopted(&tone, one);
        assert!(
            recall.owes(&tone, Some(SessionId(2))),
            "then it holds a state"
        );
    }
}
