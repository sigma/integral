//! WASM bindings for the Wave catalog: Wave Groups, their Waves, families
//! and Stereo pairs, all read from `integral_core::waves`.

use integral_core::waves::{self, WaveGroup};
use wasm_bindgen::prelude::*;

/// A Wave Group exposed to JS.
#[wasm_bindgen(js_name = WaveGroup)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WasmWaveGroup(WaveGroup);

#[wasm_bindgen(js_class = WaveGroup)]
impl WasmWaveGroup {
    /// The group an SN Synth OSC picks its PCM Wave from.
    #[wasm_bindgen(js_name = snSynth)]
    pub fn sn_synth() -> Self {
        Self(WaveGroup::SnSynth)
    }

    /// Identifier as `expansion load` spells it: `INT`, `SRX-01`…`SRX-12`,
    /// `SN-Synth`.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.0.id().to_string()
    }

    /// The Wave Group Type and ID to write for this group, or `undefined`
    /// for SN Synth, which has neither.
    pub fn params(&self) -> Option<Vec<u8>> {
        self.0.params().map(|(t, id)| vec![t, id])
    }

    /// Name of the SRX Expansion providing this group's Waves, or
    /// `undefined` for INT and SN Synth.
    #[wasm_bindgen(getter, js_name = expansionName)]
    pub fn expansion_name(&self) -> Option<String> {
        self.0.expansion().map(|e| e.name.to_string())
    }
}

/// The group selected by a Wave Group Type and ID, or `undefined` for
/// reserved values.
#[wasm_bindgen(js_name = waveGroupOf)]
pub fn wave_group_of(group_type: u8, group_id: u8) -> Option<WasmWaveGroup> {
    WaveGroup::from_params(group_type, group_id).map(WasmWaveGroup)
}

/// The Wave Groups a PCM Synth partial or PCM Drum WMT picks from: INT, then
/// SRX-01…SRX-12. SN Synth is not one of them.
#[wasm_bindgen(js_name = waveGroups)]
pub fn wave_groups() -> Vec<WasmWaveGroup> {
    WaveGroup::ALL
        .into_iter()
        .filter(|g| g.params().is_some())
        .map(WasmWaveGroup)
        .collect()
}

/// Every Wave of `group` as a JSON array of `{number, name, family}`, in
/// Wave Number order (OFF excluded).
#[wasm_bindgen(js_name = waveGroupWavesJson)]
pub fn wave_group_waves_json(group: &WasmWaveGroup) -> String {
    #[derive(serde::Serialize)]
    struct Entry {
        number: u16,
        name: &'static str,
        family: &'static str,
    }

    let entries: Vec<Entry> = waves::waves(group.0)
        .iter()
        .map(|w| Entry {
            number: w.number,
            name: w.name,
            family: w.family(),
        })
        .collect();
    serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string())
}

/// The name of Wave `number` in `group`: `"OFF"` for 0, `undefined` past
/// the end.
#[wasm_bindgen(js_name = waveName)]
pub fn wave_name(group: &WasmWaveGroup, number: u16) -> Option<String> {
    waves::name(group.0, number).map(str::to_string)
}

/// The distinct families of `group`, in catalog order.
#[wasm_bindgen(js_name = waveFamilies)]
pub fn wave_families(group: &WasmWaveGroup) -> Vec<String> {
    waves::families(group.0)
        .into_iter()
        .map(str::to_string)
        .collect()
}

/// The R Wave Number paired with L Wave `number` in `group`, or `undefined`.
#[wasm_bindgen(js_name = waveStereoPartner)]
pub fn wave_stereo_partner(group: &WasmWaveGroup, number: u16) -> Option<u16> {
    waves::stereo_partner(group.0, number)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_lookup_and_params() {
        let srx07 = wave_group_of(1, 7).unwrap();
        assert_eq!(srx07.id(), "SRX-07");
        assert_eq!(srx07.params(), Some(vec![1, 7]));
        assert_eq!(srx07.expansion_name().as_deref(), Some("SRX-07"));
        assert_eq!(wave_group_of(2, 0), None);
        assert_eq!(wave_group_of(0, 0).unwrap().expansion_name(), None);
        let sn = WasmWaveGroup::sn_synth();
        assert_eq!((sn.id().as_str(), sn.params()), ("SN-Synth", None));
    }

    #[test]
    fn groups_in_dropdown_order_without_sn_synth() {
        let ids: Vec<String> = wave_groups().iter().map(WasmWaveGroup::id).collect();
        assert_eq!(ids.len(), 13);
        assert_eq!(ids[0], "INT");
        assert_eq!(ids[1], "SRX-01");
        assert_eq!(ids[12], "SRX-12");
    }

    #[test]
    fn waves_json_names_families_and_partners() {
        let int = wave_group_of(0, 0).unwrap();
        let json: serde_json::Value = serde_json::from_str(&wave_group_waves_json(&int)).unwrap();
        assert_eq!(
            json[0],
            serde_json::json!({"number": 1, "name": "StGrand pA L", "family": "StGrand"})
        );
        assert_eq!(wave_name(&int, 1).as_deref(), Some("StGrand pA L"));
        assert_eq!(wave_name(&int, 0).as_deref(), Some("OFF"));
        assert_eq!(wave_name(&int, 1084), None);
        assert_eq!(
            wave_name(&WasmWaveGroup::sn_synth(), 1).as_deref(),
            Some("JP-8 Saw")
        );
        assert_eq!(wave_families(&int)[0], "StGrand");
        assert_eq!(wave_stereo_partner(&int, 1), Some(2));
        assert_eq!(wave_stereo_partner(&int, 2), None);
    }
}
