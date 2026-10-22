//! WASM bindings for field relevance (`integral_core::relevance`): whether a
//! field is active given the state the web holds, and if not, why. The state
//! arguments are the objects core serialized for the editors (a tone, a PMT,
//! a part).

use integral_core::{pcm_drum, pcm_synth, relevance, sn_acoustic, sn_synth, state};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

use crate::{MixerField, PcmsPartialField, PcmsPmtField, SnaCommonField, SnsPartialField};

/// Whether a field (or a Scope) acts, and if not, why (mirrors
/// [`relevance::Relevance`]). Ordered from most to least live, so the most
/// live of several fields is their minimum.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Relevance {
    /// See [`relevance::Relevance::Active`].
    Active = 0,
    /// See [`relevance::Relevance::SwitchedOff`]: dimmed, still editable.
    SwitchedOff = 1,
    /// See [`relevance::Relevance::Irrelevant`]: dimmed and inert, or hidden.
    Irrelevant = 2,
}

mirror_core_enum!(Relevance <=> relevance::Relevance {
    Active,
    SwitchedOff,
    Irrelevant,
});

/// `value`, an object core serialized for the editors, read back.
pub(crate) fn from_js<T: for<'de> Deserialize<'de>>(value: JsValue) -> Result<T, JsError> {
    serde_wasm_bindgen::from_value(value).map_err(|e| JsError::new(&e.to_string()))
}

/// Relevance of SN Synth Partial `partial`'s (0-based) Scope in `common`
/// (an SN Synth Tone's Common).
#[wasm_bindgen]
pub fn sns_partial_scope_relevance(partial: usize, common: JsValue) -> Result<Relevance, JsError> {
    let common: sn_synth::SnSynthCommon = from_js(common)?;
    Ok(relevance::sns_partial_scope_relevance(&common, partial).into())
}

/// Relevance of `field` of an SN Synth Partial in state `partial`, within
/// its active Scope.
#[wasm_bindgen]
pub fn sns_partial_relevance(
    field: SnsPartialField,
    partial: JsValue,
) -> Result<Relevance, JsError> {
    let partial: sn_synth::SnSynthPartial = from_js(partial)?;
    Ok(relevance::sns_partial_relevance(field.into(), &partial).into())
}

/// Relevance of PCM Synth Partial `partial`'s (0-based) Scope in `pmt`.
#[wasm_bindgen]
pub fn pcms_partial_scope_relevance(partial: usize, pmt: JsValue) -> Result<Relevance, JsError> {
    let pmt: pcm_synth::PcmSynthPmt = from_js(pmt)?;
    Ok(relevance::pcms_partial_scope_relevance(&pmt, partial).into())
}

/// Relevance of WMT `wmt`'s (0-based) Scope on a PCM Drum key in state
/// `partial` (the key's Partial).
#[wasm_bindgen]
pub fn pcmd_wmt_scope_relevance(wmt: usize, partial: JsValue) -> Result<Relevance, JsError> {
    let partial: pcm_drum::PcmDrumPartial = from_js(partial)?;
    Ok(relevance::pcmd_wmt_scope_relevance(&partial, wmt).into())
}

/// Relevance of the pair of PCM Synth partials holding partial `partial`
/// (0-based) in `pmt`: switched off once both its partials are.
#[wasm_bindgen]
pub fn pcms_pair_scope_relevance(partial: usize, pmt: JsValue) -> Result<Relevance, JsError> {
    let pmt: pcm_synth::PcmSynthPmt = from_js(pmt)?;
    Ok(relevance::pcms_pair_scope_relevance(&pmt, partial).into())
}

/// Relevance of `field` of PCM Synth Partial `partial` (0-based, state
/// `partial_state`) under `pmt`, within its active Scope.
#[wasm_bindgen]
pub fn pcms_partial_relevance(
    field: PcmsPartialField,
    partial: usize,
    pmt: JsValue,
    partial_state: JsValue,
) -> Result<Relevance, JsError> {
    let pmt: pcm_synth::PcmSynthPmt = from_js(pmt)?;
    let partial_state: pcm_synth::PcmSynthPartial = from_js(partial_state)?;
    Ok(relevance::pcms_partial_relevance(field.into(), partial, &pmt, &partial_state).into())
}

