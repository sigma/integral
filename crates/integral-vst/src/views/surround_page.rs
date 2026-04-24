//! Surround page view with global controls, per-part strip, and XY positioning pad.
//!
//! Mirrors the `SurroundPage` React component from `web/src/SurroundPage.tsx`.
//! Periodically reads from [`SharedState`] via a timer and updates a local
//! [`SurroundData`] model that drives the Vizia view tree.

use std::sync::Arc;
use std::time::Duration;

use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg;

use integral_core::device_spec::INTEGRA7;

use crate::SharedState;

/// Number of mixer parts (from device spec).
const NUM_PARTS: usize = INTEGRA7.part_count as usize;

/// Total entries in the part strip (parts + EXT).
const NUM_ENTRIES: usize = NUM_PARTS + 1;

/// Refresh interval for reading device state (milliseconds).
const REFRESH_INTERVAL_MS: u64 = 100;

// ---------------------------------------------------------------------------
// Per-part surround snapshot
// ---------------------------------------------------------------------------

/// Per-part surround data for the view.
#[derive(Debug, Clone, Data, Lens)]
pub struct SurroundPartViewData {
    /// Part index (0-15 for parts, 16 for EXT).
    pub index: usize,
    /// Display label ("1"-"16" or "EX").
    pub label: String,
    /// Tone name (parts only).
    pub tone_name: String,
    /// L-R position (0-127).
    pub lr: u8,
    /// F-B position (0-127).
    pub fb: u8,
    /// Width (0-32).
    pub width: u8,
    /// Ambience send level (0-127).
    pub ambience_send: u8,
}

