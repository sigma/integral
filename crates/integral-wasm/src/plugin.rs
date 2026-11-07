//! WASM bindings for the DAW plugins (`integral_core::host_params`): which
//! plugin the editor page runs in, which Bindings it accepts, and the host
//! parameters the page's own writes set, which the plugin turns into host
//! gestures.

use std::sync::OnceLock;

use integral_core::host_params::{self, Binding, EffectKind, ParamIndex};
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::{ToneType, WasmDeviceState};

/// The plugins of the family (mirrors [`host_params::PluginKind`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginKind {
    /// See [`host_params::PluginKind::StudioSet`].
    StudioSet,
    /// See [`host_params::PluginKind::SnSynth`].
    SnSynth,
    /// See [`host_params::PluginKind::SnAcoustic`].
    SnAcoustic,
    /// See [`host_params::PluginKind::PcmSynth`].
    PcmSynth,
    /// See [`host_params::PluginKind::SnDrum`].
    SnDrum,
    /// See [`host_params::PluginKind::PcmDrum`].
    PcmDrum,
}

mirror_core_enum!(PluginKind <=> host_params::PluginKind {
    StudioSet,
    SnSynth,
    SnAcoustic,
    PcmSynth,
    SnDrum,
    PcmDrum,
});

/// The plugin a name from the plugin's IPC (core's serialized form of
/// [`host_params::PluginKind`]) names, or `None` for another.
#[wasm_bindgen]
pub fn plugin_kind_from_wire(name: &str) -> Option<PluginKind> {
    serde_json::from_value::<host_params::PluginKind>(name.into())
        .ok()
        .map(Into::into)
}

/// The tone type a Part must hold for `kind` to act on it, or `None` for the
/// Studio Set plugin, which acts on any.
#[wasm_bindgen]
pub fn plugin_kind_tone_type(kind: PluginKind) -> Option<ToneType> {
    host_params::PluginKind::from(kind)
        .tone_type()
        .map(Into::into)
}

/// Whether a plugin of `kind` can be bound to `part` (0-based) and `key`
/// (both absent for the Studio Set plugin).
#[wasm_bindgen]
pub fn plugin_kind_accepts(kind: PluginKind, part: Option<u8>, key: Option<u8>) -> bool {
    host_params::PluginKind::from(kind).accepts(&Binding { part, key })
}

/// Every plugin's parameter lists, searchable by address: built once, as it
/// is the size of the lists.
fn index() -> &'static ParamIndex {
    static INDEX: OnceLock<ParamIndex> = OnceLock::new();
    INDEX.get_or_init(ParamIndex::new)
}

/// A host parameter a write set: its ID, and its raw value.
#[derive(Serialize)]
struct HostWrite {
    id: String,
    value: i32,
}

#[wasm_bindgen(typescript_custom_section)]
const HOST_WRITE_TYPE: &str = r#"
/** A host parameter a write set: its ID, and the raw value written. */
export interface HostWrite {
  id: string;
  value: number;
}
"#;

#[wasm_bindgen]
impl WasmDeviceState {
    /// Start (or stop) logging the writes this state queues, for
    /// [`take_host_writes`](Self::take_host_writes).
    #[wasm_bindgen(js_name = recordWrites)]
    pub fn record_writes(&mut self, on: bool) {
        self.inner.record_writes(on);
    }

    /// Whether there are writes to take ([`take_host_writes`](Self::take_host_writes)).
    #[wasm_bindgen(js_name = hasWrites)]
    pub fn has_writes(&self) -> bool {
        self.inner.has_writes()
    }

    /// The host parameters of a `plugin` bound to (`part`, `key`) that the
    /// writes queued since the last call set, oldest first, as
    /// `{ id, value }` with the raw value. An effect type's parameter counts
    /// only while its type is the active one: Chorus and Reverb types are in
    /// the state, the bound Part's MFX type is the one this state wrote or
    /// was told ([`note_mfx_type`](integral_core::device::DeviceState::note_mfx_type)).
    #[wasm_bindgen(js_name = takeHostWrites, unchecked_return_type = "HostWrite[]")]
    pub fn take_host_writes(
        &mut self,
        plugin: PluginKind,
        part: Option<u8>,
        key: Option<u8>,
    ) -> Result<JsValue, JsError> {
        let (plugin, binding) = (host_params::PluginKind::from(plugin), Binding { part, key });
        let writes = self.inner.take_writes();
        let mfx_type = part.and_then(|part| self.inner.mfx_type(part));
        let state = self.inner.state();
        let active = |effect: EffectKind| match effect {
            EffectKind::Mfx => mfx_type,
            EffectKind::Chorus => Some(state.chorus.fx_type),
            EffectKind::Reverb => Some(state.reverb.fx_type),
        };
        let index = index();
        let set: Vec<HostWrite> = writes
            .iter()
            .flat_map(|w| index.writes_at(plugin, &binding, &w.address, &w.data, active))
            .map(|w| HostWrite {
                id: index.get(w.handle).id.clone(),
                value: w.value,
            })
            .collect();
        serde_wasm_bindgen::to_value(&set).map_err(|e| JsError::new(&e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_plugin_is_named_on_the_wire_by_core() {
        for core in host_params::PluginKind::ALL {
            let name = serde_json::to_value(core).expect("serializes");
            let kind = plugin_kind_from_wire(name.as_str().expect("a string"));
            assert_eq!(kind.map(host_params::PluginKind::from), Some(core));
        }
        assert_eq!(plugin_kind_from_wire("part"), None);
    }

    #[test]
    fn a_kind_accepts_what_core_accepts() {
        assert!(plugin_kind_accepts(PluginKind::StudioSet, None, None));
        assert!(!plugin_kind_accepts(PluginKind::StudioSet, Some(0), None));
        assert!(plugin_kind_accepts(PluginKind::SnSynth, Some(15), None));
        assert!(!plugin_kind_accepts(PluginKind::SnSynth, None, None));
        assert!(!plugin_kind_accepts(PluginKind::SnSynth, Some(0), Some(36)));
        assert!(plugin_kind_accepts(PluginKind::PcmDrum, Some(9), Some(36)));
        assert!(!plugin_kind_accepts(PluginKind::PcmDrum, Some(16), None));
        assert_eq!(
            plugin_kind_tone_type(PluginKind::SnDrum),
            Some(ToneType::SnDrum)
        );
        assert_eq!(plugin_kind_tone_type(PluginKind::StudioSet), None);
    }
}
