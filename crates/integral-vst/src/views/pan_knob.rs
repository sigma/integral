//! A simple panning knob displaying L64..C..R63.
//!
//! Mirrors the `PanKnob` React component from `web/src/PanKnob.tsx`.
//! The knob operates in MIDI pan space (0-127) where 64 is center.
//! Displayed as L64 (hard left) through C (center) to R63 (hard right).

use nih_plug_vizia::vizia::prelude::*;

use super::MidiCallback;

/// When shift-dragging, mouse movement is scaled by this factor.
const GRANULAR_DRAG_MULTIPLIER: f32 = 0.1;

/// Events emitted by [`PanKnob`].
pub enum PanKnobEvent {
    /// The MIDI pan value changed (0 to 127).
    ValueChanged(u8),
}

/// Format a MIDI pan value (0-127) as L64..C..R63.
fn format_pan(value: u8) -> String {
    match value.cmp(&64) {
        std::cmp::Ordering::Equal => "C".to_string(),
        std::cmp::Ordering::Less => format!("L{}", 64 - value),
        std::cmp::Ordering::Greater => format!("R{}", value - 64),
    }
}

/// A panning knob control displaying L/C/R position.
///
/// # Usage
///
/// ```ignore
/// PanKnob::new(cx, data_lens.map(|d| d.pan), |cx, val| {
///     cx.emit(MyEvent::SetPan(val));
/// });
/// ```
#[derive(Lens)]
pub struct PanKnob {
    /// Current MIDI pan value (0-127, 64 = center).
    #[lens(ignore)]
    value: u8,
    /// Whether a drag is active.
    #[lens(ignore)]
    drag_active: bool,
    /// Y coordinate at drag start.
    #[lens(ignore)]
    drag_start_y: f32,
    /// Value at drag start.
    #[lens(ignore)]
    drag_start_value: u8,
    /// Callback for value changes.
    #[lens(ignore)]
    on_change: MidiCallback,
}

impl PanKnob {
    /// Creates a new [`PanKnob`] bound to a MIDI pan value (0-127) lens.
    pub fn new<L>(
        cx: &mut Context,
        value_lens: L,
        on_change: impl Fn(&mut EventContext, u8) + 'static,
    ) -> Handle<'_, Self>
    where
        L: Lens<Target = u8>,
    {
        let initial_value = value_lens.get(cx);

        Self {
            value: initial_value,
            drag_active: false,
            drag_start_y: 0.0,
            drag_start_value: initial_value,
            on_change: Some(Box::new(on_change)),
        }
        .build(cx, move |cx| {
            // Label
            Label::new(cx, "PAN")
                .class("pan-knob__label")
                .hoverable(false);

            // Knob body
            Element::new(cx).class("pan-knob__body").hoverable(false);

            // Value display
            Binding::new(cx, value_lens, |cx, val| {
                let v = val.get(cx);
                let display = format_pan(v);
                Label::new(cx, &display)
                    .class("pan-knob__value")
                    .hoverable(false);
            });
        })
    }

    /// Clamp and update the value, invoking the callback.
    fn update_value(&mut self, cx: &mut EventContext, new_value: i32) {
        self.value = new_value.clamp(0, 127) as u8;
        if let Some(on_change) = &self.on_change {
            on_change(cx, self.value);
        }
    }
}

impl View for PanKnob {
    fn element(&self) -> Option<&'static str> {
        Some("pan-knob")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| match window_event {
            WindowEvent::MouseDown(MouseButton::Left) => {
                self.drag_active = true;
                self.drag_start_y = cx.mouse().cursory;
                self.drag_start_value = self.value;
                cx.capture();
                cx.focus();
                cx.set_active(true);
                meta.consume();
            }
            WindowEvent::MouseUp(MouseButton::Left) => {
                if self.drag_active {
                    self.drag_active = false;
                    cx.release();
                    cx.set_active(false);
                    meta.consume();
                }
            }
            WindowEvent::MouseMove(_, y) => {
                if self.drag_active {
                    let dy = self.drag_start_y - y;
                    let sensitivity = if cx.modifiers().contains(Modifiers::SHIFT) {
                        GRANULAR_DRAG_MULTIPLIER * 127.0 / 200.0
                    } else {
                        127.0 / 200.0
                    };
                    let new_value = self.drag_start_value as f32 + dy * sensitivity;
                    self.update_value(cx, new_value.round() as i32);
                    meta.consume();
                }
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                // Reset to center
                self.update_value(cx, 64);
                meta.consume();
            }
            WindowEvent::MouseScroll(_, scroll_y) => {
                let delta = *scroll_y as i32;
                self.update_value(cx, self.value as i32 + delta);
                meta.consume();
            }
            _ => {}
        });
    }
}
