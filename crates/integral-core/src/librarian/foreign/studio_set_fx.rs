//! A Foreign patch's Chorus and Reverb as Temporary Studio Set settings, for
//! auditioning it. The types map by the Chorus/Reverb rows of
//! `docs/research/foreign-fx-map.md`, and the JV ones by Roland's own JV → XV
//! conversion (`docs/research/map-check-roland-pairs.md`): the JV chorus is
//! XV CHORUS type 1, the JV reverb XV REVERB type 1. Studio Set slots follow
//! `docs/research/fx-param-layout.md`.

use super::fx_map::rate_20ths;
use crate::address::Address;
use crate::fx_params::{FxParamDef, chorus_params, reverb_params};
use crate::params::{self, CHORUS_PARAM_COUNT, REVERB_PARAM_COUNT, chorus, reverb};
use crate::state::parse::{decode_nib_params, encode_nib_param};
use crate::sysex;

/// Which Studio Set effect block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FxBlock {
    /// Chorus, `18 00 04 00`.
    Chorus,
    /// Reverb, `18 00 06 00`.
    Reverb,
}

/// Chorus or Reverb settings for the Temporary Studio Set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudioSetEffect {
    /// The block they go to.
    pub block: FxBlock,
    /// The INTEGRA-7 Chorus or Reverb Type.
    pub kind: u8,
    /// Chorus or Reverb Level.
    pub level: u8,
    /// Chorus Output Select; `None` for Reverb.
    output_select: Option<u8>,
    /// Parameter slot values; `None` leaves a slot the type hides as it is.
    params: Vec<Option<i32>>,
}

impl StudioSetEffect {
    /// Type `kind` of `block` at `level`, every parameter at its default.
    fn new(block: FxBlock, kind: u8, level: u8) -> Self {
        let mut params = vec![None; block.param_count()];
        for def in block.defs(kind) {
            params[usize::from(def.index)] = Some(def.default_value);
            if let Some(sync) = def.sync {
                params[usize::from(sync.switch_index)] = Some(sync.switch_default);
                params[usize::from(sync.note_index)] = Some(sync.note_default);
            }
        }
        Self {
            block,
            kind,
            level,
            output_select: None,
            params,
        }
    }

    /// Set parameter slot `index` to `value`, when the type's range holds
    /// it; otherwise the default stays.
    fn set(&mut self, index: u8, value: i32) {
        let fits = self
            .block
            .defs(self.kind)
            .iter()
            .any(|d| d.index == index && (d.min..=d.max).contains(&value));
        if fits {
            self.params[usize::from(index)] = Some(value);
        }
    }

    /// The DT1s that write these settings: Type and Level, Chorus Output
    /// Select, then each run of parameter slots. Output Assign is left as
    /// it is.
    pub fn dt1s(&self, device_id: u8) -> Vec<Vec<u8>> {
        let (address, type_at, param_base): (fn(u8) -> Address, u8, u8) = match self.block {
            FxBlock::Chorus => (params::chorus_address, chorus::TYPE, chorus::PARAM_BASE),
            FxBlock::Reverb => (params::reverb_address, reverb::TYPE, reverb::PARAM_BASE),
        };
        let mut out = vec![sysex::build_dt1(
            device_id,
            &address(type_at),
            &[self.kind, self.level],
        )];
        if let Some(select) = self.output_select {
            out.push(sysex::build_dt1(
                device_id,
                &address(chorus::OUTPUT_SELECT),
                &[select],
            ));
        }
        let mut slot = 0;
        while slot < self.params.len() {
            let run: Vec<i32> = self.params[slot..].iter().map_while(|v| *v).collect();
            if !run.is_empty() {
                let data: Vec<u8> = run.iter().flat_map(|&v| encode_nib_param(v)).collect();
                out.push(sysex::build_dt1(
                    device_id,
                    &address(param_base + 4 * slot as u8),
                    &data,
                ));
            }
            slot += run.len() + 1;
        }
        out
    }
}

impl FxBlock {
    fn param_count(self) -> usize {
        match self {
            Self::Chorus => CHORUS_PARAM_COUNT,
            Self::Reverb => REVERB_PARAM_COUNT,
        }
    }

    fn defs(self, kind: u8) -> &'static [FxParamDef] {
        match self {
            Self::Chorus => chorus_params(kind),
            Self::Reverb => reverb_params(kind),
        }
    }
}

/// XV CHORUS type 1 Parameters 1–7 (Rate, Depth, Pre-Delay, Feedback,
/// Filter Type, Cutoff Freq, Phase) onto INTEGRA-7 Chorus type 1.
fn chorus_type_1(level: u8, output_select: u8, xv: &[i32]) -> StudioSetEffect {
    let mut fx = StudioSetEffect::new(FxBlock::Chorus, 1, level);
    fx.output_select = Some(output_select).filter(|&o| o <= 2);
    // Integra slots of Parameters 1–7; Rate converts to 1/20 Hz.
    for (&value, slot) in xv.iter().zip([4, 6, 2, 8, 0, 1, 7]) {
        let value = if slot == 4 {
            rate_20ths(value)
        } else {
            Some(value)
        };
        if let Some(value) = value {
            fx.set(slot, value);
        }
    }
    fx
}

