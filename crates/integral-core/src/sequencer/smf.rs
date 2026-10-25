//! Pattern persistence as a Standard MIDI File (ADR 0001).
//!
//! Layout: type 1, PPQ 96 (one Step = 24 ticks). Track 0 is the conductor
//! (tempo and 4/4), then one `MTrk` per Track in Part order, named `Part N`.
//! A Track's length is the time of its End-of-Track event.
//!
//! A note retriggered by a later onset of its pitch ends there, both when
//! written and when read, so overlapping same-pitch notes round-trip as they
//! play. Only notes and End-of-Track must sit on the Step grid; other events
//! are ignored wherever they fall.

use std::collections::BTreeMap;

use midly::num::{u4, u7, u15, u24, u28};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use thiserror::Error;

use super::{MAX_STEPS, Note, Pattern, STEPS_PER_QUARTER, Track, track_channel};
use crate::state::MixerState;

/// Ticks per quarter note.
const PPQ: u16 = 96;

/// Ticks per Step.
const TICKS_PER_STEP: u32 = (PPQ / STEPS_PER_QUARTER) as u32;

/// Microseconds per minute, to convert BPM to an SMF tempo.
const MICROS_PER_MINUTE: f64 = 60_000_000.0;

/// Why a file could not be read as a Pattern.
#[derive(Debug, Error)]
pub enum SmfError {
    /// The bytes are not a valid Standard MIDI File.
    #[error("not a valid MIDI file: {0}")]
    Parse(#[from] midly::Error),
    /// Only type 1 files are supported.
    #[error("unsupported MIDI file type: expected type 1")]
    Format,
    /// Only PPQ 96 files are supported.
    #[error("unsupported timing: expected {PPQ} ticks per quarter note")]
    Timing,
    /// The file does not hold one track per Part.
    #[error("expected {expected} tracks, found {found}")]
    TrackCount {
        /// Tracks the device has.
        expected: usize,
        /// Tracks found in the file.
        found: usize,
    },
    /// A note or End-of-Track does not fall on the 16th-note grid.
    #[error("track {track}: event at tick {tick} is off the step grid")]
    OffGrid {
        /// Track number (1-based, as in `Part N`).
        track: usize,
        /// Absolute tick of the event.
        tick: u32,
    },
    /// A Track is empty or longer than [`MAX_STEPS`].
    #[error("track {track}: length of {steps} steps is outside 1..={MAX_STEPS}")]
    Length {
        /// Track number (1-based).
        track: usize,
        /// Length found, in Steps.
        steps: u32,
    },
    /// A Note On without a matching Note Off.
    #[error("track {track}: note {pitch} is never released")]
    Unpaired {
        /// Track number (1-based).
        track: usize,
        /// MIDI note number.
        pitch: u8,
    },
}

impl Pattern {
    /// Encode the Pattern as SMF bytes.
    ///
    /// Each Track's channel nibble is its Part's receive channel in `state`;
    /// pass [`MixerState::default`] when no device is connected.
    /// Notes are written as they play: clamped at the Track end and at the
    /// next onset of the same pitch, and notes starting past the end are
    /// dropped.
    pub fn to_smf(&self, state: &MixerState) -> Vec<u8> {
        let tempo = (MICROS_PER_MINUTE / f64::from(self.bpm)).round() as u32;
        let conductor = vec![
            meta(0, MetaMessage::Tempo(u24::new(tempo))),
            meta(0, MetaMessage::TimeSignature(4, 2, 24, 8)),
            meta(0, MetaMessage::EndOfTrack),
        ];

        let names: Vec<String> = (1..=self.tracks.len())
            .map(|n| format!("Part {n}"))
            .collect();
        let mut tracks = vec![conductor];
        for (i, track) in self.tracks.iter().enumerate() {
            let channel = u4::new(track_channel(state, i));
            // (tick, is_on, message): an Off sorts before an On on the same tick.
            let mut events: Vec<(u32, bool, MidiMessage)> = Vec::new();
            for (start, notes) in track.steps().filter(|(s, _)| *s < track.length) {
                for note in notes {
                    // A later onset of the same pitch retriggers it: end there.
                    // shortcut: a pitch repeated within one chord (only a foreign file
                    // with two same-tick Note Ons; the editor's chords are sets) does
                    // not round-trip; dedupe in `Track::set_step` if that matters.
                    let retrigger = track
                        .steps()
                        .find(|(s, ns)| *s > start && ns.iter().any(|n| n.pitch == note.pitch))
                        .map_or(track.length, |(s, _)| s.min(track.length));
                    let end = start + note.length.min(retrigger - start);
                    let key = u7::new(note.pitch);
                    let vel = u7::new(note.velocity);
                    events.push((
                        u32::from(start) * TICKS_PER_STEP,
                        true,
                        MidiMessage::NoteOn { key, vel },
                    ));
                    let off = MidiMessage::NoteOff {
                        key,
                        vel: u7::new(0),
                    };
                    events.push((u32::from(end) * TICKS_PER_STEP, false, off));
                }
            }
            events.sort_by_key(|(tick, is_on, _)| (*tick, *is_on));

            let mut mtrk = vec![meta(0, MetaMessage::TrackName(names[i].as_bytes()))];
            let mut last = 0;
            for (tick, _, message) in events {
                mtrk.push(TrackEvent {
                    delta: u28::new(tick - last),
                    kind: TrackEventKind::Midi { channel, message },
                });
                last = tick;
            }
            let end = u32::from(track.length) * TICKS_PER_STEP;
            mtrk.push(meta(end - last, MetaMessage::EndOfTrack));
            tracks.push(mtrk);
        }

        let smf = Smf {
            header: Header::new(Format::Parallel, Timing::Metrical(u15::new(PPQ))),
            tracks,
        };
        let mut bytes = Vec::new();
        smf.write_std(&mut bytes)
            .expect("writing a well-formed SMF to memory cannot fail");
        bytes
    }

