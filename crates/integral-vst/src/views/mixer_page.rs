//! Full mixer page view assembling channel strips, FX strips, and part selector.
//!
//! Mirrors the `MixerPage` React component from `web/src/MixerPage.tsx`.
//! Periodically reads from [`SharedState`] via a timer and updates a local
//! [`MixerData`] model that drives the Vizia view tree.

use std::sync::Arc;
use std::time::Duration;

use nih_plug_vizia::vizia::prelude::*;

use integral_core::device::DeviceState;
use integral_core::device_spec::INTEGRA7;
use integral_core::params::{chorus, reverb};

use crate::SharedState;

use super::tone_editor::bank_label;
use super::tone_selector::ToneSelectorEvent;
use super::top_bar::TopBarEvent;
use super::{
    ChannelStrip, ChannelStripData, StripEdit, StripVariant, SynthSelect, SynthSwitch,
    ToneSelector, TopBar, VolumeFader, route_chip,
};

/// Number of mixer parts.
const NUM_PARTS: usize = 16;

/// Number of Drum Comp+EQ units (from device spec).
const NUM_COMP_EQ_UNITS: usize = INTEGRA7.comp_eq_unit_count as usize;

/// Refresh interval for reading device state (milliseconds).
const REFRESH_INTERVAL_MS: u64 = 100;

// ---------------------------------------------------------------------------
// View-local data snapshot
// ---------------------------------------------------------------------------

/// Per-part data snapshot for the mixer view.
#[derive(Debug, Clone, Data, Lens)]
pub struct PartViewData {
    /// Part level / volume (0-127).
    pub level: u8,
    /// Part pan (0-127, 64 = centre).
    pub pan: u8,
    /// Whether the part is muted.
    pub muted: bool,
    /// Chorus send level (0-127).
    pub chorus_send: u8,
    /// Reverb send level (0-127).
    pub reverb_send: u8,
    /// Tone name read from the device.
    pub tone_name: String,
    /// MIDI receive channel (0-15).
    pub receive_channel: u8,
}

impl Default for PartViewData {
    fn default() -> Self {
        Self {
            level: 100,
            pan: 64,
            muted: false,
            chorus_send: 0,
            reverb_send: 0,
            tone_name: String::new(),
            receive_channel: 0,
        }
    }
}

/// FX block data snapshot for the mixer view.
#[derive(Debug, Clone, Data, Lens)]
pub struct FxViewData {
    /// Effect on/off.
    pub enabled: bool,
    /// Effect type index.
    pub fx_type: u8,
    /// Effect level (0-127).
    pub level: u8,
    /// Output routing index.
    pub output: u8,
}

impl Default for FxViewData {
    fn default() -> Self {
        Self {
            enabled: true,
            fx_type: 0,
            level: 0,
            output: 0,
        }
    }
}

/// Model driving the mixer page view tree.
#[derive(Lens)]
pub struct MixerData {
    /// Snapshot of all 16 parts as channel-strip data.
    pub strips: Vec<ChannelStripData>,
    /// Currently selected part index (0-15).
    pub selected_part: usize,
    /// System master level (0-127).
    pub master_level: u8,
    /// External input level (0-127).
    pub ext_level: u8,
    /// External input mute.
    pub ext_muted: bool,
    /// Solo part (0=OFF, 1-16).
    pub solo_part: u8,
    /// Chorus (FX1) snapshot.
    pub chorus: FxViewData,
    /// Reverb (FX2) snapshot.
    pub reverb: FxViewData,
    /// Selected part info text (e.g. "Part 1 : Piano").
    pub selected_part_info: String,
    /// Ext strip data.
    pub ext_strip: ChannelStripData,
    /// Master strip data.
    pub master_strip: ChannelStripData,
    /// Whether Drum Comp+EQ is on (shows the Comp+EQ strips).
    pub comp_eq_enabled: bool,
    /// Comp+EQ unit strips.
    pub comp_eq_strips: Vec<ChannelStripData>,
    /// Studio set name from the device.
    pub studio_set_name: String,
    /// Whether preview mode is active.
    pub preview_active: bool,
    /// Whether the tone selector modal is open.
    pub tone_selector_open: bool,
    /// Tone bank MSB of the selected part.
    pub selected_tone_msb: u8,
    /// Tone bank LSB of the selected part.
    pub selected_tone_lsb: u8,
    /// Tone program change of the selected part.
    pub selected_tone_pc: u8,
    /// Shared state handle for reading/writing device state.
    #[lens(ignore)]
    shared: Arc<SharedState>,
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// The mixer strip a [`StripEdit`] applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StripTarget {
    /// A part (0-15).
    Part(u8),
    /// The external input.
    Ext,
    /// The system master output.
    Master,
    /// A Drum Comp+EQ unit (0-5).
    CompEq(u8),
}

