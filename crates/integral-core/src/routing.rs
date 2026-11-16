//! Effective routing: where each source's sound actually goes, from the
//! mixer state and the device's override rules.
//!
//! The rules are those of
//! [`docs/research/audio-output-routing.md`](../../../docs/research/audio-output-routing.md)
//! marked checked on the device:
//!
//! - Motional Surround ON overrides every Output Assign: the jacks carry the
//!   2-channel mix plus a fixed 5.1 layout
//!   ([`DeviceSpec::surround_jack_layout`](crate::device_spec::DeviceSpec::surround_jack_layout)),
//!   Chorus and Reverb are off, and every Comp+EQ unit returns to its Part's
//!   MFX.
//! - A Comp+EQ unit routed to an output skips its Part's MFX and EQ, still
//!   feeds the Part's sends, and lands on the unit's Output Assign. With the
//!   Comp/EQ Switch OFF the keys fall back to the Part, so no unit is in use.
//! - The Ext Part plays on A (MIX), and a stereo pair sums with the mono
//!   outputs on its jacks: A with 1 (A-L) and 2 (A-R).

use crate::device_spec::INTEGRA7;
use crate::params;
use crate::state::{MixerState, PartState};

/// Chorus Output Select: MAIN (MIDI Implementation, Studio Set Chorus).
const CHORUS_TO_MAIN: u8 = 0;
/// Chorus Output Select: REV, into the Reverb.
const CHORUS_TO_REVERB: u8 = 1;
/// Chorus Output Select: MAIN+REV.
const CHORUS_TO_BOTH: u8 = 2;
/// Drum Comp/EQ Output Assign: PART, back into the Part's MFX.
const COMP_EQ_TO_PART: u8 = 0;
/// Where the Ext Part plays while Motional Surround is off: A (MIX)
/// (checked on the device, [#467](https://github.com/sigma/integral/issues/467#issuecomment-6885678429)).
const EXT_PART_OUTPUT: Output = Output(0);

/// One jack of a pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The left jack (an odd mono output, 1, 3, 5, 7).
    Left,
    /// The right jack (an even mono output).
    Right,
}

/// An output destination, the index of its name in
/// [`DeviceSpec::output_assigns`](crate::device_spec::DeviceSpec::output_assigns):
/// a stereo pair (A–D), then a mono jack (1–8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Output(pub u8);

impl Output {
    /// The jack pair (index in
    /// [`DeviceSpec::output_assign_names`](crate::device_spec::DeviceSpec::output_assign_names))
    /// this output plays on, and which of its jacks: `None` for both (a
    /// stereo pair). `None` if out of range. The spec lists the pairs, then
    /// the mono outputs in jack order, left then right.
    pub fn jack(self) -> Option<(u8, Option<Side>)> {
        let pairs = INTEGRA7.output_assign_names.len() as u8;
        if usize::from(self.0) >= INTEGRA7.output_assigns.len() {
            return None;
        }
        Some(match self.0.checked_sub(pairs) {
            None => (self.0, None),
            Some(mono) => (
                mono / 2,
                Some(if mono % 2 == 0 {
                    Side::Left
                } else {
                    Side::Right
                }),
            ),
        })
    }

    /// Display name, e.g. "A" or "3".
    pub fn name(self) -> Option<&'static str> {
        INTEGRA7.output_assigns.get(usize::from(self.0)).copied()
    }
}

/// A source of sound: a row of the routing map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(
        tag = "kind",
        rename_all = "camelCase",
        rename_all_fields = "camelCase"
    )
)]
pub enum Source {
    /// A Part (0-based).
    Part {
        /// Part index (0-15).
        part: u8,
    },
    /// The Ext Part (the audio input).
    ExtPart,
    /// The Chorus.
    Chorus,
    /// The Reverb.
    Reverb,
    /// A Drum Comp+EQ unit of the Comp+EQ Part.
    CompEq {
        /// Unit index (0-5).
        unit: u8,
        /// The Part it belongs to (0-15).
        part: u8,
    },
}

/// The MFX on a source's path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(
        tag = "kind",
        rename_all = "camelCase",
        rename_all_fields = "camelCase"
    )
)]
pub enum MfxStage {
    /// The sound passes the Part's MFX.
    Applied {
        /// Its MFX Type (0 = THRU), `None` while unread.
        mfx_type: Option<u8>,
    },
    /// A Comp+EQ unit routed to an output skips the Part's MFX.
    Skipped,
}

/// The Part EQ on a source's path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum EqStage {
    /// The Part EQ is on.
    On,
    /// The Part EQ is off.
    Off,
    /// A Comp+EQ unit routed to an output skips the Part EQ.
    Skipped,
}

