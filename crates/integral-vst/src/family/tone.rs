//! The tone selection a Tone or Kit instance saves with its parameters.
//!
//! The recall (ADR 0009) selects this tone on the device before it pushes
//! the tone's values, so the saved type comes back with the tone. The
//! selection is Bank Select MSB, LSB and Program Change, and the tone's name
//! as the device showed it. It is saved under [`PERSIST_KEY`] as JSON:
//! `null`, or `{"msb":87,"lsb":64,"pc":0,"name":"Init Tone"}`.

use std::sync::Mutex;

use integral_core::host_params::PluginKind;
use integral_core::params::{TONE_NAME_SIZE, tone_type_from_bank_msb};
use serde::{Deserialize, Serialize};

/// The state key the tone selection is saved under.
pub const PERSIST_KEY: &str = "tone";

/// Whether `byte` can be a character of a tone name: ASCII from space to DEL,
/// as the device has them.
pub fn is_name_byte(byte: u8) -> bool {
    (0x20..=0x7F).contains(&byte)
}

/// A tone as the device selects it, and its name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedTone {
    /// Bank Select MSB.
    pub msb: u8,
    /// Bank Select LSB.
    pub lsb: u8,
    /// Program Change number.
    pub pc: u8,
    /// The tone's name when it was saved: ASCII (space to DEL) without the
    /// padding the device adds, empty if it was not known.
    #[serde(default)]
    pub name: String,
}

impl SavedTone {
    /// The tone selected by `msb`, `lsb` and `pc`, named `name` as the device
    /// holds it (the padding is dropped).
    pub fn new(msb: u8, lsb: u8, pc: u8, name: &str) -> Self {
        Self {
            msb,
            lsb,
            pc,
            name: name.trim_end().to_string(),
        }
    }

    /// Whether a plugin of `kind` can take this tone: the selection is made
    /// of 7-bit values and names a tone of the plugin's type, and the name is
    /// one the device can have. A state edited by hand, or written by
    /// another plugin, fails this and is ignored.
    pub fn is_valid_for(&self, kind: PluginKind) -> bool {
        kind.tone_type().is_some()
            && tone_type_from_bank_msb(self.msb) == kind.tone_type()
            && self.lsb < 0x80
            && self.pc < 0x80
            && self.name.len() <= TONE_NAME_SIZE.value() as usize
            && self.name.bytes().all(is_name_byte)
    }
}

/// An instance's saved tone, which the worker sets (from the device) and the
/// host's state save and restore read and write.
///
/// A mutex, because the selection is a few values that change together.
/// Never touched by the audio thread: the worker and the host's main thread
/// are its only users.
#[derive(Debug, Default)]
pub struct ToneCell(Mutex<Option<SavedTone>>);

impl ToneCell {
    /// The saved tone, if there is one.
    pub fn get(&self) -> Option<SavedTone> {
        self.0.lock().expect("tone lock").clone()
    }

    /// Replace the saved tone.
    pub fn set(&self, tone: Option<SavedTone>) {
        *self.0.lock().expect("tone lock") = tone;
    }

    /// The tone as it is saved.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.get()).expect("a tone is plain data")
    }

    /// Restore a saved tone for a plugin of `kind`. Text that does not
    /// decode, or holds a tone `kind` cannot take ([`SavedTone::is_valid_for`]),
    /// leaves the cell as it was. Returns whether a tone was restored.
    pub fn restore_json(&self, kind: PluginKind, text: &str) -> bool {
        let Ok(Some(tone)) = serde_json::from_str::<Option<SavedTone>>(text) else {
            return false;
        };
        if !tone.is_valid_for(kind) {
            return false;
        }
        self.set(Some(tone));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PCM Synth, Preset bank, as the device selects it.
    fn tone() -> SavedTone {
        SavedTone::new(87, 64, 3, "Warm Pad   ")
    }

    #[test]
    fn a_saved_tone_restores_in_a_fresh_cell_and_the_padding_is_dropped() {
        let cell = ToneCell::default();
        cell.set(Some(tone()));
        let restored = ToneCell::default();
        assert!(restored.restore_json(PluginKind::PcmSynth, &cell.to_json()));
        assert_eq!(restored.get(), Some(SavedTone::new(87, 64, 3, "Warm Pad")));
    }

    #[test]
    fn a_state_without_a_name_has_an_empty_one_and_del_is_a_name_byte() {
        let cell = ToneCell::default();
        assert!(cell.restore_json(PluginKind::PcmSynth, r#"{"msb":87,"lsb":64,"pc":3}"#));
        assert_eq!(cell.get().unwrap().name, "");
        assert!(is_name_byte(b' ') && is_name_byte(0x7F));
        assert!(!is_name_byte(0x1F) && !is_name_byte(0x80));
    }

    #[test]
    fn the_saved_form_is_readable_json() {
        let cell = ToneCell::default();
        assert_eq!(cell.to_json(), "null");
        cell.set(Some(tone()));
        assert_eq!(
            cell.to_json(),
            r#"{"msb":87,"lsb":64,"pc":3,"name":"Warm Pad"}"#
        );
    }

    #[test]
    fn a_tone_the_plugin_cannot_take_is_not_restored() {
        let cell = ToneCell::default();
        let text = ToneCell::default();
        text.set(Some(tone()));
        let text = text.to_json();
        // Another type's tone, null, garbage, and the Studio Set plugin.
        assert!(!cell.restore_json(PluginKind::SnSynth, &text));
        assert!(!cell.restore_json(PluginKind::PcmSynth, "null"));
        assert!(!cell.restore_json(PluginKind::PcmSynth, "{"));
        assert!(!cell.restore_json(PluginKind::StudioSet, &text));
        for bad in [
            r#"{"msb":87,"lsb":200,"pc":3,"name":""}"#,
            r#"{"msb":87,"lsb":64,"pc":128,"name":""}"#,
            r#"{"msb":87,"lsb":64,"pc":3,"name":"far too long a name"}"#,
            r#"{"msb":87,"lsb":64,"pc":3,"name":"tab\there"}"#,
            r#"{"msb":87,"lsb":64,"pc":3,"name":"café"}"#,
            r#"{"msb":300,"lsb":64,"pc":3,"name":""}"#,
        ] {
            assert!(!cell.restore_json(PluginKind::PcmSynth, bad), "{bad}");
        }
        assert_eq!(cell.get(), None, "nothing was taken");
    }
}