impl Default for SurroundPartViewData {
    fn default() -> Self {
        Self {
            index: 0,
            label: String::new(),
            tone_name: String::new(),
            lr: 64,
            fb: 64,
            width: 16,
            ambience_send: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// SurroundData model
// ---------------------------------------------------------------------------

/// Model driving the surround page view tree.
#[derive(Lens)]
pub struct SurroundData {
    /// Whether surround is enabled.
    pub enabled: bool,
    /// Room type (0-3).
    pub room_type: u8,
    /// Room size (0-2).
    pub room_size: u8,
    /// Depth (0-100).
    pub depth: u8,
    /// Ambience level (0-127).
    pub ambience_level: u8,
    /// Ambience time (0-100).
    pub ambience_time: u8,
    /// Ambience density (0-100).
    pub ambience_density: u8,
    /// Ambience HF damp (0-100).
    pub ambience_hf_damp: u8,
    /// Per-part + EXT surround data (17 entries).
    pub parts: Vec<SurroundPartViewData>,
    /// Currently selected part index (0-16).
    pub selected_part: usize,
    /// Shared state handle.
    #[lens(ignore)]
    shared: Arc<SharedState>,
}

/// Events handled by [`SurroundData`].
pub enum SurroundEvent {
    /// Refresh snapshot from device.
    Refresh,
    /// Select a part (0-16).
    SelectPart(usize),
    /// Move the selected part on the XY pad.
    MovePart { index: usize, lr: u8, fb: u8 },
}

impl Model for SurroundData {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|e, _| match e {
            SurroundEvent::Refresh => {
                self.refresh_from_device();
            }
            SurroundEvent::SelectPart(idx) => {
                if *idx < NUM_ENTRIES {
                    self.selected_part = *idx;
                }
            }
            SurroundEvent::MovePart { index, lr, fb } => {
                let index = *index;
                let lr = *lr;
                let fb = *fb;
                if index < self.parts.len() {
                    self.parts[index].lr = lr;
                    self.parts[index].fb = fb;
                }
                // Write to device state.
                if let Ok(mut dev) = self.shared.device.lock() {
                    let state = dev.state_mut();
                    if index < NUM_PARTS {
                        state.surround.parts[index].lr = lr;
                        state.surround.parts[index].fb = fb;
                    } else if index == NUM_PARTS {
                        state.surround.ext.lr = lr;
                        state.surround.ext.fb = fb;
                    }
                }
            }
        });
    }
}

impl SurroundData {
    /// Create a new SurroundData with initial state from the device.
    fn new(shared: Arc<SharedState>) -> Self {
        let mut parts: Vec<SurroundPartViewData> = (0..NUM_PARTS)
            .map(|i| SurroundPartViewData {
                index: i,
                label: format!("{}", i + 1),
                ..Default::default()
            })
            .collect();
        parts.push(SurroundPartViewData {
            index: NUM_PARTS,
            label: "EX".to_string(),
            ..Default::default()
        });

        let mut data = Self {
            enabled: false,
            room_type: 0,
            room_size: 1,
            depth: 50,
            ambience_level: 64,
            ambience_time: 50,
            ambience_density: 50,
            ambience_hf_damp: 50,
            parts,
            selected_part: 0,
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
        let surround = &mixer.surround;

        self.enabled = surround.enabled;
        self.room_type = surround.room_type;
        self.room_size = surround.room_size;
        self.depth = surround.depth;
        self.ambience_level = surround.ambience_level;
        self.ambience_time = surround.ambience_time;
        self.ambience_density = surround.ambience_density;
        self.ambience_hf_damp = surround.ambience_hf_damp;

        for (i, sp) in surround.parts.iter().enumerate() {
            if i < NUM_PARTS {
                self.parts[i].lr = sp.lr;
                self.parts[i].fb = sp.fb;
                self.parts[i].width = sp.width;
                self.parts[i].ambience_send = sp.ambience_send;
                self.parts[i].tone_name = if mixer.parts[i].tone_name.is_empty() {
                    "\u{2014}".to_string()
                } else {
                    mixer.parts[i].tone_name.clone()
                };
            }
        }
        // EXT entry
        if let Some(ext) = self.parts.get_mut(NUM_PARTS) {
            ext.lr = surround.ext.lr;
            ext.fb = surround.ext.fb;
            ext.width = surround.ext.width;
            ext.ambience_send = surround.ext.ambience_send;
            ext.tone_name = "Ext".to_string();
        }
    }
}

// ---------------------------------------------------------------------------
// SurroundPage view
// ---------------------------------------------------------------------------

/// The full surround page view.
///
/// Renders a header with global controls, a part strip, and an XY positioning pad.
pub struct SurroundPage;

impl SurroundPage {
    /// Creates a new [`SurroundPage`] view.
    pub fn new(cx: &mut Context, shared: Arc<SharedState>) -> Handle<'_, Self> {
        Self.build(cx, move |cx| {
            // Build the model.
            SurroundData::new(shared).build(cx);

            // Set up a periodic refresh timer.
            let timer = cx.add_timer(
                Duration::from_millis(REFRESH_INTERVAL_MS),
                None,
                |cx, action| {
                    if let TimerAction::Tick(_) = action {
                        cx.emit(SurroundEvent::Refresh);
                    }
                },
            );
            cx.start_timer(timer);

            // --- Header: surround switch + room controls + global knobs ---
            HStack::new(cx, |cx| {
                // On/Off switch
                Binding::new(cx, SurroundData::enabled, |cx, enabled_lens| {
                    let on = enabled_lens.get(cx);
                    let text = if on { "ON" } else { "OFF" };
                    Label::new(cx, text)
                        .class("surround-switch")
                        .toggle_class("surround-switch--on", on)
                        .cursor(CursorIcon::Hand);
                });

                // Room type
                VStack::new(cx, |cx| {
                    Label::new(cx, "Room").class("surround-param-label");
                    Binding::new(cx, SurroundData::room_type, |cx, rt_lens| {
                        let rt = rt_lens.get(cx);
                        let name = INTEGRA7.surround_room_types.get(rt as usize).copied().unwrap_or("---");
                        Label::new(cx, name).class("surround-param-value");
                    });
                })
                .class("surround-param");

                // Room size
                VStack::new(cx, |cx| {
                    Label::new(cx, "Size").class("surround-param-label");
                    Binding::new(cx, SurroundData::room_size, |cx, rs_lens| {
                        let rs = rs_lens.get(cx);
                        let name = INTEGRA7.surround_room_sizes.get(rs as usize).copied().unwrap_or("---");
                        Label::new(cx, name).class("surround-param-value");
                    });
                })
                .class("surround-param");

                // Depth
                VStack::new(cx, |cx| {
                    Label::new(cx, "Depth").class("surround-param-label");
                    Binding::new(cx, SurroundData::depth, |cx, d_lens| {
                        let d = d_lens.get(cx);
                        Label::new(cx, &d.to_string()).class("surround-param-value");
                    });
                })
                .class("surround-param");

                // Amb Level
                VStack::new(cx, |cx| {
                    Label::new(cx, "Amb Lvl").class("surround-param-label");
                    Binding::new(cx, SurroundData::ambience_level, |cx, al_lens| {
                        let al = al_lens.get(cx);
                        Label::new(cx, &al.to_string()).class("surround-param-value");
                    });
                })
                .class("surround-param");

                // Amb Time
                VStack::new(cx, |cx| {
                    Label::new(cx, "Amb Time").class("surround-param-label");
                    Binding::new(cx, SurroundData::ambience_time, |cx, at_lens| {
                        let at = at_lens.get(cx);
                        Label::new(cx, &at.to_string()).class("surround-param-value");
                    });
                })
                .class("surround-param");

                // Density
                VStack::new(cx, |cx| {
                    Label::new(cx, "Density").class("surround-param-label");
                    Binding::new(cx, SurroundData::ambience_density, |cx, ad_lens| {
                        let ad = ad_lens.get(cx);
                        Label::new(cx, &ad.to_string()).class("surround-param-value");
                    });
                })
                .class("surround-param");

                // HF Damp
                VStack::new(cx, |cx| {
                    Label::new(cx, "HF Damp").class("surround-param-label");
                    Binding::new(cx, SurroundData::ambience_hf_damp, |cx, hf_lens| {
                        let hf = hf_lens.get(cx);
                        Label::new(cx, &hf.to_string()).class("surround-param-value");
                    });
                })
                .class("surround-param");
            })
            .class("surround-header");

            // --- Part strip ---
            ScrollView::new(cx, 0.0, 0.0, true, false, |cx| {
                HStack::new(cx, |cx| {
                    for i in 0..NUM_ENTRIES {
                        let part_lens =
                            SurroundData::parts.map(move |parts| parts[i].clone());
                        let selected_lens =
                            SurroundData::selected_part.map(move |sel| *sel == i);

                        VStack::new(cx, |cx| {
                            // Label
                            Binding::new(
                                cx,
                                part_lens.map(|p| p.label.clone()),
                                |cx, label_lens| {
                                    let lbl = label_lens.get(cx);
                                    Label::new(cx, &lbl).class("surround-part-label");
                                },
                            );

                            // Tone name
                            Binding::new(
                                cx,
                                part_lens.map(|p| p.tone_name.clone()),
                                |cx, name_lens| {
                                    let name = name_lens.get(cx);
                                    Label::new(cx, &name).class("surround-part-tone");
                                },
                            );

                            // Width value
                            Binding::new(
                                cx,
                                part_lens.map(|p| p.width),
                                |cx, w_lens| {
                                    let w = w_lens.get(cx);
                                    let text = format!("W:{}", w);
                                    Label::new(cx, &text).class("surround-part-knob-val");
                                },
                            );

                            // AMB send value
                            Binding::new(
                                cx,
                                part_lens.map(|p| p.ambience_send),
                                |cx, a_lens| {
                                    let a = a_lens.get(cx);
                                    let text = format!("AMB:{}", a);
                                    Label::new(cx, &text).class("surround-part-knob-val");
                                },
                            );
                        })
                        .class("surround-part-box")
                        .toggle_class(
                            "surround-part-box--selected",
                            selected_lens,
                        )
                        .cursor(CursorIcon::Hand)
                        .on_press(move |cx| {
                            cx.emit(SurroundEvent::SelectPart(i));
                        });
                    }
                })
                .class("surround-part-strip-inner");
            })
            .class("surround-part-strip");

            // --- XY Pad ---
            SurroundXYPad::new(cx);
        })
    }
}

impl View for SurroundPage {
    fn element(&self) -> Option<&'static str> {
        Some("surround-page")
    }
}

