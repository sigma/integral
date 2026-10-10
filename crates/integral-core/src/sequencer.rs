//! Step sequencer: the Pattern model and a clock-free event scheduler.
//!
//! The [`Sequencer`] never reads a clock. Callers pass the current time to
//! [`Sequencer::due`], which returns every MIDI event that falls inside the
//! look-ahead window exactly once. Note Offs are only emitted once their time
//! falls inside a window, so nothing is queued far ahead of time.

use std::collections::BTreeMap;

use crate::device::DeviceState;

mod smf;
pub use smf::SmfError;

/// Maximum number of Steps in a Track.
pub const MAX_STEPS: u8 = 64;

/// Default number of Steps in a new Track.
pub const DEFAULT_TRACK_LENGTH: u8 = 16;

/// Default tempo in BPM.
pub const DEFAULT_BPM: u16 = 120;

/// Steps per quarter note: a Step is a 16th note.
pub const STEPS_PER_QUARTER: u16 = 4;

/// MIDI channels addressed by [`Sequencer::panic`].
const MIDI_CHANNELS: u8 = 16;

/// All Notes Off controller number.
const CC_ALL_NOTES_OFF: u8 = 123;

/// One note starting on a Step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Note {
    /// MIDI note number (0–127).
    pub pitch: u8,
    /// Note On velocity (1–127).
    pub velocity: u8,
    /// Length in whole Steps (1–64). A length over 1 is a tie.
    pub length: u8,
}

impl Note {
    /// Create a note, clamping each field into its valid range.
    pub fn new(pitch: u8, velocity: u8, length: u8) -> Self {
        Self {
            pitch: pitch.min(127),
            velocity: velocity.clamp(1, 127),
            length: length.clamp(1, MAX_STEPS),
        }
    }
}

/// One row of the Pattern, played on its Part's receive channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    length: u8,
    /// Notes keyed by their start Step.
    steps: BTreeMap<u8, Vec<Note>>,
}

impl Default for Track {
    fn default() -> Self {
        Self {
            length: DEFAULT_TRACK_LENGTH,
            steps: BTreeMap::new(),
        }
    }
}

impl Track {
    /// Track length in Steps (1–64).
    pub fn length(&self) -> u8 {
        self.length
    }

    /// Set the Track length, clamped to 1–64. Notes past the end are kept
    /// but not played.
    pub fn set_length(&mut self, length: u8) {
        self.length = length.clamp(1, MAX_STEPS);
    }

    /// Notes starting on `step`.
    pub fn notes_at(&self, step: u8) -> &[Note] {
        self.steps.get(&step).map_or(&[], Vec::as_slice)
    }

    /// Replace the notes starting on `step` with `notes`. An empty chord
    /// clears the Step. Steps past [`MAX_STEPS`] are ignored.
    pub fn set_step(&mut self, step: u8, notes: Vec<Note>) {
        if step >= MAX_STEPS {
            return;
        }
        if notes.is_empty() {
            self.steps.remove(&step);
        } else {
            self.steps.insert(step, notes);
        }
    }

    /// Every non-empty Step with its notes, in Step order.
    pub fn steps(&self) -> impl Iterator<Item = (u8, &[Note])> {
        self.steps.iter().map(|(s, n)| (*s, n.as_slice()))
    }
}

/// All Tracks together, plus the tempo they play at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    bpm: u16,
    tracks: Vec<Track>,
}

impl Pattern {
    /// Create an empty Pattern with one Track per Part.
    ///
    /// Pass `DeviceSpec::part_count`, never a literal.
    pub fn new(part_count: u8) -> Self {
        Self {
            bpm: DEFAULT_BPM,
            tracks: vec![Track::default(); part_count as usize],
        }
    }

    /// Tempo in BPM.
    pub fn bpm(&self) -> u16 {
        self.bpm
    }

    /// The Tracks in Part order.
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// Mutable access to one Track, if it exists.
    pub fn track_mut(&mut self, index: usize) -> Option<&mut Track> {
        self.tracks.get_mut(index)
    }

    /// Duration of one Step in milliseconds.
    pub fn step_ms(&self) -> f64 {
        60_000.0 / f64::from(self.bpm) / f64::from(STEPS_PER_QUARTER)
    }

    /// Set the tempo directly. Use [`Sequencer::set_bpm`] while playing.
    pub(crate) fn set_bpm_raw(&mut self, bpm: u16) {
        self.bpm = bpm.max(1);
    }
}

/// A MIDI message to send at a given time.
#[derive(Debug, Clone, PartialEq)]
pub struct TimedMidi {
    /// Send time in milliseconds, on the caller's clock.
    pub t_ms: f64,
    /// Raw MIDI message bytes.
    pub bytes: Vec<u8>,
}