/// An effect block shown by an [`FxStrip`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FxBlock {
    /// FX1: Chorus.
    Chorus,
    /// FX2: Reverb.
    Reverb,
}

/// An edit made on an [`FxStrip`] control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FxEdit {
    /// Effect switched on or off.
    Switch(bool),
    /// Effect type picked (index into the device-spec type names).
    Type(u8),
    /// Effect level moved (0-127).
    Level(u8),
    /// Output route picked (index into the device-spec output names).
    Output(u8),
}

/// Events handled by [`MixerData`].
enum MixerEvent {
    /// Refresh the local snapshot from SharedState.
    Refresh,
    /// Select a part (0-15).
    SelectPart(usize),
    /// A channel strip control was edited.
    Strip(StripTarget, StripEdit),
    /// An FX strip control was edited.
    Fx(FxBlock, FxEdit),
}

impl Model for MixerData {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|e, _| match e {
            MixerEvent::Refresh => {
                self.refresh_from_device();
            }
            MixerEvent::SelectPart(idx) => {
                if *idx < NUM_PARTS {
                    self.selected_part = *idx;
                    self.update_selected_part_info();
                }
            }
            MixerEvent::Strip(target, edit) => {
                self.edit_device(|dev| apply_strip_edit(dev, *target, *edit));
            }
            MixerEvent::Fx(fx, edit) => {
                self.edit_device(|dev| apply_fx_edit(dev, *fx, *edit));
            }
        });

        event.map(|e, _| match e {
            TopBarEvent::TogglePreview => {
                self.preview_active = !self.preview_active;
            }
            TopBarEvent::OpenToneSelector => {
                self.tone_selector_open = true;
            }
        });

        event.map(|e, _| match e {
            ToneSelectorEvent::SelectTone { msb, lsb, pc } => {
                let part = self.selected_part as u8;
                let (msb, lsb, pc) = (*msb, *lsb, *pc);
                self.edit_device(|dev| dev.change_part_tone(part, msb, lsb, pc));
                self.tone_selector_open = false;
            }
            ToneSelectorEvent::Close => {
                self.tone_selector_open = false;
            }
        });
    }
}

impl MixerData {
    /// Create a new MixerData and start reading the mixer state from the
    /// device; the refresh timer shows the replies as they arrive.
    fn new(shared: Arc<SharedState>) -> Self {
        let default_strip = ChannelStripData {
            label: String::new(),
            volume: 100,
            pan: 64,
            fx1_send: 0.0,
            fx2_send: 0.0,
            muted: false,
            soloed: false,
            route: 0,
            route_names: &[],
            route_overridden: false,
        };

        let mut data = Self {
            strips: vec![
                ChannelStripData {
                    route_names: INTEGRA7.output_assigns,
                    ..default_strip.clone()
                };
                NUM_PARTS
            ],
            selected_part: 0,
            master_level: 100,
            ext_level: 100,
            ext_muted: false,
            solo_part: 0,
            chorus: FxViewData::default(),
            reverb: FxViewData::default(),
            selected_part_info: "Part 1".to_string(),
            ext_strip: ChannelStripData {
                label: "EX".to_string(),
                ..default_strip.clone()
            },
            master_strip: ChannelStripData {
                label: "Master".to_string(),
                ..default_strip.clone()
            },
            comp_eq_enabled: false,
            comp_eq_strips: (0..NUM_COMP_EQ_UNITS)
                .map(|i| ChannelStripData {
                    label: format!("C+E{}", i + 1),
                    route_names: INTEGRA7.comp_eq_output_assigns,
                    ..default_strip.clone()
                })
                .collect(),
            studio_set_name: String::new(),
            preview_active: false,
            tone_selector_open: false,
            selected_tone_msb: 0,
            selected_tone_lsb: 0,
            selected_tone_pc: 0,
            shared,
        };
        data.edit_device(DeviceState::load_mixer);
        data
    }