/// A send into the Chorus or the Reverb.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Send {
    /// Send level (0-127).
    pub level: u8,
    /// Whether the effect it feeds plays: switched on, a type other than
    /// OFF, and Motional Surround off.
    pub live: bool,
}

/// A source's Output Assign, as set on the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "kind", content = "output", rename_all = "camelCase")
)]
pub enum Assign {
    /// An output.
    Output(Output),
    /// A Comp+EQ unit's PART: back into the Part's MFX.
    Part,
}

/// Where a source's sound comes out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "kind", content = "output", rename_all = "camelCase")
)]
pub enum Landing {
    /// An output, by its Output Assign.
    Output(Output),
    /// The Motional Surround mix, on every jack.
    Surround,
}

/// One source's path: its stages, and where it lands.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Route {
    /// The source.
    pub source: Source,
    /// The Part MFX on its path (Parts and Comp+EQ units only).
    pub mfx: Option<MfxStage>,
    /// The Part EQ on its path (Parts and Comp+EQ units only).
    pub part_eq: Option<EqStage>,
    /// Its Chorus send (Parts, Comp+EQ units through their Part's, Ext Part).
    pub chorus_send: Option<Send>,
    /// Its Reverb send (Parts, Comp+EQ units through their Part's, Ext Part).
    pub reverb_send: Option<Send>,
    /// Whether it feeds the Reverb by its output (the Chorus on Output
    /// Select REV or MAIN+REV, while both play).
    pub feeds_reverb: bool,
    /// Its Output Assign as set, if it has one (the Ext Part hasn't).
    /// Motional Surround ON overrides it.
    pub assign: Option<Assign>,
    /// Where it comes out: empty when it doesn't (the Chorus or Reverb off,
    /// the Chorus only into the Reverb).
    pub lands_on: Vec<Landing>,
}

/// What a jack pair carries.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Jack {
    /// The pair (index in
    /// [`DeviceSpec::output_assign_names`](crate::device_spec::DeviceSpec::output_assign_names)).
    pub pair: u8,
    /// Its left and right channels under Motional Surround (from
    /// [`DeviceSpec::surround_jack_layout`](crate::device_spec::DeviceSpec::surround_jack_layout)),
    /// `None` while Surround is off.
    pub surround_channels: Option<[&'static str; 2]>,
    /// The sources on its left jack; several sum. Under Motional Surround,
    /// every source in the Surround mix.
    pub left: Vec<Source>,
    /// The sources on its right jack; several sum. Under Motional Surround,
    /// every source in the Surround mix.
    pub right: Vec<Source>,
}

/// The effective routing of a mixer state.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct EffectiveRouting {
    /// Whether Motional Surround is on, overriding every Output Assign.
    pub surround: bool,
    /// One route per source: the Parts, the Ext Part, the Chorus, the
    /// Reverb, then the Comp+EQ units in use: all six of the Comp+EQ Part,
    /// while the Switch is ON and the Part plays a drum kit (which keys
    /// each unit takes is in the kit, not the mixer state). A route says
    /// where a source goes, not whether it sounds: Level and Mute are left
    /// out.
    pub routes: Vec<Route>,
    /// One per jack pair, A (MIX) first.
    pub jacks: Vec<Jack>,
}