/// XV REVERB type 1 Parameters 1–2 (the JV Reverb Type, Time) onto an
/// INTEGRA-7 Room or Hall. DELAY and PAN-DLY have no INTEGRA-7 reverb.
fn reverb_type_1(level: u8, xv: &[i32]) -> Option<StudioSetEffect> {
    // shortcut: STAGE1/2 become Hall 1/2, the nearest size; settle by ear.
    let kind = match xv.first()? {
        0 => 1,
        1 => 2,
        2 | 4 => 3,
        3 | 5 => 4,
        _ => return None,
    };
    let mut fx = StudioSetEffect::new(FxBlock::Reverb, kind, level);
    // shortcut: the JV Time and HF Damp curves are unpublished, so Time
    // scales linearly onto 0.1–10 s and HF Damp keeps its default; settle by
    // ear (foreign-fx-map.md, Unresolved 3).
    if let Some(&time) = xv.get(1).filter(|t| (0..=127).contains(*t)) {
        fx.set(2, 1 + time * 99 / 127);
    }
    Some(fx)
}

/// A JV Patch Chorus (Level, Rate, Depth, Pre-Delay, Feedback, Output), as
/// Roland's JV → XV conversion does: CHORUS type 1, Rate and Pre-Delay
/// clamped to 125, filter OFF, Phase 180°.
pub(super) fn jv_chorus(chorus: [u8; 6]) -> StudioSetEffect {
    let [level, rate, depth, pre_delay, feedback, output] = chorus;
    let xv = [rate.min(125), depth, pre_delay.min(125), feedback, 0, 7, 90];
    chorus_type_1(level, output, &xv.map(i32::from))
}

/// A JV Patch Reverb (Type, Level, Time, …), as XV REVERB type 1.
pub(super) fn jv_reverb(kind: u8, level: u8, time: u8) -> Option<StudioSetEffect> {
    reverb_type_1(level, &[kind.into(), time.into()])
}

/// An XV Patch or Rhythm Chorus block (Type, Level, Output Assign, Output
/// Select, Parameters 1–12): the INTEGRA-7 Chorus layout, 8 Parameters
/// short.
pub(super) fn xv_chorus(block: &[u8]) -> Option<StudioSetEffect> {
    let at = |offset: u8| usize::from(offset);
    let (level, select) = (block[at(chorus::LEVEL)], block[at(chorus::OUTPUT_SELECT)]);
    let xv = decode_nib_params(&block[at(chorus::PARAM_BASE)..], 12);
    // shortcut: the XV DELAY and GM2 CHORUS layouts are unverified, so they
    // keep the INTEGRA-7 type's defaults; copy them once checked.
    match block[at(chorus::TYPE)] {
        0 => Some(StudioSetEffect::new(FxBlock::Chorus, 0, level)),
        1 => Some(chorus_type_1(level, select, &xv)),
        kind @ (2 | 3) => {
            let mut fx = StudioSetEffect::new(FxBlock::Chorus, kind, level);
            fx.output_select = Some(select).filter(|&o| o <= 2);
            Some(fx)
        }
        _ => None,
    }
}

/// An XV Patch or Rhythm Reverb block (Type, Level, Output Assign,
/// Parameters 1–20): the INTEGRA-7 Reverb layout, 4 Parameters short.
/// SRV ROOM/HALL/PLATE and GM2 REVERB keep the INTEGRA-7 type's defaults:
/// their parameters differ or are unverified.
pub(super) fn xv_reverb(block: &[u8]) -> Option<StudioSetEffect> {
    let at = |offset: u8| usize::from(offset);
    let level = block[at(reverb::LEVEL)];
    let kind = match block[at(reverb::TYPE)] {
        0 => 0,
        1 => {
            let xv = decode_nib_params(&block[at(reverb::PARAM_BASE)..], 2);
            return reverb_type_1(level, &xv);
        }
        2 => 1,
        3 => 3,
        4 => 5,
        5 => 6,
        _ => return None,
    };
    Some(StudioSetEffect::new(FxBlock::Reverb, kind, level))
}

/// The DT1s that apply each of `effects` to the Temporary Studio Set.
pub fn effects_dt1s(device_id: u8, effects: &[StudioSetEffect]) -> Vec<Vec<u8>> {
    effects.iter().flat_map(|fx| fx.dt1s(device_id)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_slots_are_left_as_they_are() {
        // XV GM2 REVERB → GM2 Reverb: Character at slot 1 and Time at slot
        // 4, around the two hidden slots.
        let mut block = vec![0; 83];
        block[..2].copy_from_slice(&[5, 64]);
        let dt1s = xv_reverb(&block).unwrap().dt1s(0x10);
        let at = |a: u8, data: &[u8]| sysex::build_dt1(0x10, &params::reverb_address(a), data);
        assert_eq!(
            dt1s,
            [
                at(0x00, &[6, 64]),
                at(0x03, &encode_nib_param(4)),
                at(0x0F, &encode_nib_param(64)),
            ]
        );
    }

    #[test]
    fn out_of_range_values_keep_the_default() {
        // Filter Type 3 and Phase 91 don't exist on the INTEGRA-7.
        let fx = chorus_type_1(127, 3, &[0, 0, 0, 0, 3, 0, 91]);
        assert_eq!(fx.output_select, None);
        assert_eq!((fx.params[0], fx.params[7]), (Some(2), Some(90)));
    }
}