/// A note that has been switched on and not yet off.
#[derive(Debug, Clone, Copy)]
struct Sounding {
    channel: u8,
    pitch: u8,
    off_step: u64,
}

/// Maps global Step indices to times while playing.
#[derive(Debug, Clone, Copy)]
struct Clock {
    anchor_ms: f64,
    anchor_step: u64,
    /// First global Step not emitted yet.
    next_step: u64,
}

impl Clock {
    fn time(&self, step: u64, step_ms: f64) -> f64 {
        self.anchor_ms + (step - self.anchor_step) as f64 * step_ms
    }
}

/// Plays a [`Pattern`] by handing out timed MIDI events on request.
#[derive(Debug, Clone)]
pub struct Sequencer {
    pattern: Pattern,
    clock: Option<Clock>,
    sounding: Vec<Sounding>,
    last_t_ms: f64,
}

impl Sequencer {
    /// Create a stopped Sequencer for `pattern`.
    pub fn new(pattern: Pattern) -> Self {
        Self {
            pattern,
            clock: None,
            sounding: Vec::new(),
            last_t_ms: f64::NEG_INFINITY,
        }
    }

    /// The Pattern being played.
    pub fn pattern(&self) -> &Pattern {
        &self.pattern
    }

    /// Mutable access to the Pattern. Edits never change a sounding note.
    pub fn pattern_mut(&mut self) -> &mut Pattern {
        &mut self.pattern
    }

    /// Whether the transport is playing.
    pub fn is_playing(&self) -> bool {
        self.clock.is_some()
    }

    /// Start playing with Step 0 at `now_ms`. Does nothing if already playing.
    pub fn play(&mut self, now_ms: f64) {
        if self.clock.is_none() {
            self.clock = Some(Clock {
                anchor_ms: now_ms,
                anchor_step: 0,
                next_step: 0,
            });
        }
    }

    /// Change the tempo. While playing, Steps already emitted keep their
    /// times and the next Step starts the new tempo.
    pub fn set_bpm(&mut self, bpm: u16) {
        if let Some(clock) = &mut self.clock {
            clock.anchor_ms = clock.time(clock.next_step, self.pattern.step_ms());
            clock.anchor_step = clock.next_step;
        }
        self.pattern.set_bpm_raw(bpm);
    }

    /// Every event in `[now_ms, now_ms + horizon_ms]` not returned before,
    /// in time order. A Note Off sorts before a Note On on the same tick.
    ///
    /// Note Ons read Track N's channel from Part N's receive channel in
    /// `state`. Steps that fell entirely behind `now_ms` (a frozen tab) are
    /// skipped rather than sent late.
    pub fn due(&mut self, state: &DeviceState, now_ms: f64, horizon_ms: f64) -> Vec<TimedMidi> {
        let Some(clock) = &mut self.clock else {
            return Vec::new();
        };
        let step_ms = self.pattern.step_ms();
        let end = now_ms + horizon_ms;

        if clock.time(clock.next_step, step_ms) < now_ms {
            let behind = ((now_ms - clock.anchor_ms) / step_ms).ceil() as u64;
            clock.next_step = clock.anchor_step + behind;
        }

        // (step, is_on, bytes): sorting puts Offs before Ons on a shared step.
        let mut events: Vec<(u64, bool, Vec<u8>)> = Vec::new();
        while clock.time(clock.next_step, step_ms) <= end {
            let step = clock.next_step;
            for (i, track) in self.pattern.tracks.iter().enumerate() {
                let local = (step % u64::from(track.length)) as u8;
                let notes = track.notes_at(local);
                if notes.is_empty() {
                    continue;
                }
                let channel = track_channel(state, i);
                for note in notes {
                    let length = note.length.min(track.length - local);
                    events.push((step, true, vec![0x90 | channel, note.pitch, note.velocity]));
                    self.sounding.push(Sounding {
                        channel,
                        pitch: note.pitch,
                        off_step: step + u64::from(length),
                    });
                }
            }
            clock.next_step += 1;
        }

        self.sounding.retain(|s| {
            if clock.time(s.off_step, step_ms) <= end {
                events.push((s.off_step, false, note_off(s.channel, s.pitch)));
                false
            } else {
                true
            }
        });

        events.sort_by_key(|(step, is_on, _)| (*step, *is_on));
        let out: Vec<TimedMidi> = events
            .into_iter()
            .map(|(step, _, bytes)| TimedMidi {
                t_ms: clock.time(step, step_ms),
                bytes,
            })
            .collect();
        if let Some(last) = out.last() {
            self.last_t_ms = self.last_t_ms.max(last.t_ms);
        }
        out
    }