/// The effective routing of `state`.
pub fn effective_routing(state: &MixerState) -> EffectiveRouting {
    let surround = state.surround.enabled;
    let plays = |fx: &crate::state::FxState| fx.enabled && fx.fx_type != 0 && !surround;
    let (chorus_plays, reverb_plays) = (plays(&state.chorus), plays(&state.reverb));
    let sends = |chorus: u8, reverb: u8| {
        (
            Some(Send {
                level: chorus,
                live: chorus_plays,
            }),
            Some(Send {
                level: reverb,
                live: reverb_plays,
            }),
        )
    };
    let direct = |output: Output| {
        if surround {
            Landing::Surround
        } else {
            Landing::Output(output)
        }
    };

    let part_route = |source: Source, p: &PartState| {
        let (chorus_send, reverb_send) = sends(p.chorus_send, p.reverb_send);
        let output = Output(p.output_assign);
        Route {
            source,
            mfx: Some(MfxStage::Applied {
                mfx_type: p.mfx_type,
            }),
            part_eq: Some(if p.eq.enabled {
                EqStage::On
            } else {
                EqStage::Off
            }),
            chorus_send,
            reverb_send,
            feeds_reverb: false,
            assign: Some(Assign::Output(output)),
            lands_on: vec![direct(output)],
        }
    };

    let mut routes: Vec<Route> = (0u8..)
        .zip(&state.parts)
        .map(|(part, p)| part_route(Source::Part { part }, p))
        .collect();

    let (chorus_send, reverb_send) = sends(state.ext_chorus_send, state.ext_reverb_send);
    routes.push(Route {
        source: Source::ExtPart,
        mfx: None,
        part_eq: None,
        chorus_send,
        reverb_send,
        feeds_reverb: false,
        assign: None,
        lands_on: vec![direct(EXT_PART_OUTPUT)],
    });

    let chorus_select = state.chorus.output;
    let chorus_output = Output(state.chorus.output_assign);
    routes.push(Route {
        source: Source::Chorus,
        mfx: None,
        part_eq: None,
        chorus_send: None,
        reverb_send: None,
        feeds_reverb: chorus_plays
            && reverb_plays
            && matches!(chorus_select, CHORUS_TO_REVERB | CHORUS_TO_BOTH),
        assign: Some(Assign::Output(chorus_output)),
        lands_on: (chorus_plays && matches!(chorus_select, CHORUS_TO_MAIN | CHORUS_TO_BOTH))
            .then_some(Landing::Output(chorus_output))
            .into_iter()
            .collect(),
    });

    let reverb_output = Output(state.reverb.output);
    routes.push(Route {
        source: Source::Reverb,
        mfx: None,
        part_eq: None,
        chorus_send: None,
        reverb_send: None,
        feeds_reverb: false,
        assign: Some(Assign::Output(reverb_output)),
        lands_on: reverb_plays
            .then_some(Landing::Output(reverb_output))
            .into_iter()
            .collect(),
    });

    let drum_comp_eq = &state.drum_comp_eq;
    if let Some(p) = state.parts.get(usize::from(drum_comp_eq.part))
        && drum_comp_eq.enabled
        && params::is_drum_kit_bank(p.tone_bank_msb)
    {
        for (unit, &assign) in (0u8..).zip(&drum_comp_eq.output_assigns) {
            let source = Source::CompEq {
                unit,
                part: drum_comp_eq.part,
            };
            let mut route = part_route(source, p);
            if assign == COMP_EQ_TO_PART {
                route.assign = Some(Assign::Part);
            } else {
                let output = Output(assign - 1);
                route.assign = Some(Assign::Output(output));
                // Under Surround every unit returns to its Part's MFX.
                if !surround {
                    route.mfx = Some(MfxStage::Skipped);
                    route.part_eq = Some(EqStage::Skipped);
                    route.lands_on = vec![Landing::Output(output)];
                }
            }
            routes.push(route);
        }
    }

    let jacks = jacks(surround, &routes);
    EffectiveRouting {
        surround,
        routes,
        jacks,
    }
}

