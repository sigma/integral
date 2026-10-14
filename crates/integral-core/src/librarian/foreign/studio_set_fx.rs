//! A Foreign patch's Chorus and Reverb as Temporary Studio Set settings, for
//! auditioning it. The types map by the Chorus/Reverb rows of
//! `docs/research/foreign-fx-map.md`, and the JV ones by Roland's own JV → XV
//! conversion (`docs/research/map-check-roland-pairs.md`): the JV chorus is
//! XV CHORUS type 1, the JV reverb XV REVERB type 1. Studio Set slots follow
//! `docs/research/fx-param-layout.md`.

use super::fx_map::rate_20ths;
use crate::address::Address;
use crate::fx_params::{FxParamDef, NOTE_NAMES, chorus_params, pre_delay_tenths, reverb_params};
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

    /// Chorus type `kind` at `level`, every parameter at its default, with
    /// Output Select `select` when the INTEGRA-7 has it.
    fn chorus(kind: u8, level: u8, select: u8) -> Self {
        Self {
            output_select: Some(select).filter(|&o| o <= 2),
            ..Self::new(FxBlock::Chorus, kind, level)
        }
    }

    /// Set the tempo-syncable time whose msec slot is `index` from an XV
    /// time: 0–1000 ms, then the notes of [`NOTE_NAMES`] (X1 note \*1).
    fn set_time(&mut self, index: u8, xv: i32) {
        let Some(def) = self.block.defs(self.kind).iter().find(|d| d.index == index) else {
            return;
        };
        let (Some(sync), Some(note)) = (def.sync, def.note_param()) else {
            return;
        };
        let notes = 1001..1001 + NOTE_NAMES.len() as i32;
        let (switch, def, value) = if notes.contains(&xv) {
            (1, &note, xv - notes.start)
        } else {
            (0, def, xv)
        };
        if (def.min..=def.max).contains(&value) {
            self.params[usize::from(sync.switch_index)] = Some(switch);
            self.params[usize::from(def.index)] = Some(value);
        }
    }

    /// Set Room/Hall/Plate Time (slot 3) from a 0–127 Time.
    fn set_reverb_time(&mut self, time: i32) {
        // shortcut: the JV and XV Time curves are unpublished, so Time scales
        // linearly onto 0.1–10 s; settle by ear (foreign-fx-map.md,
        // Unresolved 3).
        if (0..=127).contains(&time) {
            self.set(2, 1 + time * 99 / 127);
        }
    }

    /// The DT1s that write these settings: Type and Level, Chorus Output
    /// Select, each run of parameter slots, then the effect's Switch ON so
    /// a Studio Set with it OFF still plays it. Output Assign is left as
    /// it is.
    pub fn dt1s(&self, device_id: u8) -> Vec<Vec<u8>> {
        let (address, type_at, param_base, switch): (fn(u8) -> Address, u8, u8, Address) =
            match self.block {
                FxBlock::Chorus => (
                    params::chorus_address,
                    chorus::TYPE,
                    chorus::PARAM_BASE,
                    params::CHORUS_SWITCH,
                ),
                FxBlock::Reverb => (
                    params::reverb_address,
                    reverb::TYPE,
                    reverb::PARAM_BASE,
                    params::REVERB_SWITCH,
                ),
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
        out.push(sysex::build_dt1(device_id, &switch, &[1]));
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
    let mut fx = StudioSetEffect::chorus(1, level, output_select);
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
    if let Some(&time) = xv.get(1) {
        fx.set_reverb_time(time);
    }
    // shortcut: HF Damp keeps its default, since which way the INTEGRA-7's
    // 0–100 damps is unpublished; convert it once the device shows
    // (foreign-fx-map.md, Unresolved 3).
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
/// short. Parameter orders are the corpus's (foreign-fx-map.md, Chorus
/// and Reverb).
pub(super) fn xv_chorus(block: &[u8]) -> Option<StudioSetEffect> {
    let at = |offset: u8| usize::from(offset);
    let (level, select) = (block[at(chorus::LEVEL)], block[at(chorus::OUTPUT_SELECT)]);
    let xv = decode_nib_params(&block[at(chorus::PARAM_BASE)..], 12);
    let fx = match block[at(chorus::TYPE)] {
        0 => StudioSetEffect::new(FxBlock::Chorus, 0, level),
        1 => chorus_type_1(level, select, &xv),
        // DELAY: Delay L/R/C, Feedback, HF Damp, Level L/R/C.
        2 => {
            let mut fx = StudioSetEffect::chorus(2, level, select);
            for (&value, slot) in xv.iter().zip([1, 4, 7]) {
                fx.set_time(slot, value);
            }
            for (&value, slot) in xv[3..8].iter().zip(9..) {
                fx.set(slot, value);
            }
            fx
        }
        // GM2 CHORUS: Pre-LPF, Level, Feedback, Delay, Rate, Depth, Send
        // Level to Reverb, as on the INTEGRA-7.
        3 => {
            let mut fx = StudioSetEffect::chorus(3, level, select);
            for (&value, slot) in xv[..7].iter().zip(0..) {
                fx.set(slot, value);
            }
            fx
        }
        _ => return None,
    };
    Some(fx)
}

/// An XV Patch or Rhythm Reverb block (Type, Level, Output Assign,
/// Parameters 1–20): the INTEGRA-7 Reverb layout, 4 Parameters short.
/// Parameter orders are the corpus's (foreign-fx-map.md, Chorus and
/// Reverb).
pub(super) fn xv_reverb(block: &[u8]) -> Option<StudioSetEffect> {
    let at = |offset: u8| usize::from(offset);
    let level = block[at(reverb::LEVEL)];
    let xv = decode_nib_params(&block[at(reverb::PARAM_BASE)..], 10);
    let fx = match block[at(reverb::TYPE)] {
        0 => StudioSetEffect::new(FxBlock::Reverb, 0, level),
        1 => return reverb_type_1(level, &xv),
        // SRV ROOM/HALL/PLATE: Pre Delay, Time, Size, High Cut, Density,
        // Diffusion, LF Damp and Gain, HF Damp and Gain. Size and High Cut
        // have no INTEGRA-7 counterpart.
        // shortcut: the damps keep their defaults, since which way the
        // INTEGRA-7's 0–100 damps is unpublished; convert them once the
        // device shows.
        srv @ 2..=4 => {
            let kind = [1, 3, 5][usize::from(srv - 2)];
            let mut fx = StudioSetEffect::new(FxBlock::Reverb, kind, level);
            if (0..=125).contains(&xv[0]) {
                fx.set(1, (pre_delay_tenths(xv[0]) + 5) / 10);
            }
            fx.set_reverb_time(xv[1]);
            fx.set(3, xv[4]);
            fx.set(4, xv[5]);
            fx
        }
        // GM2 REVERB: Character, Pre-LPF, Level, Time, Delay Feedback.
        // Only Character and Time have an INTEGRA-7 slot.
        // shortcut: the order is GS's, which one corpus block at the inits
        // can't tell from Time before Level; recheck on a Patch that moves
        // them.
        5 => {
            let mut fx = StudioSetEffect::new(FxBlock::Reverb, 6, level);
            fx.set(0, xv[0]);
            fx.set(3, xv[3]);
            fx
        }
        _ => return None,
    };
    Some(fx)
}

/// The DT1s that apply each of `effects` to the Temporary Studio Set.
pub fn effects_dt1s(device_id: u8, effects: &[StudioSetEffect]) -> Vec<Vec<u8>> {
    effects.iter().flat_map(|fx| fx.dt1s(device_id)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An XV Chorus or Reverb block of `len` bytes: `head` (Type, Level, …),
    /// then Parameters from offset `base`.
    fn xv_block(len: usize, head: &[u8], base: u8, params: &[i32]) -> Vec<u8> {
        let mut block = vec![0; len];
        block[..head.len()].copy_from_slice(head);
        for (k, &v) in params.iter().enumerate() {
            let at = usize::from(base) + 4 * k;
            block[at..at + 4].copy_from_slice(&encode_nib_param(v));
        }
        block
    }

    fn nibs(values: &[i32]) -> Vec<u8> {
        values.iter().flat_map(|&v| encode_nib_param(v)).collect()
    }

    fn chorus_at(offset: u8, data: &[u8]) -> Vec<u8> {
        sysex::build_dt1(0x10, &params::chorus_address(offset), data)
    }

    fn reverb_at(offset: u8, data: &[u8]) -> Vec<u8> {
        sysex::build_dt1(0x10, &params::reverb_address(offset), data)
    }

    /// The DT1 that switches `address`'s effect on.
    fn on(address: Address) -> Vec<u8> {
        sysex::build_dt1(0x10, &address, &[1])
    }

    #[test]
    fn xv_delay_times_are_msec_or_notes() {
        // Delay L 1/8, R 250 ms, C 1/4, Feedback +40 %, HF Damp BYPASS,
        // Levels L/R/C.
        let xv = [1010, 250, 1013, 69, 17, 100, 110, 120];
        let block = xv_block(52, &[2, 90, 0, 1], chorus::PARAM_BASE, &xv);
        assert_eq!(
            xv_chorus(&block).unwrap().dt1s(0x10),
            [
                chorus_at(chorus::TYPE, &[2, 90]),
                chorus_at(chorus::OUTPUT_SELECT, &[1]),
                // Each delay is sync switch, msec, note; the unused one keeps
                // its default.
                chorus_at(
                    chorus::PARAM_BASE,
                    &nibs(&[1, 200, 9, 0, 250, 10, 1, 600, 12, 69, 17, 100, 110, 120])
                ),
                on(params::CHORUS_SWITCH),
            ]
        );
    }

    #[test]
    fn xv_gm2_chorus_copies() {
        let xv = [2, 100, 20, 90, 5, 30, 40];
        let block = xv_block(52, &[3, 80, 0, 2], chorus::PARAM_BASE, &xv);
        assert_eq!(
            xv_chorus(&block).unwrap().dt1s(0x10),
            [
                chorus_at(chorus::TYPE, &[3, 80]),
                chorus_at(chorus::OUTPUT_SELECT, &[2]),
                chorus_at(chorus::PARAM_BASE, &nibs(&xv)),
                on(params::CHORUS_SWITCH),
            ]
        );
    }

    #[test]
    fn xv_srv_reverbs_copy_what_has_a_counterpart() {
        // Pre Delay 20 ms, Time 127, Size 4, High Cut, Density 90,
        // Diffusion 80, LF and HF Damp and their gains.
        let xv = [70, 127, 3, 19, 90, 80, 10, 20, 2, 10];
        for (srv, kind) in [(2, 1), (3, 3), (4, 5)] {
            let block = xv_block(83, &[srv, 100, 0], reverb::PARAM_BASE, &xv);
            assert_eq!(
                xv_reverb(&block).unwrap().dt1s(0x10),
                [
                    reverb_at(reverb::TYPE, &[kind, 100]),
                    // From slot 2: the hidden slot 1 stays. Damps keep
                    // their defaults.
                    reverb_at(
                        reverb::PARAM_BASE + 4,
                        &nibs(&[20, 100, 90, 80, 50, 50, 127, 64])
                    ),
                    on(params::REVERB_SWITCH),
                ],
                "SRV type {srv}"
            );
        }
    }

    #[test]
    fn hidden_slots_are_left_as_they_are() {
        // XV GM2 REVERB (Character, Pre-LPF, Level, Time, Delay Feedback) →
        // GM2 Reverb: Character at slot 1 and Time at slot 4, around the
        // two hidden slots.
        let block = xv_block(83, &[5, 64, 0], reverb::PARAM_BASE, &[2, 3, 90, 100, 0]);
        assert_eq!(
            xv_reverb(&block).unwrap().dt1s(0x10),
            [
                reverb_at(reverb::TYPE, &[6, 64]),
                reverb_at(0x03, &encode_nib_param(2)),
                reverb_at(0x0F, &encode_nib_param(100)),
                on(params::REVERB_SWITCH),
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