// ---------------------------------------------------------------------------
// SurroundXYPad — custom canvas view for 2D positioning
// ---------------------------------------------------------------------------

/// Custom XY pad view that draws part positions and supports dragging.
pub struct SurroundXYPad {
    /// Whether the pointer is currently dragging.
    dragging: bool,
}

impl SurroundXYPad {
    /// Creates a new [`SurroundXYPad`] view.
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self { dragging: false }.build(cx, |_cx| {})
    }

    /// Convert pointer position to (lr, fb) values.
    fn pointer_to_lr_fb(&self, cx: &mut EventContext, x: f32, y: f32) -> (u8, u8) {
        let bounds = cx.bounds();
        let nx = ((x - bounds.x) / bounds.w).clamp(0.0, 1.0);
        let ny = ((y - bounds.y) / bounds.h).clamp(0.0, 1.0);
        let lr = (nx * 127.0).round() as u8;
        // Y axis is inverted: top = front (high FB), bottom = back (low FB)
        let fb = ((1.0 - ny) * 127.0).round() as u8;
        (lr, fb)
    }
}

impl View for SurroundXYPad {
    fn element(&self) -> Option<&'static str> {
        Some("xy-pad")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|e, _meta| match *e {
            WindowEvent::MouseDown(MouseButton::Left) => {
                let mouse = cx.mouse();
                let (lr, fb) = self.pointer_to_lr_fb(cx, mouse.cursorx, mouse.cursory);
                // Get the selected part from the model.
                {
                    let sel = SurroundData::selected_part.get(cx);
                    cx.emit(SurroundEvent::MovePart {
                        index: sel,
                        lr,
                        fb,
                    });
                }
                self.dragging = true;
                cx.capture();
                cx.lock_cursor_icon();
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if self.dragging {
                    self.dragging = false;
                    cx.release();
                    cx.unlock_cursor_icon();
                }
            }
            WindowEvent::MouseMove(x, y) => {
                if self.dragging {
                    let (lr, fb) = self.pointer_to_lr_fb(cx, x, y);
                    let sel = SurroundData::selected_part.get(cx);
                    cx.emit(SurroundEvent::MovePart {
                        index: sel,
                        lr,
                        fb,
                    });
                }
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        if bounds.w < 1.0 || bounds.h < 1.0 {
            return;
        }

        // Background
        let mut bg_path = vg::Path::new();
        bg_path.rect(bounds.x, bounds.y, bounds.w, bounds.h);
        canvas.fill_path(
            &bg_path,
            &vg::Paint::color(vg::Color::rgba(10, 10, 24, 255)),
        );

        // Border
        let mut border_paint = vg::Paint::color(vg::Color::rgba(42, 42, 74, 255));
        border_paint.set_line_width(1.0);
        canvas.stroke_path(&bg_path, &border_paint);

        // Grid crosshairs (center lines)
        let cx_mid = bounds.x + bounds.w * 0.5;
        let cy_mid = bounds.y + bounds.h * 0.5;
        let mut grid_paint = vg::Paint::color(vg::Color::rgba(50, 50, 80, 128));
        grid_paint.set_line_width(1.0);

        let mut h_line = vg::Path::new();
        h_line.move_to(bounds.x, cy_mid);
        h_line.line_to(bounds.x + bounds.w, cy_mid);
        canvas.stroke_path(&h_line, &grid_paint);

        let mut v_line = vg::Path::new();
        v_line.move_to(cx_mid, bounds.y);
        v_line.line_to(cx_mid, bounds.y + bounds.h);
        canvas.stroke_path(&v_line, &grid_paint);

        // Axis labels
        let label_paint = vg::Paint::color(vg::Color::rgba(100, 100, 140, 200));
        let mut font_paint = label_paint;
        font_paint.set_font_size(10.0);

        // "Front" at top
        let _ = canvas.fill_text(cx_mid - 12.0, bounds.y + 12.0, "Front", &font_paint);
        // "Back" at bottom
        let _ = canvas.fill_text(
            cx_mid - 10.0,
            bounds.y + bounds.h - 4.0,
            "Back",
            &font_paint,
        );
        // "L" at left
        let _ = canvas.fill_text(bounds.x + 4.0, cy_mid - 4.0, "L", &font_paint);
        // "R" at right
        let _ = canvas.fill_text(bounds.x + bounds.w - 12.0, cy_mid - 4.0, "R", &font_paint);

        // Draw part dots
        let parts = SurroundData::parts.get(cx);
        let selected = SurroundData::selected_part.get(cx);

        for part in parts.iter() {
            let nx = part.lr as f32 / 127.0;
            // Invert Y: high FB = top
            let ny = 1.0 - (part.fb as f32 / 127.0);
            let px = bounds.x + nx * bounds.w;
            let py = bounds.y + ny * bounds.h;

            let is_selected = part.index == selected;
            let radius = if is_selected { 8.0 } else { 5.0 };

            let dot_color = if is_selected {
                vg::Color::rgba(74, 108, 247, 255)
            } else {
                vg::Color::rgba(100, 100, 160, 200)
            };

            let mut dot_path = vg::Path::new();
            dot_path.circle(px, py, radius);
            canvas.fill_path(&dot_path, &vg::Paint::color(dot_color));

            if is_selected {
                let mut ring_paint = vg::Paint::color(vg::Color::rgba(120, 160, 255, 180));
                ring_paint.set_line_width(2.0);
                let mut ring = vg::Path::new();
                ring.circle(px, py, radius + 2.0);
                canvas.stroke_path(&ring, &ring_paint);
            }

            // Label
            let mut lbl_paint = vg::Paint::color(vg::Color::rgba(220, 220, 240, 255));
            lbl_paint.set_font_size(8.0);
            let _ = canvas.fill_text(px + radius + 2.0, py - 2.0, &part.label, &lbl_paint);
        }
    }
}