/// What each jack pair carries, from where `routes` land.
fn jacks(surround: bool, routes: &[Route]) -> Vec<Jack> {
    let mut jacks: Vec<Jack> = (0u8..)
        .zip(INTEGRA7.output_assign_names)
        .map(|(pair, _)| Jack {
            pair,
            surround_channels: surround
                .then(|| {
                    INTEGRA7
                        .surround_jack_layout
                        .get(usize::from(pair))
                        .copied()
                })
                .flatten(),
            left: Vec::new(),
            right: Vec::new(),
        })
        .collect();
    for route in routes {
        for landing in &route.lands_on {
            match *landing {
                Landing::Surround => {
                    for jack in &mut jacks {
                        jack.left.push(route.source);
                        jack.right.push(route.source);
                    }
                }
                Landing::Output(output) => {
                    let Some((pair, side)) = output.jack() else {
                        continue;
                    };
                    let Some(jack) = jacks.get_mut(usize::from(pair)) else {
                        continue;
                    };
                    if side != Some(Side::Right) {
                        jack.left.push(route.source);
                    }
                    if side != Some(Side::Left) {
                        jack.right.push(route.source);
                    }
                }
            }
        }
    }
    jacks
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: Output = Output(0);
    const B: Output = Output(1);
    const D: Output = Output(3);
    const MONO_1: Output = Output(4);
    const MONO_2: Output = Output(5);
    const MONO_8: Output = Output(11);

    /// A mixer state with every Part on `output` and no Chorus or Reverb.
    fn mixer(output: Output) -> MixerState {
        let mut state = MixerState::default();
        for p in &mut state.parts {
            p.output_assign = output.0;
        }
        state.chorus.fx_type = 0;
        state.reverb.fx_type = 0;
        state
    }

    fn route(routing: &EffectiveRouting, source: Source) -> &Route {
        (routing.routes.iter())
            .find(|r| r.source == source)
            .expect("a route")
    }

    fn part(part: u8) -> Source {
        Source::Part { part }
    }

    /// A state whose Part 10 plays an SN Drum Kit through the Comp+EQ, its
    /// units on `assigns` (raw: 0 = PART, then A–D, 1–8).
    fn comp_eq(assigns: [u8; 6]) -> MixerState {
        let mut state = mixer(B);
        let kit = &mut state.parts[9];
        kit.tone_bank_msb = 88;
        kit.mfx_type = Some(12);
        kit.chorus_send = 40;
        kit.reverb_send = 50;
        state.drum_comp_eq.enabled = true;
        state.drum_comp_eq.part = 9;
        state.drum_comp_eq.output_assigns = assigns;
        state
    }

    #[test]
    fn a_part_lands_on_its_output_assign_through_its_mfx_and_eq() {
        let mut state = mixer(B);
        state.parts[2].output_assign = MONO_8.0;
        state.parts[2].mfx_type = Some(0);
        state.parts[2].eq.enabled = false;
        let routing = effective_routing(&state);
        let r = route(&routing, part(2));
        assert_eq!(r.mfx, Some(MfxStage::Applied { mfx_type: Some(0) }), "THRU");
        assert_eq!(r.part_eq, Some(EqStage::Off));
        assert_eq!(r.assign, Some(Assign::Output(MONO_8)));
        assert_eq!(r.lands_on, [Landing::Output(MONO_8)]);
        assert_eq!(
            route(&routing, part(3)).mfx,
            Some(MfxStage::Applied { mfx_type: None }),
            "unread"
        );
        let d = &routing.jacks[3];
        assert_eq!(
            (d.left.as_slice(), d.right.as_slice()),
            (&[][..], &[part(2)][..])
        );
    }

    #[test]
    fn motional_surround_on_overrides_every_output_assign() {
        let mut state = comp_eq([2, 0, 0, 0, 0, 0]);
        state.parts[0].output_assign = D.0;
        state.chorus.fx_type = 1;
        state.reverb.fx_type = 1;
        state.surround.enabled = true;
        let routing = effective_routing(&state);
        assert!(routing.surround);

        let r = route(&routing, part(0));
        assert_eq!(r.assign, Some(Assign::Output(D)), "the Assign as set");
        assert_eq!(r.lands_on, [Landing::Surround]);
        assert_eq!(
            route(&routing, Source::ExtPart).lands_on,
            [Landing::Surround]
        );
        // Chorus and Reverb are off, and so are the sends into them.
        assert!(route(&routing, Source::Chorus).lands_on.is_empty());
        assert!(route(&routing, Source::Reverb).lands_on.is_empty());
        assert!(!r.chorus_send.unwrap().live && !r.reverb_send.unwrap().live);
        // A Comp+EQ unit returns to its Part's MFX.
        let unit = route(&routing, Source::CompEq { unit: 0, part: 9 });
        assert_eq!(unit.mfx, Some(MfxStage::Applied { mfx_type: Some(12) }));
        assert_eq!(unit.lands_on, [Landing::Surround]);

        // The jacks carry the 2-channel mix and the fixed 5.1 layout.
        let layout: Vec<_> = routing.jacks.iter().map(|j| j.surround_channels).collect();
        assert_eq!(
            layout,
            [
                Some(["2CH L", "2CH R"]),
                Some(["L", "R"]),
                Some(["C", "Ls"]),
                Some(["Rs", "LFE"]),
            ]
        );
        assert!(routing.jacks.iter().all(|j| j.left.contains(&part(0))));
    }

    #[test]
    fn a_comp_eq_unit_on_an_output_skips_the_parts_mfx_and_eq_but_feeds_its_sends() {
        // Unit 1 on C (raw 3), unit 2 on PART.
        let mut state = comp_eq([3, 0, 0, 0, 0, 0]);
        state.chorus.fx_type = 1;
        let routing = effective_routing(&state);

        let unit = route(&routing, Source::CompEq { unit: 0, part: 9 });
        assert_eq!(unit.mfx, Some(MfxStage::Skipped));
        assert_eq!(unit.part_eq, Some(EqStage::Skipped));
        assert_eq!(
            unit.lands_on,
            [Landing::Output(Output(2))],
            "the unit's, not B"
        );
        assert_eq!(
            unit.chorus_send,
            Some(Send {
                level: 40,
                live: true
            })
        );
        assert_eq!(
            unit.reverb_send.map(|s| (s.level, s.live)),
            Some((50, false))
        );

        // On PART, a unit goes through the Part's MFX and EQ to its output.
        let back = route(&routing, Source::CompEq { unit: 1, part: 9 });
        assert_eq!(back.assign, Some(Assign::Part));
        assert_eq!(back.mfx, Some(MfxStage::Applied { mfx_type: Some(12) }));
        assert_eq!(back.part_eq, Some(EqStage::On));
        assert_eq!(back.lands_on, [Landing::Output(B)]);
    }

    #[test]
    fn with_the_comp_eq_switch_off_the_part_keeps_its_own_output() {
        let mut state = comp_eq([3, 3, 3, 3, 3, 3]);
        state.drum_comp_eq.enabled = false;
        let routing = effective_routing(&state);
        assert!(
            !(routing.routes.iter()).any(|r| matches!(r.source, Source::CompEq { .. })),
            "no unit in use"
        );
        assert_eq!(route(&routing, part(9)).lands_on, [Landing::Output(B)]);
    }

    #[test]
    fn the_comp_eq_units_are_in_use_only_on_a_drum_kit() {
        let mut state = comp_eq([3; 6]);
        state.parts[9].tone_bank_msb = 87;
        let routing = effective_routing(&state);
        assert_eq!(routing.routes.len(), 16 + 3);
        state.parts[9].tone_bank_msb = 88;
        assert_eq!(effective_routing(&state).routes.len(), 16 + 3 + 6);
    }

    #[test]
    fn the_ext_part_and_a_with_1_and_2_sum_on_jack_a() {
        let mut state = mixer(B);
        state.parts[0].output_assign = A.0;
        state.parts[1].output_assign = MONO_1.0;
        state.parts[2].output_assign = MONO_2.0;
        let routing = effective_routing(&state);
        assert_eq!(
            route(&routing, Source::ExtPart).lands_on,
            [Landing::Output(A)]
        );
        let a = &routing.jacks[0];
        assert_eq!(a.left, [part(0), part(1), Source::ExtPart]);
        assert_eq!(a.right, [part(0), part(2), Source::ExtPart]);
        assert_eq!(a.surround_channels, None);
    }

    #[test]
    fn the_chorus_lands_on_its_output_assign_or_feeds_the_reverb() {
        let mut state = mixer(B);
        state.chorus.fx_type = 1;
        state.chorus.output_assign = D.0;
        state.reverb.fx_type = 1;
        state.reverb.output = B.0;
        for (select, lands, feeds) in [
            (CHORUS_TO_MAIN, true, false),
            (CHORUS_TO_REVERB, false, true),
            (CHORUS_TO_BOTH, true, true),
        ] {
            state.chorus.output = select;
            let routing = effective_routing(&state);
            let chorus = route(&routing, Source::Chorus);
            assert_eq!(!chorus.lands_on.is_empty(), lands, "select {select}");
            assert_eq!(chorus.feeds_reverb, feeds, "select {select}");
            assert_eq!(
                route(&routing, Source::Reverb).lands_on,
                [Landing::Output(B)]
            );
        }
        // Switched off, or of type OFF, it plays nowhere.
        state.chorus.enabled = false;
        assert!(
            route(&effective_routing(&state), Source::Chorus)
                .lands_on
                .is_empty()
        );
        state.chorus.enabled = true;
        state.chorus.fx_type = 0;
        assert!(
            route(&effective_routing(&state), Source::Chorus)
                .lands_on
                .is_empty()
        );
    }

    #[test]
    fn the_raw_values_match_the_spec_names() {
        let select = INTEGRA7.chorus_output_names;
        assert_eq!(select[usize::from(CHORUS_TO_MAIN)], "MAIN");
        assert_eq!(select[usize::from(CHORUS_TO_REVERB)], "REV");
        assert_eq!(select[usize::from(CHORUS_TO_BOTH)], "MAIN+REV");
        // A unit's Output Assign is PART, then a Part's outputs.
        let unit = INTEGRA7.comp_eq_output_assigns;
        assert_eq!(unit[usize::from(COMP_EQ_TO_PART)], "PART");
        assert_eq!(&unit[1..], INTEGRA7.output_assigns);
        assert_eq!(EXT_PART_OUTPUT.name(), Some("A"));
    }

    #[test]
    fn an_output_names_its_jack() {
        assert_eq!(A.jack(), Some((0, None)));
        assert_eq!(D.jack(), Some((3, None)));
        assert_eq!(MONO_1.jack(), Some((0, Some(Side::Left))));
        assert_eq!(MONO_2.jack(), Some((0, Some(Side::Right))));
        assert_eq!(MONO_8.jack(), Some((3, Some(Side::Right))));
        assert_eq!(Output(12).jack(), None);
        assert_eq!(MONO_8.name(), Some("8"));
    }
}