    /// Decode a Pattern written by [`Pattern::to_smf`].
    ///
    /// Channels in the file are ignored: Track identity comes from `MTrk`
    /// order. Files we did not write (type 0, other PPQ, off-grid timing,
    /// Tracks over 64 Steps) are rejected.
    pub fn from_smf(bytes: &[u8], part_count: u8) -> Result<Self, SmfError> {
        let smf = Smf::parse(bytes)?;
        if smf.header.format != Format::Parallel {
            return Err(SmfError::Format);
        }
        if smf.header.timing != Timing::Metrical(u15::new(PPQ)) {
            return Err(SmfError::Timing);
        }
        let expected = usize::from(part_count);
        let found = smf.tracks.len().saturating_sub(1);
        if found != expected {
            return Err(SmfError::TrackCount { expected, found });
        }

        let mut pattern = Pattern::new(part_count);
        let tempo = smf.tracks[0].iter().find_map(|e| match e.kind {
            TrackEventKind::Meta(MetaMessage::Tempo(t)) => Some(t.as_int()),
            _ => None,
        });
        if let Some(tempo) = tempo {
            pattern.set_bpm_raw((MICROS_PER_MINUTE / f64::from(tempo.max(1))).round() as u16);
        }

        for (i, (events, track)) in smf.tracks[1..].iter().zip(&mut pattern.tracks).enumerate() {
            *track = read_track(events, i + 1)?;
        }
        Ok(pattern)
    }
}

fn read_track(events: &[TrackEvent], number: usize) -> Result<Track, SmfError> {
    let step_of = |tick: u32| -> Result<u8, SmfError> {
        let steps = tick / TICKS_PER_STEP;
        if !tick.is_multiple_of(TICKS_PER_STEP) {
            return Err(SmfError::OffGrid {
                track: number,
                tick,
            });
        }
        if steps > u32::from(MAX_STEPS) {
            return Err(SmfError::Length {
                track: number,
                steps,
            });
        }
        Ok(steps as u8)
    };

    let mut steps: BTreeMap<u8, Vec<Note>> = BTreeMap::new();
    // Pitch -> (start Step, index in that Step's chord).
    let mut open: BTreeMap<u8, (u8, usize)> = BTreeMap::new();
    let mut tick = 0u32;
    for event in events {
        tick += event.delta.as_int();
        match event.kind {
            TrackEventKind::Midi {
                message: MidiMessage::NoteOn { key, vel },
                ..
            } if vel.as_int() > 0 => {
                let start = step_of(tick)?;
                // A retrigger cuts the open note, as the device plays it.
                if let Some((s, i)) = open.remove(&key.as_int()) {
                    steps.get_mut(&s).expect("open note has a step")[i].length = (start - s).max(1);
                }
                let chord = steps.entry(start).or_default();
                open.insert(key.as_int(), (start, chord.len()));
                chord.push(Note::new(key.as_int(), vel.as_int(), 1));
            }
            TrackEventKind::Midi {
                message: MidiMessage::NoteOff { key, .. } | MidiMessage::NoteOn { key, .. },
                ..
            } => {
                let end = step_of(tick)?;
                if let Some((start, index)) = open.remove(&key.as_int()) {
                    steps.get_mut(&start).expect("open note has a step")[index].length =
                        end - start;
                }
            }
            TrackEventKind::Meta(MetaMessage::EndOfTrack) => break,
            _ => {}
        }
    }

    if let Some((&pitch, _)) = open.iter().next() {
        return Err(SmfError::Unpaired {
            track: number,
            pitch,
        });
    }
    let length = step_of(tick)?;
    if length == 0 {
        return Err(SmfError::Length {
            track: number,
            steps: 0,
        });
    }
    Ok(Track { length, steps })
}

fn meta(delta: u32, message: MetaMessage<'_>) -> TrackEvent<'_> {
    TrackEvent {
        delta: u28::new(delta),
        kind: TrackEventKind::Meta(message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_spec::INTEGRA7;

    fn state() -> MixerState {
        MixerState::default()
    }

    fn sample() -> Pattern {
        let mut p = Pattern::new(INTEGRA7.part_count);
        p.set_bpm_raw(93);
        let t = p.track_mut(0).unwrap();
        t.set_step(
            0,
            vec![
                Note::new(60, 100, 1),
                Note::new(64, 90, 1),
                Note::new(67, 80, 1),
            ],
        );
        t.set_step(1, vec![Note::new(60, 100, 1)]);
        t.set_step(4, vec![Note::new(48, 127, 8)]);
        let t = p.track_mut(5).unwrap();
        t.set_length(7);
        t.set_step(6, vec![Note::new(36, 1, 1)]);
        let t = p.track_mut(15).unwrap();
        t.set_length(64);
        t.set_step(0, vec![Note::new(72, 64, 64)]);
        p
    }

    #[test]
    fn round_trips_chords_ties_lengths_and_tempo() {
        let p = sample();
        let bytes = p.to_smf(&state());
        let back = Pattern::from_smf(&bytes, INTEGRA7.part_count).unwrap();
        assert_eq!(back, p);
        assert_eq!(back.to_smf(&state()), bytes);
    }

    #[test]
    fn tracks_are_named_and_carry_receive_channels() {
        let smf_bytes = sample().to_smf(&state());
        let smf = Smf::parse(&smf_bytes).unwrap();
        assert_eq!(smf.tracks.len(), 17);
        assert!(matches!(
            smf.tracks[16][0].kind,
            TrackEventKind::Meta(MetaMessage::TrackName(b"Part 16"))
        ));
        let channel = smf.tracks[6].iter().find_map(|e| match e.kind {
            TrackEventKind::Midi { channel, .. } => Some(channel.as_int()),
            _ => None,
        });
        assert_eq!(channel, Some(5));
    }

    #[test]
    fn offline_export_puts_each_track_on_its_part_channel() {
        let smf_bytes = sample().to_smf(&MixerState::default());
        let smf = Smf::parse(&smf_bytes).unwrap();
        for (part, track) in smf.tracks[1..].iter().enumerate() {
            let mut channels = track.iter().filter_map(|e| match e.kind {
                TrackEventKind::Midi { channel, .. } => Some(usize::from(channel.as_int())),
                _ => None,
            });
            assert!(channels.all(|c| c == part), "Part {} channel", part + 1);
        }
    }

    #[test]
    fn channels_are_ignored_on_read() {
        let mut other = state();
        other.parts[0].receive_channel = 9;
        let back = Pattern::from_smf(&sample().to_smf(&other), INTEGRA7.part_count).unwrap();
        assert_eq!(back, sample());
    }

    #[test]
    fn writes_notes_as_they_play() {
        let mut p = Pattern::new(INTEGRA7.part_count);
        let t = p.track_mut(0).unwrap();
        t.set_step(14, vec![Note::new(60, 100, 4)]);
        t.set_step(20, vec![Note::new(62, 100, 1)]);
        let back = Pattern::from_smf(&p.to_smf(&state()), INTEGRA7.part_count).unwrap();
        let steps: Vec<(u8, Vec<Note>)> = back.tracks()[0]
            .steps()
            .map(|(s, n)| (s, n.to_vec()))
            .collect();
        assert_eq!(steps, vec![(14, vec![Note::new(60, 100, 2)])]);
    }

    fn file(format: Format, ppq: u16, tracks: Vec<Vec<TrackEvent<'static>>>) -> Vec<u8> {
        let smf = Smf {
            header: Header::new(format, Timing::Metrical(u15::new(ppq))),
            tracks,
        };
        let mut bytes = Vec::new();
        smf.write_std(&mut bytes).unwrap();
        bytes
    }

    fn padded(track: Vec<TrackEvent<'static>>) -> Vec<Vec<TrackEvent<'static>>> {
        let mut tracks = vec![vec![meta(0, MetaMessage::EndOfTrack)]; 17];
        tracks[1] = track;
        tracks
    }

    fn note(delta: u32, on: bool) -> TrackEvent<'static> {
        let (key, vel) = (u7::new(60), u7::new(100));
        let message = if on {
            MidiMessage::NoteOn { key, vel }
        } else {
            MidiMessage::NoteOff { key, vel }
        };
        TrackEvent {
            delta: u28::new(delta),
            kind: TrackEventKind::Midi {
                channel: u4::new(0),
                message,
            },
        }
    }

    fn first_track_steps(p: &Pattern) -> Vec<(u8, Vec<Note>)> {
        p.tracks()[0]
            .steps()
            .map(|(s, n)| (s, n.to_vec()))
            .collect()
    }

    /// Read a file whose first Track is `track` and whose other Tracks are
    /// one empty Step.
    fn read_first_track(track: Vec<TrackEvent<'static>>) -> Pattern {
        let empty = vec![meta(24, MetaMessage::EndOfTrack)];
        let mut tracks = vec![empty; usize::from(INTEGRA7.part_count) + 1];
        tracks[1] = track;
        Pattern::from_smf(&file(Format::Parallel, PPQ, tracks), INTEGRA7.part_count).unwrap()
    }

    #[test]
    fn a_retriggered_note_is_cut_at_the_retrigger() {
        // On 0, On 2 (same pitch), Off 4, Off 8: the second On cuts the first.
        let eot = meta(0, MetaMessage::EndOfTrack);
        let track = vec![
            note(0, true),
            note(48, true),
            note(48, false),
            note(96, false),
            eot,
        ];
        assert_eq!(
            first_track_steps(&read_first_track(track)),
            vec![
                (0, vec![Note::new(60, 100, 2)]),
                (2, vec![Note::new(60, 100, 2)])
            ]
        );
    }

    #[test]
    fn overlapping_same_pitch_notes_round_trip_cut_at_the_retrigger() {
        let mut p = Pattern::new(INTEGRA7.part_count);
        let t = p.track_mut(0).unwrap();
        t.set_step(0, vec![Note::new(60, 100, 2)]);
        t.set_step(1, vec![Note::new(60, 90, 8)]);

        let bytes = p.to_smf(&state());
        let back = Pattern::from_smf(&bytes, INTEGRA7.part_count).unwrap();
        // The first note ends where the second starts; the second keeps its length.
        assert_eq!(
            first_track_steps(&back),
            vec![
                (0, vec![Note::new(60, 100, 1)]),
                (1, vec![Note::new(60, 90, 8)])
            ]
        );
        assert_eq!(back.to_smf(&state()), bytes);
    }

    #[test]
    fn off_grid_events_other_than_notes_are_ignored() {
        let controller = TrackEvent {
            delta: u28::new(10),
            kind: TrackEventKind::Midi {
                channel: u4::new(0),
                message: MidiMessage::Controller {
                    controller: u7::new(7),
                    value: u7::new(100),
                },
            },
        };
        // On at 0, a controller at tick 10, Off and End-of-Track at 24.
        let eot = meta(0, MetaMessage::EndOfTrack);
        let back = read_first_track(vec![note(0, true), controller, note(14, false), eot]);
        assert_eq!(
            first_track_steps(&back),
            vec![(0, vec![Note::new(60, 100, 1)])]
        );
    }

    #[test]
    fn rejects_files_we_did_not_write() {
        let parse = |b: Vec<u8>| Pattern::from_smf(&b, INTEGRA7.part_count);
        let eot = |d| meta(d, MetaMessage::EndOfTrack);

        let single = file(Format::SingleTrack, PPQ, vec![vec![eot(24)]]);
        assert!(matches!(parse(single), Err(SmfError::Format)));
        assert!(matches!(
            parse(file(Format::Parallel, 480, padded(vec![eot(24)]))),
            Err(SmfError::Timing)
        ));
        assert!(matches!(
            parse(file(Format::Parallel, PPQ, vec![vec![eot(0)]; 3])),
            Err(SmfError::TrackCount {
                expected: 16,
                found: 2
            })
        ));
        let off_grid = padded(vec![note(10, true), note(24, false), eot(10)]);
        assert!(matches!(
            parse(file(Format::Parallel, PPQ, off_grid)),
            Err(SmfError::OffGrid { tick: 10, .. })
        ));
        let too_long = padded(vec![eot(65 * TICKS_PER_STEP)]);
        assert!(matches!(
            parse(file(Format::Parallel, PPQ, too_long)),
            Err(SmfError::Length { steps: 65, .. })
        ));
        let unpaired = padded(vec![note(0, true), eot(24)]);
        assert!(matches!(
            parse(file(Format::Parallel, PPQ, unpaired)),
            Err(SmfError::Unpaired { pitch: 60, .. })
        ));
        assert!(matches!(parse(b"nope".to_vec()), Err(SmfError::Parse(_))));
    }
}
