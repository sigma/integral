//! Routing page view showing part output assignments, FX routing, and Drum Comp+EQ.
//!
//! Mirrors the `RoutingPage` React component from `web/src/RoutingPage.tsx`.
//! Periodically reads from [`SharedState`] via a timer and updates a local
//! [`RoutingData`] model that drives the Vizia view tree.

use std::sync::Arc;
use std::time::Duration;

use nih_plug_vizia::vizia::prelude::*;

use crate::SharedState;

/// Number of mixer parts.
const NUM_PARTS: usize = 16;

/// Refresh interval for reading device state (milliseconds).
const REFRESH_INTERVAL_MS: u64 = 100;

/// Output assign labels for parts.
const OUTPUT_ASSIGN_NAMES: &[&str] = &["A", "B", "C", "D", "1", "2", "3", "4", "5", "6", "7", "8"];

/// Output assign labels for Comp+EQ units.
const COMP_EQ_OUTPUT_NAMES: &[&str] = &[
    "PART", "A", "B", "C", "D", "1", "2", "3", "4", "5", "6", "7", "8",
];

/// Chorus type names.
const CHORUS_TYPE_NAMES: &[&str] = &["OFF", "Chorus", "Delay", "GM2 Cho"];

/// Reverb type names.
const REVERB_TYPE_NAMES: &[&str] = &["OFF", "Room 1", "Room 2", "Hall 1", "Hall 2", "Plate", "GM2 Rev"];

/// Chorus output names.
const CHORUS_OUTPUT_NAMES: &[&str] = &["MAIN", "REV", "MAIN+REV"];

/// Reverb output names.
const REVERB_OUTPUT_NAMES: &[&str] = &["A", "B", "C", "D"];

// ---------------------------------------------------------------------------
// Tone type label helper
// ---------------------------------------------------------------------------

/// Return a short tone type label from the bank MSB.
fn tone_type_short(bank_msb: u8) -> &'static str {
    match bank_msb {
        87 => "PCM-S",
        89 => "SN-A",
        95 => "SN-S",
        86 => "PCM-D",
        88 => "SN-D",
        121 => "GM2",
        97 => "ExPCM",
        _ => "---",
    }
}

// ---------------------------------------------------------------------------
// Per-part routing data
// ---------------------------------------------------------------------------

/// Per-part routing snapshot.
#[derive(Debug, Clone, Data, Lens)]
pub struct PartRoutingData {
    /// Part index (0-15).
    pub index: usize,
    /// Tone type label.
    pub tone_type: String,
    /// Tone name.
    pub tone_name: String,
    /// Output assign (0-11).
    pub output_assign: u8,
    /// Chorus send level (0-127).
    pub chorus_send: u8,
    /// Reverb send level (0-127).
    pub reverb_send: u8,
}

