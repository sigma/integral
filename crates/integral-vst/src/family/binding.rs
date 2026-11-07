//! The Binding of an instance, as the plugin keeps and saves it.
//!
//! A [`BindingCell`] holds `Option<Binding>`: `None` is a new instance that
//! has not taken a target yet. The cell is a single atomic, so the audio
//! thread can read it (a later slice sends writes for the Binding from
//! `process()`) while the worker and the editor set it. It is saved with the
//! host's state under [`PERSIST_KEY`] as JSON: `null`, `{"part":2}` or
//! `{"part":2,"key":36}`.

use std::sync::atomic::{AtomicU32, Ordering};

use integral_core::host_params::{Binding, PluginKind};

/// The state key the Binding is saved under.
pub const PERSIST_KEY: &str = "binding";

const BOUND: u32 = 1 << 18;
const HAS_KEY: u32 = 1 << 17;
const HAS_PART: u32 = 1 << 16;

/// An `Option<Binding>` readable and writable from any thread without
/// locking.
#[derive(Debug)]
pub struct BindingCell(AtomicU32);

impl BindingCell {
    /// A cell holding `binding`.
    pub fn new(binding: Option<Binding>) -> Self {
        Self(AtomicU32::new(encode(binding)))
    }

    /// The Binding, or `None` if the instance has not taken a target.
    pub fn get(&self) -> Option<Binding> {
        decode(self.0.load(Ordering::Acquire))
    }

    /// Replace the Binding.
    pub fn set(&self, binding: Option<Binding>) {
        self.0.store(encode(binding), Ordering::Release);
    }

    /// The Binding as it is saved.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.get()).expect("a Binding is plain data")
    }

    /// Restore a saved Binding for a plugin of `kind`. Text that does not
    /// decode, or names a target `kind` cannot have ([`PluginKind::accepts`];
    /// the Studio Set plugin is always bound, so `null` is refused for it),
    /// leaves the cell as it was. Returns whether it was restored.
    pub fn restore_json(&self, kind: PluginKind, text: &str) -> bool {
        match serde_json::from_str::<Option<Binding>>(text) {
            Ok(Some(binding)) if kind.accepts(&binding) => {
                self.set(Some(binding));
                true
            }
            Ok(None) if kind != PluginKind::StudioSet => {
                self.set(None);
                true
            }
            _ => false,
        }
    }
}

fn encode(binding: Option<Binding>) -> u32 {
    let Some(binding) = binding else { return 0 };
    let mut bits = BOUND;
    if let Some(part) = binding.part {
        bits |= HAS_PART | u32::from(part);
    }
    if let Some(key) = binding.key {
        bits |= HAS_KEY | (u32::from(key) << 8);
    }
    bits
}

fn decode(bits: u32) -> Option<Binding> {
    if bits & BOUND == 0 {
        return None;
    }
    // The masks keep each field to its byte.
    Some(Binding {
        part: (bits & HAS_PART != 0).then_some((bits & 0xFF) as u8),
        key: (bits & HAS_KEY != 0).then_some(((bits >> 8) & 0xFF) as u8),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Option<Binding>; 6] = [
        None,
        Some(Binding::ANY),
        Some(Binding::of_part(0)),
        Some(Binding::of_part(15)),
        Some(Binding::of_part(3).with_key(21)),
        Some(Binding::of_part(9).with_key(108)),
    ];

    #[test]
    fn the_cell_returns_what_it_was_given() {
        for binding in ALL {
            let cell = BindingCell::new(None);
            cell.set(binding);
            assert_eq!(cell.get(), binding, "{binding:?}");
            assert_eq!(BindingCell::new(binding).get(), binding);
        }
    }

    #[test]
    fn a_saved_binding_restores_in_a_fresh_cell() {
        let drum = PluginKind::PcmDrum;
        for binding in ALL
            .into_iter()
            .filter(|b| b.is_none_or(|b| drum.accepts(&b)))
        {
            let saved = BindingCell::new(binding).to_json();
            let fresh = BindingCell::new(None);
            assert!(fresh.restore_json(drum, &saved), "{saved}");
            assert_eq!(fresh.get(), binding, "{saved}");
        }
    }

    #[test]
    fn the_saved_form_is_readable_json() {
        assert_eq!(BindingCell::new(None).to_json(), "null");
        assert_eq!(
            BindingCell::new(Some(Binding::of_part(2))).to_json(),
            r#"{"part":2}"#
        );
        assert_eq!(
            BindingCell::new(Some(Binding::of_part(2).with_key(36))).to_json(),
            r#"{"part":2,"key":36}"#
        );
    }

    #[test]
    fn a_saved_binding_a_plugin_cannot_have_is_not_restored() {
        let cell = BindingCell::new(Some(Binding::of_part(1)));
        // Not JSON, a Part past the last, a key on a Tone plugin, a key
        // past 127, a Part on the Studio Set plugin.
        for (kind, text) in [
            (PluginKind::PcmSynth, "garbage"),
            (PluginKind::PcmSynth, r#"{"part":16}"#),
            (PluginKind::PcmSynth, r#"{"part":1,"key":36}"#),
            (PluginKind::PcmDrum, r#"{"part":1,"key":128}"#),
            (PluginKind::PcmDrum, r#"{"part":1,"key":20}"#),
            (PluginKind::SnDrum, r#"{"part":1,"key":89}"#),
            (PluginKind::StudioSet, "null"),
            (PluginKind::StudioSet, r#"{"part":1}"#),
            (PluginKind::SnDrum, "{}"),
        ] {
            assert!(!cell.restore_json(kind, text), "{kind:?} {text}");
            assert_eq!(cell.get(), Some(Binding::of_part(1)), "{kind:?} {text}");
        }
    }

    #[test]
    fn the_studio_set_plugin_restores_its_any_binding() {
        let cell = BindingCell::new(None);
        assert!(cell.restore_json(PluginKind::StudioSet, r#"{}"#));
        assert_eq!(cell.get(), Some(Binding::ANY));
    }
}
