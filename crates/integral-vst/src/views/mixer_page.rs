//! Full mixer page view assembling channel strips, FX strips, and part selector.
//!
//! Mirrors the `MixerPage` React component from `web/src/MixerPage.tsx`.
//! Periodically reads from [`SharedState`] via a timer and updates a local
//! [`MixerData`] model that drives the Vizia view tree.

use std::sync::Arc;
use std::time::Duration;

use nih_plug_vizia::vizia::prelude::*;

use crate::SharedState;

use super::{ChannelStrip, ChannelStripData, StripVariant};

/// Number of mixer parts.
const NUM_PARTS: usize = 16;

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
}

impl Default for FxViewData {
    fn default() -> Self {
        Self {
            enabled: true,
            fx_type: 0,
            level: 0,
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
    /// Shared state handle for reading/writing device state.
    #[lens(ignore)]
    shared: Arc<SharedState>,
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Events handled by [`MixerData`].
enum MixerEvent {
    /// Refresh the local snapshot from SharedState.
    Refresh,
    /// Select a part (0-15).
    SelectPart(usize),
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
        });
    }
}

impl MixerData {
    /// Create a new MixerData with initial state from the device.
    fn new(shared: Arc<SharedState>) -> Self {
        let default_strip = ChannelStripData {
            label: String::new(),
            volume: 100,
            pan: 64,
            fx1_send: 0.0,
            fx2_send: 0.0,
            muted: false,
            soloed: false,
        };

        let mut data = Self {
            strips: vec![default_strip.clone(); NUM_PARTS],
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
                ..default_strip
            },
            shared,
        };
        data.refresh_from_device();
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

        self.chorus = FxViewData {
            enabled: mixer.chorus.enabled,
            fx_type: mixer.chorus.fx_type,
            level: mixer.chorus.level,
        };
        self.reverb = FxViewData {
            enabled: mixer.reverb.enabled,
            fx_type: mixer.reverb.fx_type,
            level: mixer.reverb.level,
        };

        self.ext_strip.volume = mixer.ext_level;
        self.ext_strip.muted = mixer.ext_muted;

        self.master_strip.volume = mixer.master_level;

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
                };
            }
        }

        // Drop the lock before calling update.
        drop(dev);
        self.update_selected_part_info();
    }

    /// Update the selected part info text from current state.
    fn update_selected_part_info(&mut self) {
        let Ok(dev) = self.shared.device.lock() else {
            return;
        };
        let mixer = dev.state();
        let idx = self.selected_part;
        let tone = &mixer.parts[idx].tone_name;
        self.selected_part_info = if tone.is_empty() {
            format!("Part {}", idx + 1)
        } else {
            format!("Part {} : {}", idx + 1, tone)
        };
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
                        let strip_lens =
                            MixerData::strips.map(move |strips| strips[i].clone());
                        ChannelStrip::new(cx, StripVariant::Part, strip_lens);
                    }

                    // Ext strip.
                    ChannelStrip::new(cx, StripVariant::Ext, MixerData::ext_strip);

                    // FX1 (Chorus) strip.
                    FxStrip::new(cx, "FX1", MixerData::chorus);

                    // FX2 (Reverb) strip.
                    FxStrip::new(cx, "FX2", MixerData::reverb);

                    // Master strip.
                    ChannelStrip::new(cx, StripVariant::Master, MixerData::master_strip);
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

/// A simplified FX strip showing label, on/off state, type, and level.
pub struct FxStrip;

impl FxStrip {
    /// Creates a new [`FxStrip`] view.
    ///
    /// `label` is the display name (e.g. "FX1", "FX2").
    /// `data_lens` must resolve to an [`FxViewData`].
    pub fn new<'a, L>(cx: &'a mut Context, label: &str, data_lens: L) -> Handle<'a, Self>
    where
        L: Lens<Target = FxViewData>,
    {
        let label_owned = label.to_string();
        Self.build(cx, move |cx| {
            // Label
            Label::new(cx, &label_owned).class("fx-strip__label");

            // On/Off indicator
            Binding::new(cx, data_lens.map(|d| d.enabled), |cx, enabled_lens| {
                let on = enabled_lens.get(cx);
                let text = if on { "ON" } else { "OFF" };
                Label::new(cx, text)
                    .class("fx-strip__switch")
                    .toggle_class("fx-strip__switch--on", on);
            });

            // Type display
            Binding::new(cx, data_lens.map(|d| d.fx_type), |cx, type_lens| {
                let t = type_lens.get(cx);
                let text = format!("Type {}", t);
                Label::new(cx, &text).class("fx-strip__type");
            });

            // Level display
            Binding::new(cx, data_lens.map(|d| d.level), |cx, level_lens| {
                let lvl = level_lens.get(cx);
                let text = format!("{}", lvl);
                Label::new(cx, &text).class("fx-strip__level");
            });
        })
    }
}

impl View for FxStrip {
    fn element(&self) -> Option<&'static str> {
        Some("fx-strip")
    }
}