    /// Stop the transport and silence everything, in order:
    /// 1. Offs for every sounding note at `now_ms`.
    /// 2. CC 123 on each Track's channel at `now_ms`.
    /// 3. CC 123 on each Track's channel 1 ms after the last event already
    ///    emitted, so it lands after any Note On still queued.
    pub fn stop(&mut self, state: &DeviceState, now_ms: f64) -> Vec<TimedMidi> {
        self.clock = None;
        let mut out: Vec<TimedMidi> = self
            .sounding
            .drain(..)
            .map(|s| TimedMidi {
                t_ms: now_ms,
                bytes: note_off(s.channel, s.pitch),
            })
            .collect();
        let tail_ms = self.last_t_ms.max(now_ms) + 1.0;
        for t_ms in [now_ms, tail_ms] {
            for i in 0..self.pattern.tracks.len() {
                out.push(TimedMidi {
                    t_ms,
                    bytes: all_notes_off(track_channel(state, i)),
                });
            }
        }
        out
    }

    /// CC 123 on all 16 MIDI channels, to send immediately.
    pub fn panic() -> Vec<Vec<u8>> {
        (0..MIDI_CHANNELS).map(all_notes_off).collect()
    }
}

fn track_channel(state: &DeviceState, track: usize) -> u8 {
    state
        .state()
        .parts
        .get(track)
        .map_or(0, |p| p.receive_channel & 0x0F)
}

fn note_off(channel: u8, pitch: u8) -> Vec<u8> {
    vec![0x80 | channel, pitch, 0]
}