impl Default for PartRoutingData {
    fn default() -> Self {
        Self {
            index: 0,
            tone_type: "---".to_string(),
            tone_name: String::new(),
            output_assign: 0,
            chorus_send: 0,
            reverb_send: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// FX routing data
// ---------------------------------------------------------------------------

/// FX routing snapshot for chorus or reverb.
#[derive(Debug, Clone, Default, Data, Lens)]
pub struct FxRoutingData {
    /// Effect type index.
    pub fx_type: u8,
    /// Output routing index.
    pub output: u8,
}

// ---------------------------------------------------------------------------
// Drum Comp+EQ data
// ---------------------------------------------------------------------------

/// Drum Comp+EQ snapshot.
#[derive(Debug, Clone, Data, Lens)]
pub struct DrumCompEqViewData {
    /// Whether Drum Comp+EQ is enabled.
    pub enabled: bool,
    /// Assigned part index (0-15).
    pub part: u8,
    /// Per-unit output assigns (6 units).
    pub output_assigns: Vec<u8>,
}

impl Default for DrumCompEqViewData {
    fn default() -> Self {
        Self {
            enabled: false,
            part: 9,
            output_assigns: vec![0; 6],
        }
    }
}

// ---------------------------------------------------------------------------
// RoutingData model
// ---------------------------------------------------------------------------

/// Model driving the routing page view tree.
#[derive(Lens)]
pub struct RoutingData {
    /// Per-part routing data.
    pub parts: Vec<PartRoutingData>,
    /// Chorus routing.
    pub chorus: FxRoutingData,
    /// Reverb routing.
    pub reverb: FxRoutingData,
    /// Drum Comp+EQ state.
    pub drum_comp_eq: DrumCompEqViewData,
    /// Whether surround is active.
    pub surround_active: bool,
    /// Shared state handle.
    #[lens(ignore)]
    shared: Arc<SharedState>,
}

/// Events handled by [`RoutingData`].
enum RoutingEvent {
    /// Refresh snapshot from device.
    Refresh,
}

impl Model for RoutingData {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|e, _| match e {
            RoutingEvent::Refresh => {
                self.refresh_from_device();
            }
        });
    }
}

impl RoutingData {
    /// Create a new RoutingData with initial state from the device.
    fn new(shared: Arc<SharedState>) -> Self {
        let mut data = Self {
            parts: (0..NUM_PARTS)
                .map(|i| PartRoutingData {
                    index: i,
                    ..Default::default()
                })
                .collect(),
            chorus: FxRoutingData::default(),
            reverb: FxRoutingData::default(),
            drum_comp_eq: DrumCompEqViewData::default(),
            surround_active: false,
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

        self.surround_active = mixer.surround.enabled;

        for (i, part) in mixer.parts.iter().enumerate() {
            if i < self.parts.len() {
                self.parts[i].tone_type = tone_type_short(part.tone_bank_msb).to_string();
                self.parts[i].tone_name = if part.tone_name.is_empty() {
                    "---".to_string()
                } else {
                    part.tone_name.clone()
                };
                self.parts[i].output_assign = part.output_assign;
                self.parts[i].chorus_send = part.chorus_send;
                self.parts[i].reverb_send = part.reverb_send;
            }
        }

        self.chorus = FxRoutingData {
            fx_type: mixer.chorus.fx_type,
            output: mixer.chorus.output,
        };
        self.reverb = FxRoutingData {
            fx_type: mixer.reverb.fx_type,
            output: mixer.reverb.output,
        };

        self.drum_comp_eq = DrumCompEqViewData {
            enabled: mixer.drum_comp_eq.enabled,
            part: mixer.drum_comp_eq.part,
            output_assigns: mixer.drum_comp_eq.output_assigns.to_vec(),
        };
    }
}

// ---------------------------------------------------------------------------
// Helper: lookup name from index in a static slice
// ---------------------------------------------------------------------------

/// Look up a name from a static slice, falling back to "---".
fn lookup_name<'a>(names: &'a [&'a str], idx: u8) -> &'a str {
    names.get(idx as usize).copied().unwrap_or("---")
}

// ---------------------------------------------------------------------------
// RoutingPage view
// ---------------------------------------------------------------------------

/// The full routing page view.
///
/// Three sections: Part Routing Grid, Effects Routing, and Drum Comp+EQ.
pub struct RoutingPage;

impl RoutingPage {
    /// Creates a new [`RoutingPage`] view.
    pub fn new(cx: &mut Context, shared: Arc<SharedState>) -> Handle<'_, Self> {
        Self.build(cx, move |cx| {
            // Build the model.
            RoutingData::new(shared).build(cx);

            // Set up a periodic refresh timer.
            let timer = cx.add_timer(
                Duration::from_millis(REFRESH_INTERVAL_MS),
                None,
                |cx, action| {
                    if let TimerAction::Tick(_) = action {
                        cx.emit(RoutingEvent::Refresh);
                    }
                },
            );
            cx.start_timer(timer);

            // --- Surround banner ---
            Binding::new(cx, RoutingData::surround_active, |cx, lens| {
                if lens.get(cx) {
                    Label::new(
                        cx,
                        "Motional Surround is active \u{2014} output routing is overridden.",
                    )
                    .class("routing-surround-banner");
                }
            });

            // --- Section 1: Part Routing Grid ---
            VStack::new(cx, |cx| {
                Label::new(cx, "Part Routing").class("routing-section-title");

                ScrollView::new(cx, 0.0, 0.0, true, false, |cx| {
                    HStack::new(cx, |cx| {
                        for i in 0..NUM_PARTS {
                            let part_lens =
                                RoutingData::parts.map(move |parts| parts[i].clone());
                            RoutingPartColumn::new(cx, i, part_lens);
                        }
                    })
                    .class("routing-grid");
                })
                .class("routing-grid-scroll");
            })
            .class("routing-section");

            // --- Section 2: Effects Routing ---
            VStack::new(cx, |cx| {
                Label::new(cx, "Effects Routing").class("routing-section-title");

                HStack::new(cx, |cx| {
                    // Chorus card
                    VStack::new(cx, |cx| {
                        Label::new(cx, "Chorus").class("fx-routing-label");
                        Binding::new(
                            cx,
                            RoutingData::chorus.map(|c| c.fx_type),
                            |cx, type_lens| {
                                let t = type_lens.get(cx);
                                Label::new(cx, lookup_name(CHORUS_TYPE_NAMES, t))
                                    .class("fx-routing-type");
                            },
                        );
                        Binding::new(
                            cx,
                            RoutingData::chorus.map(|c| c.output),
                            |cx, out_lens| {
                                let o = out_lens.get(cx);
                                let text = format!("Out: {}", lookup_name(CHORUS_OUTPUT_NAMES, o));
                                Label::new(cx, &text).class("fx-routing-output");
                            },
                        );
                    })
                    .class("fx-routing-card");

                    // Reverb card
                    VStack::new(cx, |cx| {
                        Label::new(cx, "Reverb").class("fx-routing-label");
                        Binding::new(
                            cx,
                            RoutingData::reverb.map(|r| r.fx_type),
                            |cx, type_lens| {
                                let t = type_lens.get(cx);
                                Label::new(cx, lookup_name(REVERB_TYPE_NAMES, t))
                                    .class("fx-routing-type");
                            },
                        );
                        Binding::new(
                            cx,
                            RoutingData::reverb.map(|r| r.output),
                            |cx, out_lens| {
                                let o = out_lens.get(cx);
                                let text = format!("Out: {}", lookup_name(REVERB_OUTPUT_NAMES, o));
                                Label::new(cx, &text).class("fx-routing-output");
                            },
                        );
                    })
                    .class("fx-routing-card");
                })
                .class("fx-routing-bar");
            })
            .class("routing-section");

            // --- Section 3: Drum Comp+EQ ---
            Binding::new(
                cx,
                RoutingData::drum_comp_eq.map(|d| d.enabled),
                |cx, enabled_lens| {
                    if enabled_lens.get(cx) {
                        VStack::new(cx, |cx| {
                            Binding::new(
                                cx,
                                RoutingData::drum_comp_eq.map(|d| d.part),
                                |cx, part_lens| {
                                    let p = part_lens.get(cx);
                                    let title = format!("Drum Comp+EQ (Part {})", p + 1);
                                    Label::new(cx, &title).class("routing-section-title");
                                },
                            );

                            HStack::new(cx, |cx| {
                                for unit in 0..6 {
                                    let assign_lens = RoutingData::drum_comp_eq.map(
                                        move |d| {
                                            d.output_assigns
                                                .get(unit)
                                                .copied()
                                                .unwrap_or(0)
                                        },
                                    );
                                    VStack::new(cx, |cx| {
                                        let label = format!("C+EQ{}", unit + 1);
                                        Label::new(cx, &label).class("comp-eq-label");
                                        Binding::new(cx, assign_lens, |cx, val_lens| {
                                            let val = val_lens.get(cx);
                                            let name = lookup_name(COMP_EQ_OUTPUT_NAMES, val);
                                            Label::new(cx, name).class("comp-eq-value");
                                        });
                                    })
                                    .class("comp-eq-unit");
                                }
                            })
                            .class("comp-eq-grid");
                        })
                        .class("routing-section");
                    }
                },
            );
        })
    }
}

impl View for RoutingPage {
    fn element(&self) -> Option<&'static str> {
        Some("routing-page")
    }
}

// ---------------------------------------------------------------------------
// RoutingPartColumn — a single part column in the routing grid
// ---------------------------------------------------------------------------

/// A single part column in the routing grid.
struct RoutingPartColumn;

impl RoutingPartColumn {
    /// Creates a new routing part column.
    fn new<L>(cx: &mut Context, index: usize, data_lens: L) -> Handle<'_, Self>
    where
        L: Lens<Target = PartRoutingData>,
    {
        Self.build(cx, move |cx| {
            // Part number
            let num_label = format!("{}", index + 1);
            Label::new(cx, &num_label).class("routing-part-num");

            // Tone type
            Binding::new(
                cx,
                data_lens.map(|d| d.tone_type.clone()),
                |cx, type_lens| {
                    let t = type_lens.get(cx);
                    Label::new(cx, &t).class("routing-part-tone-type");
                },
            );

            // Tone name
            Binding::new(
                cx,
                data_lens.map(|d| d.tone_name.clone()),
                |cx, name_lens| {
                    let n = name_lens.get(cx);
                    Label::new(cx, &n).class("routing-part-tone-name");
                },
            );

            // Output assign label (read-only display)
            Binding::new(
                cx,
                data_lens.map(|d| d.output_assign),
                |cx, assign_lens| {
                    let a = assign_lens.get(cx);
                    let name = lookup_name(OUTPUT_ASSIGN_NAMES, a);
                    Label::new(cx, name).class("routing-part-output");
                },
            );

            // FX1 send bar
            Binding::new(
                cx,
                data_lens.map(|d| d.chorus_send),
                |cx, send_lens| {
                    let send = send_lens.get(cx);
                    let pct = send as f32 / 127.0;
                    HStack::new(cx, |cx| {
                        Label::new(cx, "FX1").class("routing-send-label");
                        ZStack::new(cx, |cx| {
                            Element::new(cx).class("routing-send-bar-bg");
                            Element::new(cx)
                                .class("routing-send-bar-fill")
                                .class("routing-send-bar-chorus")
                                .width(Percentage(pct * 100.0));
                        })
                        .class("routing-send-bar-container");
                    })
                    .class("routing-send-row");
                },
            );

            // FX2 send bar
            Binding::new(
                cx,
                data_lens.map(|d| d.reverb_send),
                |cx, send_lens| {
                    let send = send_lens.get(cx);
                    let pct = send as f32 / 127.0;
                    HStack::new(cx, |cx| {
                        Label::new(cx, "FX2").class("routing-send-label");
                        ZStack::new(cx, |cx| {
                            Element::new(cx).class("routing-send-bar-bg");
                            Element::new(cx)
                                .class("routing-send-bar-fill")
                                .class("routing-send-bar-reverb")
                                .width(Percentage(pct * 100.0));
                        })
                        .class("routing-send-bar-container");
                    })
                    .class("routing-send-row");
                },
            );
        })
    }
}

impl View for RoutingPartColumn {
    fn element(&self) -> Option<&'static str> {
        Some("routing-part-column")
    }
}