    /// Lock the device mutex and copy state into the view-local snapshot.
    fn refresh_from_device(&mut self) {
        let Ok(dev) = self.shared.device.lock() else {
            return;
        };
        let mixer = dev.state();

        self.master_level = mixer.master_level;
        self.ext_level = mixer.ext_level;
        self.ext_muted = mixer.ext_muted;
        self.solo_part = mixer.solo_part;
        self.studio_set_name = mixer.studio_set_name.clone();

        self.chorus = FxViewData {
            enabled: mixer.chorus.enabled,
            fx_type: mixer.chorus.fx_type,
            level: mixer.chorus.level,
            output: mixer.chorus.output,
        };
        self.reverb = FxViewData {
            enabled: mixer.reverb.enabled,
            fx_type: mixer.reverb.fx_type,
            level: mixer.reverb.level,
            output: mixer.reverb.output,
        };

        self.ext_strip.volume = mixer.ext_level;
        self.ext_strip.muted = mixer.ext_muted;

        self.master_strip.volume = mixer.master_level;

        let surround = mixer.surround.enabled;
        for (i, part) in mixer.parts.iter().enumerate() {
            if i < self.strips.len() {
                self.strips[i] = ChannelStripData {
                    label: format!("{}", i + 1),
                    volume: part.level,
                    pan: part.pan,
                    fx1_send: part.chorus_send as f32 / 127.0,
                    fx2_send: part.reverb_send as f32 / 127.0,
                    muted: part.muted,
                    soloed: mixer.solo_part == (i as u8 + 1),
                    route: part.output_assign,
                    route_names: INTEGRA7.output_assigns,
                    route_overridden: surround,
                };
            }
        }

        self.comp_eq_enabled = mixer.drum_comp_eq.enabled;
        for (strip, &assign) in self
            .comp_eq_strips
            .iter_mut()
            .zip(&mixer.drum_comp_eq.output_assigns)
        {
            strip.route = assign;
            strip.route_overridden = surround;
        }

        // Drop the lock before calling update.
        drop(dev);
        self.update_selected_part_info();
    }

    /// Apply `edit` to the device state, which queues its DT1, then refresh
    /// the snapshot so the view shows the new value.
    fn edit_device(&mut self, edit: impl FnOnce(&mut DeviceState)) {
        let Ok(mut dev) = self.shared.device.lock() else {
            return;
        };
        edit(&mut dev);
        drop(dev);
        self.refresh_from_device();
    }