fn all_notes_off(channel: u8) -> Vec<u8> {
    vec![0xB0 | channel, CC_ALL_NOTES_OFF, 0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_spec::INTEGRA7;

    /// 120 BPM: one Step is 125 ms.
    const STEP: f64 = 125.0;

    fn state() -> DeviceState {
        let mut s = DeviceState::new(0x10);
        for (i, p) in s.state_mut().parts.iter_mut().enumerate() {
            p.receive_channel = i as u8;
        }
        s
    }

    fn seq_with(f: impl FnOnce(&mut Pattern)) -> Sequencer {
        let mut p = Pattern::new(INTEGRA7.part_count);
        f(&mut p);
        Sequencer::new(p)
    }

    fn ons(events: &[TimedMidi]) -> Vec<(f64, u8, u8)> {
        events
            .iter()
            .filter(|e| e.bytes[0] & 0xF0 == 0x90)
            .map(|e| (e.t_ms, e.bytes[0] & 0x0F, e.bytes[1]))
            .collect()
    }

    fn offs(events: &[TimedMidi]) -> Vec<(f64, u8, u8)> {
        events
            .iter()
            .filter(|e| e.bytes[0] & 0xF0 == 0x80)
            .map(|e| (e.t_ms, e.bytes[0] & 0x0F, e.bytes[1]))
            .collect()
    }

    #[test]
    fn overlapping_due_calls_emit_each_event_once() {
        let pattern = |p: &mut Pattern| {
            let t = p.track_mut(0).unwrap();
            t.set_step(0, vec![Note::new(60, 100, 1), Note::new(64, 100, 2)]);
            t.set_step(5, vec![Note::new(67, 90, 3)]);
            p.track_mut(3).unwrap().set_length(5);
            p.track_mut(3)
                .unwrap()
                .set_step(2, vec![Note::new(40, 80, 1)]);
        };
        let st = state();

        let mut once = seq_with(pattern);
        once.play(0.0);
        let expected = once.due(&st, 0.0, 4000.0);

        let mut ticked = seq_with(pattern);
        ticked.play(0.0);
        let mut got = Vec::new();
        let mut now = 0.0;
        while now + 100.0 <= 4000.0 {
            got.extend(ticked.due(&st, now, 100.0));
            now += 25.0;
        }
        got.extend(ticked.due(&st, 3900.0, 100.0));

        assert!(!expected.is_empty());
        assert_eq!(got, expected);
    }

    #[test]
    fn tracks_loop_on_their_own_length() {
        let mut seq = seq_with(|p| {
            p.track_mut(0).unwrap().set_length(3);
            p.track_mut(0)
                .unwrap()
                .set_step(0, vec![Note::new(60, 100, 1)]);
            p.track_mut(1).unwrap().set_length(4);
            p.track_mut(1)
                .unwrap()
                .set_step(0, vec![Note::new(62, 100, 1)]);
        });
        seq.play(0.0);
        let on = ons(&seq.due(&state(), 0.0, 11.0 * STEP));
        let at = |ch: u8| -> Vec<f64> { on.iter().filter(|e| e.1 == ch).map(|e| e.0).collect() };
        assert_eq!(at(0), vec![0.0, 3.0 * STEP, 6.0 * STEP, 9.0 * STEP]);
        assert_eq!(at(1), vec![0.0, 4.0 * STEP, 8.0 * STEP]);
    }

    #[test]
    fn ties_hold_and_clamp_at_track_end() {
        let mut seq = seq_with(|p| {
            let t = p.track_mut(0).unwrap();
            t.set_step(0, vec![Note::new(60, 100, 4)]);
            t.set_step(14, vec![Note::new(62, 100, 4)]);
        });
        seq.play(0.0);
        let off = offs(&seq.due(&state(), 0.0, 16.0 * STEP));
        assert_eq!(off, vec![(4.0 * STEP, 0, 60), (16.0 * STEP, 0, 62)]);
    }

    #[test]
    fn off_sorts_before_on_on_the_same_tick() {
        let mut seq = seq_with(|p| {
            let t = p.track_mut(0).unwrap();
            t.set_step(0, vec![Note::new(60, 100, 1)]);
            t.set_step(1, vec![Note::new(60, 100, 1)]);
        });
        seq.play(0.0);
        let events = seq.due(&state(), 0.0, STEP);
        let kinds: Vec<(f64, u8)> = events.iter().map(|e| (e.t_ms, e.bytes[0] & 0xF0)).collect();
        assert_eq!(kinds, vec![(0.0, 0x90), (STEP, 0x80), (STEP, 0x90)]);
    }

    #[test]
    fn sounding_note_keeps_its_channel() {
        let mut st = state();
        let mut seq = seq_with(|p| {
            p.track_mut(2).unwrap().set_length(4);
            p.track_mut(2)
                .unwrap()
                .set_step(0, vec![Note::new(60, 100, 2)]);
        });
        seq.play(0.0);
        assert_eq!(ons(&seq.due(&st, 0.0, 10.0)), vec![(0.0, 2, 60)]);

        st.state_mut().parts[2].receive_channel = 9;
        let events = seq.due(&st, 10.0, 4.0 * STEP);
        assert_eq!(offs(&events), vec![(2.0 * STEP, 2, 60)]);
        assert_eq!(ons(&events), vec![(4.0 * STEP, 9, 60)]);
    }

    #[test]
    fn edits_do_not_touch_sounding_notes() {
        let st = state();
        let mut seq = seq_with(|p| {
            p.track_mut(0)
                .unwrap()
                .set_step(0, vec![Note::new(60, 100, 3)]);
        });
        seq.play(0.0);
        seq.due(&st, 0.0, 10.0);

        let t = seq.pattern_mut().track_mut(0).unwrap();
        t.set_step(0, vec![]);
        t.set_length(1);
        let events = seq.due(&st, 10.0, 3.0 * STEP);
        assert_eq!(offs(&events), vec![(3.0 * STEP, 0, 60)]);
        assert!(ons(&events).is_empty());
    }

    #[test]
    fn stop_silences_in_order_with_trailing_cc123() {
        let st = state();
        let mut seq = seq_with(|p| {
            p.track_mut(1)
                .unwrap()
                .set_step(0, vec![Note::new(60, 100, 8)]);
            p.track_mut(1)
                .unwrap()
                .set_step(1, vec![Note::new(64, 100, 8)]);
        });
        seq.play(0.0);
        seq.due(&st, 0.0, 100.0 + STEP);

        let events = seq.stop(&st, 100.0);
        let tracks = usize::from(INTEGRA7.part_count);
        assert_eq!(events.len(), 2 + 2 * tracks);
        assert_eq!(offs(&events[..2]), vec![(100.0, 1, 60), (100.0, 1, 64)]);
        for (i, e) in events[2..2 + tracks].iter().enumerate() {
            assert_eq!(
                (e.t_ms, e.bytes.clone()),
                (100.0, vec![0xB0 | i as u8, 123, 0])
            );
        }
        for (i, e) in events[2 + tracks..].iter().enumerate() {
            assert_eq!(
                (e.t_ms, e.bytes.clone()),
                (STEP + 1.0, vec![0xB0 | i as u8, 123, 0])
            );
        }
        assert!(!seq.is_playing());
        assert!(seq.due(&st, 200.0, 100.0).is_empty());
    }

    #[test]
    fn panic_covers_all_channels() {
        let msgs = Sequencer::panic();
        assert_eq!(msgs.len(), 16);
        assert_eq!(msgs[15], vec![0xBF, 123, 0]);
    }

    #[test]
    fn tempo_change_keeps_emitted_steps() {
        let st = state();
        let mut seq = seq_with(|p| {
            p.track_mut(0).unwrap().set_length(1);
            p.track_mut(0)
                .unwrap()
                .set_step(0, vec![Note::new(60, 100, 1)]);
        });
        seq.play(0.0);
        seq.due(&st, 0.0, STEP);
        seq.set_bpm(60);
        let on = ons(&seq.due(&st, STEP, 2.0 * 250.0));
        assert_eq!(on, vec![(2.0 * STEP, 0, 60), (2.0 * STEP + 250.0, 0, 60)]);
    }
}