/// Relevance of PMT `field` in `pmt`.
#[wasm_bindgen]
pub fn pcms_pmt_relevance(field: PcmsPmtField, pmt: JsValue) -> Result<Relevance, JsError> {
    let pmt: pcm_synth::PcmSynthPmt = from_js(pmt)?;
    Ok(relevance::pcms_pmt_relevance(field.into(), &pmt).into())
}

/// Relevance of SN Acoustic Common `field` in `common` (an SN Acoustic
/// Common).
#[wasm_bindgen]
pub fn sna_common_relevance(field: SnaCommonField, common: JsValue) -> Result<Relevance, JsError> {
    let common: sn_acoustic::SnAcousticCommon = from_js(common)?;
    Ok(relevance::sna_common_relevance(field.into(), &common).into())
}

/// Relevance of mixer `field` of part `part` (0-based, state `part_state`),
/// given `drum_comp_eq` (the Drum Comp+EQ state).
#[wasm_bindgen]
pub fn mixer_relevance(
    field: MixerField,
    part: usize,
    part_state: JsValue,
    drum_comp_eq: JsValue,
) -> Result<Relevance, JsError> {
    let part_state: state::PartState = from_js(part_state)?;
    let drum_comp_eq: state::DrumCompEqState = from_js(drum_comp_eq)?;
    Ok(relevance::mixer_relevance(field.into(), part, &part_state, &drum_comp_eq).into())
}

/// Relevance of mixer `field` while Motional Surround is `surround_enabled`
/// (it overrides the Output Assigns).
#[wasm_bindgen]
pub fn mixer_output_relevance(field: MixerField, surround_enabled: bool) -> Relevance {
    relevance::mixer_output_relevance(field.into(), surround_enabled).into()
}

/// Relevance of mixer `field` of a Drum Comp+EQ unit in state `unit` (a
/// `CompEqUnit`).
#[wasm_bindgen]
pub fn comp_eq_unit_relevance(field: MixerField, unit: JsValue) -> Result<Relevance, JsError> {
    let unit: state::CompEqUnit = from_js(unit)?;
    Ok(relevance::comp_eq_unit_relevance(field.into(), &unit).into())
}

/// Relevance of mixer EQ `field` on a strip whose EQ switch is `eq_switch`.
#[wasm_bindgen]
pub fn mixer_eq_relevance(field: MixerField, eq_switch: bool) -> Relevance {
    relevance::mixer_eq_relevance(field.into(), eq_switch).into()
}

/// Relevance of the Chorus or Reverb block in state `fx` (an `FxState`), and
/// so of its type's parameters.
#[wasm_bindgen]
pub fn fx_block_relevance(fx: JsValue) -> Result<Relevance, JsError> {
    let fx: state::FxState = from_js(fx)?;
    Ok(relevance::fx_block_relevance(&fx).into())
}

/// The raw value of Chorus or Reverb Studio Set common `field` in `fx` (an
/// `FxState`, the field's own block's); `undefined` for a field of neither
/// block. See [`integral_core::device::mixer_fx_value`].
#[wasm_bindgen]
pub fn mixer_fx_value(field: MixerField, fx: JsValue) -> Result<Option<u8>, JsError> {
    let fx: state::FxState = from_js(fx)?;
    Ok(integral_core::device::mixer_fx_value(field.into(), &fx))
}

/// Relevance of Studio Set common `field` of the Chorus or Reverb block in
/// state `fx` (an `FxState`, the field's own block's).
#[wasm_bindgen]
pub fn mixer_fx_relevance(field: MixerField, fx: JsValue) -> Result<Relevance, JsError> {
    let fx: state::FxState = from_js(fx)?;
    Ok(relevance::mixer_fx_relevance(field.into(), &fx).into())
}