    /// Update the selected part info text and tone bank data from current state.
    fn update_selected_part_info(&mut self) {
        let Ok(dev) = self.shared.device.lock() else {
            return;
        };
        let mixer = dev.state();
        let idx = self.selected_part;
        let part = &mixer.parts[idx];
        let tone = &part.tone_name;
        let type_label = bank_label(part.tone_bank_msb, part.tone_bank_lsb);
        self.selected_part_info = if tone.is_empty() {
            format!("Part {}", idx + 1)
        } else {
            format!("Part {} : {} : {}", idx + 1, type_label, tone)
        };
        self.selected_tone_msb = part.tone_bank_msb;
        self.selected_tone_lsb = part.tone_bank_lsb;
        self.selected_tone_pc = part.tone_pc;
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Write a channel strip edit through the matching [`DeviceState`] accessor.
fn apply_strip_edit(dev: &mut DeviceState, target: StripTarget, edit: StripEdit) {
    use StripEdit::*;
    use StripTarget::*;
    match (target, edit) {
        (Part(p), Volume(v)) => dev.set_part_level(p, v),
        (Part(p), Pan(v)) => dev.set_part_pan(p, v),
        (Part(p), Fx1Send(v)) => dev.set_part_chorus_send(p, v),
        (Part(p), Fx2Send(v)) => dev.set_part_reverb_send(p, v),
        (Part(p), Route(v)) => dev.set_part_output_assign(p, v),
        (Part(p), ToggleMute) => dev.toggle_part_mute(p),
        (Part(p), ToggleSolo) => dev.toggle_solo(p),
        (Ext, Volume(v)) => dev.set_ext_level(v),
        (Ext, ToggleMute) => dev.toggle_ext_mute(),
        (Master, Volume(v)) => dev.set_master_level(v),
        (CompEq(unit), Route(v)) => dev.set_drum_comp_eq_output_assign(unit, v),
        // The strip variant doesn't show this control.
        _ => {}
    }
}

/// Write an FX strip edit through the matching [`DeviceState`] accessor.
fn apply_fx_edit(dev: &mut DeviceState, fx: FxBlock, edit: FxEdit) {
    let state = dev.state();
    let enabled = match fx {
        FxBlock::Chorus => state.chorus.enabled,
        FxBlock::Reverb => state.reverb.enabled,
    };
    match (fx, edit) {
        (_, FxEdit::Switch(on)) if on == enabled => {}
        (FxBlock::Chorus, FxEdit::Switch(_)) => dev.toggle_chorus_switch(),
        (FxBlock::Chorus, FxEdit::Type(v)) => dev.set_chorus_param(chorus::TYPE, v),
        (FxBlock::Chorus, FxEdit::Level(v)) => dev.set_chorus_param(chorus::LEVEL, v),
        (FxBlock::Chorus, FxEdit::Output(v)) => dev.set_chorus_param(chorus::OUTPUT_SELECT, v),
        (FxBlock::Reverb, FxEdit::Switch(_)) => dev.toggle_reverb_switch(),
        (FxBlock::Reverb, FxEdit::Type(v)) => dev.set_reverb_param(reverb::TYPE, v),
        (FxBlock::Reverb, FxEdit::Level(v)) => dev.set_reverb_param(reverb::LEVEL, v),
        (FxBlock::Reverb, FxEdit::Output(v)) => dev.set_reverb_param(reverb::OUTPUT_ASSIGN, v),
    }
}

// ---------------------------------------------------------------------------
// MixerPage view
// ---------------------------------------------------------------------------

/// The full mixer page view.
///
/// Renders a part selector, selected-part info, 16 part strips, an ext strip,
/// FX strips, and a master strip in a horizontal scrollable layout.
pub struct MixerPage;

impl MixerPage {
    /// Creates a new [`MixerPage`] view.
    ///
    /// `shared` provides access to the device state via a mutex.
    pub fn new(cx: &mut Context, shared: Arc<SharedState>) -> Handle<'_, Self> {
        Self.build(cx, move |cx| {
            // Build the model.
            MixerData::new(shared).build(cx);

            // Set up a periodic refresh timer.
            let timer = cx.add_timer(
                Duration::from_millis(REFRESH_INTERVAL_MS),
                None,
                |cx, action| {
                    if let TimerAction::Tick(_) = action {
                        cx.emit(MixerEvent::Refresh);
                    }
                },
            );
            cx.start_timer(timer);

            // --- Top bar ---
            TopBar::new(cx);

            // --- Tone selector modal (shown when open) ---
            Binding::new(cx, MixerData::tone_selector_open, |cx, open_lens| {
                if open_lens.get(cx) {
                    Binding::new(cx, MixerData::selected_part, |cx, part_lens| {
                        let part_idx = part_lens.get(cx);
                        Binding::new(cx, MixerData::selected_tone_msb, move |cx, msb_lens| {
                            let msb = msb_lens.get(cx);
                            Binding::new(cx, MixerData::selected_tone_lsb, move |cx, lsb_lens| {
                                let lsb = lsb_lens.get(cx);
                                Binding::new(
                                    cx,
                                    MixerData::selected_tone_pc,
                                    move |cx, pc_lens| {
                                        let pc = pc_lens.get(cx);
                                        ToneSelector::new(cx, part_idx, msb, lsb, pc);
                                    },
                                );
                            });
                        });
                    });
                }
            });

            // --- Part selector row ---
            HStack::new(cx, |cx| {
                // Part number buttons (rebuilt on selection change).
                Binding::new(cx, MixerData::selected_part, |cx, sel_lens| {
                    let sel = sel_lens.get(cx);
                    HStack::new(cx, |cx| {
                        for i in 0..NUM_PARTS {
                            let label = format!("{}", i + 1);
                            Label::new(cx, &label)
                                .class("part-selector__btn")
                                .toggle_class("part-selector__btn--active", i == sel)
                                .cursor(CursorIcon::Hand)
                                .on_press(move |cx| {
                                    cx.emit(MixerEvent::SelectPart(i));
                                });
                        }
                    })
                    .class("part-selector");
                });
            })
            .class("mixer-header");

            // Selected part info label.
            Binding::new(cx, MixerData::selected_part_info, |cx, info_lens| {
                let text = info_lens.get(cx);
                Label::new(cx, &text).class("mixer-part-info");
            });

            // --- Mixer strips area ---
            ScrollView::new(cx, 0.0, 0.0, true, false, |cx| {
                HStack::new(cx, |cx| {
                    // 16 part strips.
                    for i in 0..NUM_PARTS {
                        let strip_lens = MixerData::strips.map(move |strips| strips[i].clone());
                        let target = StripTarget::Part(i as u8);
                        ChannelStrip::new(cx, StripVariant::Part, strip_lens, move |cx, edit| {
                            cx.emit(MixerEvent::Strip(target, edit));
                        });
                    }

                    // Ext strip.
                    ChannelStrip::new(cx, StripVariant::Ext, MixerData::ext_strip, |cx, edit| {
                        cx.emit(MixerEvent::Strip(StripTarget::Ext, edit));
                    });

                    // Comp+EQ unit strips, shown while Drum Comp+EQ is on.
                    Binding::new(cx, MixerData::comp_eq_enabled, |cx, enabled| {
                        if enabled.get(cx) {
                            for i in 0..NUM_COMP_EQ_UNITS {
                                let strip_lens =
                                    MixerData::comp_eq_strips.map(move |s| s[i].clone());
                                let target = StripTarget::CompEq(i as u8);
                                ChannelStrip::new(
                                    cx,
                                    StripVariant::CompEq,
                                    strip_lens,
                                    move |cx, edit| cx.emit(MixerEvent::Strip(target, edit)),
                                );
                            }
                        }
                    });

                    // FX1 (Chorus) strip.
                    FxStrip::new(
                        cx,
                        "FX1",
                        INTEGRA7.chorus_type_names,
                        INTEGRA7.chorus_output_names,
                        MixerData::chorus,
                        |cx, edit| cx.emit(MixerEvent::Fx(FxBlock::Chorus, edit)),
                    );

                    // FX2 (Reverb) strip.
                    FxStrip::new(
                        cx,
                        "FX2",
                        INTEGRA7.reverb_type_names,
                        INTEGRA7.output_assign_names,
                        MixerData::reverb,
                        |cx, edit| cx.emit(MixerEvent::Fx(FxBlock::Reverb, edit)),
                    );

                    // Master strip.
                    ChannelStrip::new(
                        cx,
                        StripVariant::Master,
                        MixerData::master_strip,
                        |cx, edit| cx.emit(MixerEvent::Strip(StripTarget::Master, edit)),
                    );
                })
                .class("mixer-strips");
            })
            .class("mixer-scroll");
        })
    }
}

impl View for MixerPage {
    fn element(&self) -> Option<&'static str> {
        Some("mixer-page")
    }
}

