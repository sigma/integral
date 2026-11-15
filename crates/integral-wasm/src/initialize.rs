//! WASM bindings for the device's Init of a Temporary area
//! (`integral_core::initialize`).
//!
//! `Initialization` is driven like the Librarian machines: `nextAction`
//! returns a `LibrarianAction` (`Send` the Init, `AwaitWrite` its "Init
//! finished", then `Done`), DT1s go to `onWriteReply` and a wait that ran
//! out to `onTimeout`. On `Done` with no error, read the reset area back:
//! the device announces none of its values.

use integral_core::initialize::{INIT_FINISHED_ADDR, InitAction, InitTarget, Initialization};
use wasm_bindgen::prelude::*;

use crate::ToneType;
use crate::librarian::{LibrarianAction, LibrarianActionKind, address, js_err};

/// Address of the device's "Init finished" DT1.
#[wasm_bindgen(js_name = initFinishedAddress)]
pub fn init_finished_address() -> Vec<u8> {
    INIT_FINISHED_ADDR.as_bytes().to_vec()
}

/// Sends one Init and waits for "Init finished". Parts are 0-indexed.
#[wasm_bindgen(js_name = Initialization)]
pub struct WasmInitialization {
    inner: Initialization,
}

impl WasmInitialization {
    fn of(device_id: u8, target: InitTarget) -> Result<Self, JsError> {
        Ok(Self {
            inner: Initialization::new(device_id, target).map_err(js_err)?,
        })
    }
}

#[wasm_bindgen(js_class = Initialization)]
impl WasmInitialization {
    /// Studio Set Init: Studio Set Common and all 16 Parts; every Part's
    /// tone reloads.
    #[wasm_bindgen(js_name = studioSet)]
    pub fn studio_set(device_id: u8) -> Result<WasmInitialization, JsError> {
        Self::of(device_id, InitTarget::StudioSet)
    }

    /// Sound Ctrl Init: the eight Sound Ctrl offsets of every Part.
    #[wasm_bindgen(js_name = soundCtrl)]
    pub fn sound_ctrl(device_id: u8) -> Result<WasmInitialization, JsError> {
        Self::of(device_id, InitTarget::SoundCtrl)
    }

    /// Part Init: Part `part`'s Studio Set Part block; its tone reloads.
    pub fn part(device_id: u8, part: u8) -> Result<WasmInitialization, JsError> {
        Self::of(device_id, InitTarget::Part(part))
    }

    /// Tone Init: Part `part`'s Temporary tone, which keeps its type.
    pub fn tone(device_id: u8, part: u8) -> Result<WasmInitialization, JsError> {
        Self::of(device_id, InitTarget::Tone(part))
    }

    /// Partial Init of Part `part`, which plays a tone of `tone_type`:
    /// `partial` is the 0-based partial index for a tone, the MIDI key
    /// number for a drum kit. Throws for an SN Acoustic tone, which has no
    /// partials.
    pub fn partial(
        device_id: u8,
        part: u8,
        tone_type: ToneType,
        partial: u8,
    ) -> Result<WasmInitialization, JsError> {
        let target = InitTarget::partial(part, tone_type.into(), partial).map_err(js_err)?;
        Self::of(device_id, target)
    }

    /// The next step. On `Done` with no error, read the reset area back.
    #[wasm_bindgen(js_name = nextAction)]
    pub fn next_action(&mut self) -> LibrarianAction {
        match self.inner.next_action() {
            InitAction::Send(bytes) => {
                LibrarianAction::with_bytes(LibrarianActionKind::Send, bytes)
            }
            InitAction::Await { timeout_ms } => LibrarianAction::await_write(timeout_ms),
            InitAction::Done(result) => LibrarianAction::done(result.err().map(|e| e.to_string())),
        }
    }

    /// An Init reads nothing: always false. Present so the Librarian
    /// driver can run it.
    #[wasm_bindgen(js_name = onRead)]
    pub fn on_read(&mut self, _addr: &[u8], _data: &[u8]) -> bool {
        false
    }

    /// Feed an incoming DT1. Returns true if it was the awaited "Init
    /// finished".
    #[wasm_bindgen(js_name = onWriteReply)]
    pub fn on_write_reply(&mut self, addr: &[u8], data: &[u8]) -> bool {
        address(addr).is_some_and(|a| self.inner.on_reply(&a, data))
    }

    /// The wait ran out: give up on "Init finished".
    #[wasm_bindgen(js_name = onTimeout)]
    pub fn on_timeout(&mut self) {
        self.inner.on_timeout();
    }

    /// Stop before the Init is sent; once sent, "finished" is still
    /// awaited.
    pub fn cancel(&mut self) {
        self.inner.cancel();
    }
}

#[cfg(test)]
mod tests {
    use integral_core::initialize::INIT_TIMEOUT_MS;

    use super::*;

    const FINISHED: [u8; 4] = *INIT_FINISHED_ADDR.as_bytes();

    #[test]
    fn sends_the_init_then_awaits_finished() {
        let Ok(mut init) = WasmInitialization::partial(0x10, 15, ToneType::PcmDrum, 36) else {
            panic!("key 36 is on a PCM Drum Kit");
        };
        let send = init.next_action();
        assert_eq!(send.kind, LibrarianActionKind::Send);
        assert_eq!(
            &send.bytes()[7..15],
            [0x0F, 0x00, 0x18, 0x30, 0x7F, 0x7F, 0x0F, 0x24]
        );
        let wait = init.next_action();
        assert_eq!(wait.kind, LibrarianActionKind::AwaitWrite);
        assert_eq!(wait.timeout_ms, INIT_TIMEOUT_MS);
        assert!(!init.on_read(&FINISHED, &[0; 4]));
        assert!(!init.on_write_reply(&[0x0F, 0x00, 0x18], &[0; 4]));
        assert!(init.on_write_reply(&FINISHED, &[0; 4]));
        let done = init.next_action();
        assert_eq!(done.kind, LibrarianActionKind::Done);
        assert_eq!(done.error(), None);
    }

    #[test]
    fn every_target_builds() {
        for init in [
            WasmInitialization::studio_set(0x10),
            WasmInitialization::sound_ctrl(0x10),
            WasmInitialization::part(0x10, 0),
            WasmInitialization::tone(0x10, 0),
        ] {
            let Ok(mut init) = init else {
                panic!("a valid target");
            };
            assert_eq!(init.next_action().kind, LibrarianActionKind::Send);
        }
    }

    #[test]
    fn a_timeout_reports_an_error() {
        let Ok(mut init) = WasmInitialization::sound_ctrl(0x10) else {
            panic!("a valid target");
        };
        init.next_action();
        init.on_timeout();
        let done = init.next_action();
        assert_eq!(done.kind, LibrarianActionKind::Done);
        assert!(done.error().is_some());
    }
}
