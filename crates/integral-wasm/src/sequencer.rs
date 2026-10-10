//! WASM handle for the step Sequencer.
//!
//! The JS side owns the clock: it calls `due` from a Worker-driven tick with
//! `performance.now()` and sends each event with a timestamped
//! `MIDIOutput.send`. Receive channels come from the `WasmDeviceState` passed
//! to each call.

use integral_core::device_spec::INTEGRA7;
use integral_core::sequencer::{MAX_STEPS, Pattern, Sequencer, TimedMidi};
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::WasmDeviceState;

/// A MIDI message to send at `tMs` on the `performance.now()` clock.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TimedMidiJs {
    t_ms: f64,
    bytes: Vec<u8>,
}

fn to_js(events: Vec<TimedMidi>) -> Result<JsValue, JsError> {
    let events: Vec<TimedMidiJs> = events
        .into_iter()
        .map(|e| TimedMidiJs {
            t_ms: e.t_ms,
            bytes: e.bytes,
        })
        .collect();
    serde_wasm_bindgen::to_value(&events).map_err(|e| JsError::new(&e.to_string()))
}

/// One note, as `{ pitch, velocity, length }`.
#[derive(Serialize)]
struct NoteJs {
    pitch: u8,
    velocity: u8,
    length: u8,
}

/// The notes starting on one Step.
#[derive(Serialize)]
struct StepJs {
    step: u8,
    notes: Vec<NoteJs>,
}

/// One Track: its length and its non-empty Steps.
#[derive(Serialize)]
struct TrackJs {
    length: u8,
    steps: Vec<StepJs>,
}

/// The app-level Sequencer: the Pattern plus its transport.
#[wasm_bindgen]
pub struct WasmSequencer {
    inner: Sequencer,
}

impl Default for WasmSequencer {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmSequencer {
    /// Create a stopped Sequencer with an empty Pattern, one Track per Part.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Sequencer::new(Pattern::new(INTEGRA7.part_count)),
        }
    }

    /// Events in `[now_ms, now_ms + horizon_ms]` not returned before, as
    /// `{ tMs, bytes }[]`.
    pub fn due(
        &mut self,
        device: &WasmDeviceState,
        now_ms: f64,
        horizon_ms: f64,
    ) -> Result<JsValue, JsError> {
        to_js(self.inner.due(&device.inner, now_ms, horizon_ms))
    }

    /// Start playing with Step 0 at `now_ms`.
    pub fn play(&mut self, now_ms: f64) {
        self.inner.play(now_ms);
    }

    /// Stop and return the events that silence every Track.
    pub fn stop(&mut self, device: &WasmDeviceState, now_ms: f64) -> Result<JsValue, JsError> {
        to_js(self.inner.stop(&device.inner, now_ms))
    }

    /// CC 123 on all 16 channels, as `number[][]` to send immediately.
    pub fn panic() -> Result<JsValue, JsError> {
        serde_wasm_bindgen::to_value(&Sequencer::panic()).map_err(|e| JsError::new(&e.to_string()))
    }

    /// Maximum Steps in a Track.
    #[wasm_bindgen(js_name = maxSteps)]
    pub fn max_steps() -> u8 {
        MAX_STEPS
    }

    /// Whether the transport is playing.
    #[wasm_bindgen(js_name = isPlaying)]
    pub fn is_playing(&self) -> bool {
        self.inner.is_playing()
    }

    /// Tempo in BPM.
    pub fn bpm(&self) -> u16 {
        self.inner.pattern().bpm()
    }

    /// Change the tempo, keeping already-emitted Steps in place.
    #[wasm_bindgen(js_name = setBpm)]
    pub fn set_bpm(&mut self, bpm: u16) {
        self.inner.set_bpm(bpm);
    }

    /// The Pattern as `{ length, steps: { step, notes }[] }[]`, one per Track.
    pub fn tracks(&self) -> Result<JsValue, JsError> {
        let tracks: Vec<TrackJs> = self
            .inner
            .pattern()
            .tracks()
            .iter()
            .map(|t| TrackJs {
                length: t.length(),
                steps: t
                    .steps()
                    .map(|(step, notes)| StepJs {
                        step,
                        notes: notes
                            .iter()
                            .map(|n| NoteJs {
                                pitch: n.pitch,
                                velocity: n.velocity,
                                length: n.length,
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect();
        serde_wasm_bindgen::to_value(&tracks).map_err(|e| JsError::new(&e.to_string()))
    }

    /// The global Step sounding at `now_ms`, or `undefined` when stopped.
    #[wasm_bindgen(js_name = stepAt)]
    pub fn step_at(&self, now_ms: f64) -> Option<f64> {
        self.inner.step_at(now_ms).map(|s| s as f64)
    }

    /// Set a Track's length in Steps (clamped to 1–64).
    #[wasm_bindgen(js_name = setTrackLength)]
    pub fn set_track_length(&mut self, track: usize, length: u8) {
        if let Some(t) = self.inner.pattern_mut().track_mut(track) {
            t.set_length(length);
        }
    }

    /// Set the velocity of every note in the chord starting on `step`.
    #[wasm_bindgen(js_name = setChordVelocity)]
    pub fn set_chord_velocity(&mut self, track: usize, step: u8, velocity: u8) {
        if let Some(t) = self.inner.pattern_mut().track_mut(track) {
            t.set_velocity(step, velocity);
        }
    }

    /// The Pattern as Standard MIDI File bytes.
    #[wasm_bindgen(js_name = toSmf)]
    pub fn to_smf(&self, device: &WasmDeviceState) -> Vec<u8> {
        self.inner.pattern().to_smf(&device.inner)
    }

    /// Replace the Pattern with one read from Standard MIDI File bytes.
    /// Notes already sounding are unaffected.
    #[wasm_bindgen(js_name = loadSmf)]
    pub fn load_smf(&mut self, bytes: &[u8]) -> Result<(), JsError> {
        let pattern = Pattern::from_smf(bytes, INTEGRA7.part_count)?;
        *self.inner.pattern_mut() = pattern;
        Ok(())
    }
}