// ---------------------------------------------------------------------------
// FxStrip — lightweight FX block view
// ---------------------------------------------------------------------------

/// A simplified FX strip with label, on/off switch, type, level, and route chip.
pub struct FxStrip;

impl FxStrip {
    /// Creates a new [`FxStrip`] view.
    ///
    /// `label` is the display name (e.g. "FX1", "FX2"); `type_names` and
    /// `output_names` are the device-spec names for its type and output
    /// routing. `data_lens` must resolve to an [`FxViewData`]; `on_edit`
    /// receives every control edit.
    pub fn new<'a, L, F>(
        cx: &'a mut Context,
        label: &str,
        type_names: &'static [&'static str],
        output_names: &'static [&'static str],
        data_lens: L,
        on_edit: F,
    ) -> Handle<'a, Self>
    where
        L: Lens<Target = FxViewData>,
        F: Fn(&mut EventContext, FxEdit) + Copy + Send + Sync + 'static,
    {
        let label_owned = label.to_string();
        Self.build(cx, move |cx| {
            Label::new(cx, &label_owned).class("fx-strip__label");

            SynthSwitch::new(
                cx,
                data_lens.map(|d| usize::from(d.enabled)),
                &["OFF", "ON"],
                move |cx, idx| on_edit(cx, FxEdit::Switch(idx == 1)),
            );

            SynthSelect::new(
                cx,
                data_lens.map(|d| usize::from(d.fx_type)),
                type_names,
                move |cx, idx| on_edit(cx, FxEdit::Type(idx as u8)),
            );

            // FX outputs are never overridden by Motional Surround.
            route_chip(
                cx,
                output_names,
                data_lens.map(|d| d.output),
                data_lens.map(|_| false),
                move |cx, idx| on_edit(cx, FxEdit::Output(idx)),
            );

            VolumeFader::new(cx, data_lens.map(|d| d.level), move |cx, val| {
                on_edit(cx, FxEdit::Level(val));
            });
        })
    }
}

