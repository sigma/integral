//! WASM bindings for field relevance (`integral_core::relevance`): whether a
//! field is active given the state the web holds. The state arguments are
//! the objects core serialized for the editors (a tone, a PMT, a part).

use integral_core::{pcm_drum, pcm_synth, relevance, sn_acoustic, sn_synth, state};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

use crate::{MixerField, PcmsPartialField, PcmsPmtField, SnaCommonField, SnsPartialField};

/// `value`, an object core serialized for the editors, read back.
pub(crate) fn from_js<T: for<'de> Deserialize<'de>>(value: JsValue) -> Result<T, JsError> {
    serde_wasm_bindgen::from_value(value).map_err(|e| JsError::new(&e.to_string()))
}

/// Whether SN Synth Partial `partial` (0-based) is switched on in `common`
/// (an SN Synth Tone's Common).
#[wasm_bindgen]
pub fn sns_partial_scope_is_active(partial: usize, common: JsValue) -> Result<bool, JsError> {
    let common: sn_synth::SnSynthCommon = from_js(common)?;
    Ok(relevance::sns_partial_scope_is_active(&common, partial))
}

/// Whether `field` of an SN Synth Partial in state `partial` is active,
/// within its active Scope.
#[wasm_bindgen]
pub fn sns_partial_is_active(field: SnsPartialField, partial: JsValue) -> Result<bool, JsError> {
    let partial: sn_synth::SnSynthPartial = from_js(partial)?;
    Ok(relevance::sns_partial_is_active(field.into(), &partial))
}

/// Whether PCM Synth Partial `partial` (0-based) is switched on in `pmt`.
#[wasm_bindgen]
pub fn pcms_partial_scope_is_active(partial: usize, pmt: JsValue) -> Result<bool, JsError> {
    let pmt: pcm_synth::PcmSynthPmt = from_js(pmt)?;
    Ok(relevance::pcms_partial_scope_is_active(&pmt, partial))
}

/// Whether WMT `wmt` (0-based) of a PCM Drum key in state `partial` (the
/// key's Partial) is switched on.
#[wasm_bindgen]
pub fn pcmd_wmt_scope_is_active(wmt: usize, partial: JsValue) -> Result<bool, JsError> {
    let partial: pcm_drum::PcmDrumPartial = from_js(partial)?;
    Ok(relevance::pcmd_wmt_scope_is_active(&partial, wmt))
}

/// Whether the pair of PCM Synth partials holding partial `partial`
/// (0-based) is active in `pmt`: either of its partials is switched on.
#[wasm_bindgen]
pub fn pcms_pair_scope_is_active(partial: usize, pmt: JsValue) -> Result<bool, JsError> {
    let pmt: pcm_synth::PcmSynthPmt = from_js(pmt)?;
    Ok(relevance::pcms_pair_scope_is_active(&pmt, partial))
}

/// Whether `field` of PCM Synth Partial `partial` (0-based, state
/// `partial_state`) is active under `pmt`, within its active Scope.
#[wasm_bindgen]
pub fn pcms_partial_is_active(
    field: PcmsPartialField,
    partial: usize,
    pmt: JsValue,
    partial_state: JsValue,
) -> Result<bool, JsError> {
    let pmt: pcm_synth::PcmSynthPmt = from_js(pmt)?;
    let partial_state: pcm_synth::PcmSynthPartial = from_js(partial_state)?;
    Ok(relevance::pcms_partial_is_active(
        field.into(),
        partial,
        &pmt,
        &partial_state,
    ))
}

/// Whether PMT `field` is active in `pmt`.
#[wasm_bindgen]
pub fn pcms_pmt_is_active(field: PcmsPmtField, pmt: JsValue) -> Result<bool, JsError> {
    let pmt: pcm_synth::PcmSynthPmt = from_js(pmt)?;
    Ok(relevance::pcms_pmt_is_active(field.into(), &pmt))
}

/// Whether SN Acoustic Common `field` is active in `common` (an SN Acoustic
/// Common).
#[wasm_bindgen]
pub fn sna_common_is_active(field: SnaCommonField, common: JsValue) -> Result<bool, JsError> {
    let common: sn_acoustic::SnAcousticCommon = from_js(common)?;
    Ok(relevance::sna_common_is_active(field.into(), &common))
}

/// Whether mixer `field` of part `part` (0-based, state
/// `part_state`) is active, given `drum_comp_eq` (the Drum Comp+EQ state).
#[wasm_bindgen]
pub fn mixer_is_active(
    field: MixerField,
    part: usize,
    part_state: JsValue,
    drum_comp_eq: JsValue,
) -> Result<bool, JsError> {
    let part_state: state::PartState = from_js(part_state)?;
    let drum_comp_eq: state::DrumCompEqState = from_js(drum_comp_eq)?;
    Ok(relevance::mixer_is_active(
        field.into(),
        part,
        &part_state,
        &drum_comp_eq,
    ))
}

/// Whether mixer `field` is active while Motional Surround is
/// `surround_enabled` (it overrides the Output Assigns).
#[wasm_bindgen]
pub fn mixer_output_is_active(field: MixerField, surround_enabled: bool) -> bool {
    relevance::mixer_output_is_active(field.into(), surround_enabled)
}

/// Whether mixer `field` of a Drum Comp+EQ unit in state `unit` (a
/// `CompEqUnit`) is active.
#[wasm_bindgen]
pub fn comp_eq_unit_is_active(field: MixerField, unit: JsValue) -> Result<bool, JsError> {
    let unit: state::CompEqUnit = from_js(unit)?;
    Ok(relevance::comp_eq_unit_is_active(field.into(), &unit))
}

/// Whether mixer EQ `field` is active on a strip whose EQ switch is
/// `eq_switch`.
#[wasm_bindgen]
pub fn mixer_eq_is_active(field: MixerField, eq_switch: bool) -> bool {
    relevance::mixer_eq_is_active(field.into(), eq_switch)
}

/// Whether the Chorus or Reverb block in state `fx` (an `FxState`) acts, so
/// its type's parameters are active.
#[wasm_bindgen]
pub fn fx_block_is_active(fx: JsValue) -> Result<bool, JsError> {
    let fx: state::FxState = from_js(fx)?;
    Ok(relevance::fx_block_is_active(&fx))
}

/// The raw value of Chorus or Reverb Studio Set common `field` in `fx` (an
/// `FxState`, the field's own block's); `undefined` for a field of neither
/// block. See [`integral_core::device::mixer_fx_value`].
#[wasm_bindgen]
pub fn mixer_fx_value(field: MixerField, fx: JsValue) -> Result<Option<u8>, JsError> {
    let fx: state::FxState = from_js(fx)?;
    Ok(integral_core::device::mixer_fx_value(field.into(), &fx))
}

/// Whether Studio Set common `field` of the Chorus or Reverb block in state
/// `fx` (an `FxState`, the field's own block's) is active.
#[wasm_bindgen]
pub fn mixer_fx_is_active(field: MixerField, fx: JsValue) -> Result<bool, JsError> {
    let fx: state::FxState = from_js(fx)?;
    Ok(relevance::mixer_fx_is_active(field.into(), &fx))
}