impl View for FxStrip {
    fn element(&self) -> Option<&'static str> {
        Some("fx-strip")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use integral_core::address::{Address, DataSize};
    use integral_core::params::{self, part};
    use integral_core::sysex;
    use std::collections::HashMap;
    use std::sync::Mutex;

    const DEV_ID: u8 = 0x10;

    /// A mixer model on a fresh device, with a clock for draining sends.
    struct Harness {
        data: MixerData,
        now_ms: f64,
    }

    impl Harness {
        /// Open the mixer on a device that answers every read with zeros.
        fn new() -> Self {
            Self::open(&HashMap::new())
        }

        /// Open the mixer on a device answering each read from `memory` (a
        /// block's bytes by start address, zeros elsewhere), until the load
        /// is over.
        fn open(memory: &HashMap<Address, Vec<u8>>) -> Self {
            let data = MixerData::new(Arc::new(SharedState {
                device: Mutex::new(DeviceState::new(DEV_ID)),
            }));
            let mut h = Self { data, now_ms: 0.0 };
            while h.has_pending() {
                h.now_ms += 100.0;
                let mut dev = h.data.shared.device.lock().unwrap();
                let Some(rq1) = dev.drain(h.now_ms) else {
                    continue;
                };
                let address = Address::new(rq1[7], rq1[8], rq1[9], rq1[10]);
                let size = DataSize::new(rq1[11], rq1[12], rq1[13], rq1[14]).value();
                let data =
                    (memory.get(&address).cloned()).unwrap_or_else(|| vec![0; size as usize]);
                dev.handle_dt1(&address, &data, h.now_ms);
            }
            h.data.refresh_from_device();
            h
        }

        fn strip(&mut self, target: StripTarget, edit: StripEdit) {
            self.data
                .edit_device(|dev| apply_strip_edit(dev, target, edit));
        }

        fn fx(&mut self, fx: FxBlock, edit: FxEdit) {
            self.data.edit_device(|dev| apply_fx_edit(dev, fx, edit));
        }

        fn has_pending(&self) -> bool {
            self.data.shared.device.lock().unwrap().has_pending()
        }

        /// The single DT1 queued on the device, as `(address, data)`.
        fn sent(&mut self) -> (Address, Vec<u8>) {
            // Step past the send throttle.
            self.now_ms += 1_000.0;
            let mut dev = self.data.shared.device.lock().unwrap();
            let msg = dev.drain(self.now_ms).expect("a DT1 was queued");
            assert!(!dev.has_pending(), "only one DT1 is queued");
            let dt1 = sysex::parse_dt1(&msg).unwrap();
            (dt1.address, dt1.data)
        }
    }

    #[test]
    fn opening_shows_the_device_state() {
        let mut dump = vec![0; params::PART_MIXER_SIZE.value() as usize];
        dump[0x27] = 127; // Chorus send
        let h = Harness::open(&HashMap::from([
            (params::SYSTEM_MASTER_LEVEL, vec![90]),
            (params::EXT_PART_LEVEL, vec![55]),
            (params::part_address(2, part::RECEIVE_CHANNEL), dump),
            (params::chorus_address(chorus::TYPE), vec![2, 100, 0, 1]),
            (params::REVERB_SWITCH, vec![1]),
            (params::DRUM_COMP_EQ_SWITCH, vec![1, 9, 1, 2, 3, 4, 5, 6]),
        ]));
        assert_eq!(h.data.master_strip.volume, 90);
        assert_eq!(h.data.ext_strip.volume, 55);
        assert_eq!(h.data.strips[2].fx1_send, 1.0);
        assert_eq!((h.data.chorus.fx_type, h.data.chorus.level), (2, 100));
        assert_eq!(h.data.chorus.output, 1);
        assert!(h.data.reverb.enabled && h.data.comp_eq_enabled);
        assert_eq!(h.data.comp_eq_strips[5].route, 6);
    }

    #[test]
    fn part_strip_edits_write_the_part_and_show_it() {
        let mut h = Harness::new();
        let target = StripTarget::Part(3);
        let cases = [
            (StripEdit::Volume(90), part::LEVEL, 90),
            (StripEdit::Pan(20), part::PAN, 20),
            (StripEdit::Fx1Send(30), part::CHORUS_SEND, 30),
            (StripEdit::Fx2Send(40), part::REVERB_SEND, 40),
            (StripEdit::Route(5), part::OUTPUT_ASSIGN, 5),
            (StripEdit::ToggleMute, part::MUTE, 1),
        ];
        for (edit, offset, value) in cases {
            h.strip(target, edit);
            assert_eq!(h.sent(), (params::part_address(3, offset), vec![value]));
        }
        h.strip(target, StripEdit::ToggleSolo);
        assert_eq!(h.sent(), (params::SOLO_PART, vec![4]));
        let strip = &h.data.strips[3];
        assert_eq!(strip.volume, 90);
        assert_eq!(strip.pan, 20);
        assert_eq!(strip.fx1_send, 30.0 / 127.0);
        assert_eq!(strip.fx2_send, 40.0 / 127.0);
        assert_eq!(strip.route, 5);
        assert!(strip.muted && strip.soloed);
    }

    #[test]
    fn ext_and_master_faders_write_their_levels() {
        let mut h = Harness::new();
        h.strip(StripTarget::Ext, StripEdit::Volume(50));
        assert_eq!(h.sent(), (params::EXT_PART_LEVEL, vec![50]));
        assert_eq!(h.data.ext_strip.volume, 50);
        h.strip(StripTarget::Ext, StripEdit::ToggleMute);
        assert_eq!(h.sent(), (params::EXT_PART_MUTE, vec![1]));
        assert!(h.data.ext_strip.muted);

        h.strip(StripTarget::Master, StripEdit::Volume(70));
        assert_eq!(h.sent(), (params::SYSTEM_MASTER_LEVEL, vec![70]));
        assert_eq!(h.data.master_strip.volume, 70);
    }

    #[test]
    fn comp_eq_route_writes_the_unit_output() {
        let mut h = Harness::new();
        h.strip(StripTarget::CompEq(1), StripEdit::Route(2));
        assert_eq!(h.sent(), (params::drum_comp_eq_output_assign(1), vec![2]));
        assert_eq!(h.data.comp_eq_strips[1].route, 2);
    }

    #[test]
    fn fx_edits_write_chorus_and_reverb() {
        let mut h = Harness::new();
        let chorus_cases = [
            (FxEdit::Type(2), chorus::TYPE, 2),
            (FxEdit::Level(80), chorus::LEVEL, 80),
            (FxEdit::Output(1), chorus::OUTPUT_SELECT, 1),
        ];
        for (edit, offset, value) in chorus_cases {
            h.fx(FxBlock::Chorus, edit);
            assert_eq!(h.sent(), (params::chorus_address(offset), vec![value]));
        }
        assert_eq!((h.data.chorus.fx_type, h.data.chorus.level), (2, 80));
        assert_eq!(h.data.chorus.output, 1);

        let reverb_cases = [
            (FxEdit::Type(4), reverb::TYPE, 4),
            (FxEdit::Level(60), reverb::LEVEL, 60),
            (FxEdit::Output(3), reverb::OUTPUT_ASSIGN, 3),
        ];
        for (edit, offset, value) in reverb_cases {
            h.fx(FxBlock::Reverb, edit);
            assert_eq!(h.sent(), (params::reverb_address(offset), vec![value]));
        }
        assert_eq!((h.data.reverb.fx_type, h.data.reverb.level), (4, 60));
        assert_eq!(h.data.reverb.output, 3);
    }

    #[test]
    fn fx_switch_writes_only_on_change() {
        let mut h = Harness::new();
        let on = h.data.chorus.enabled;
        h.fx(FxBlock::Chorus, FxEdit::Switch(on));
        assert!(!h.has_pending());

        h.fx(FxBlock::Chorus, FxEdit::Switch(!on));
        assert_eq!(h.sent(), (params::CHORUS_SWITCH, vec![u8::from(!on)]));
        assert_eq!(h.data.chorus.enabled, !on);

        let on = h.data.reverb.enabled;
        h.fx(FxBlock::Reverb, FxEdit::Switch(!on));
        assert_eq!(h.sent(), (params::REVERB_SWITCH, vec![u8::from(!on)]));
        assert_eq!(h.data.reverb.enabled, !on);
    }
}
