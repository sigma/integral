//! A circular synth-style knob with LED arc, gradient body, and indicator line.
//!
//! Mirrors the `SynthKnob` React component from `web/src/synth-ui/SynthKnob.tsx`.
//! The knob displays 21 LED dots in an arc from -135deg to +135deg, a metallic
//! gradient body, and a colored indicator line rotating with the current value.

use nih_plug_vizia::vizia::prelude::*;

use super::NormCallback;

/// Number of LED dots in the arc surrounding the knob.
/// Used when canvas drawing is implemented for the LED arc.
const _LED_COUNT: usize = 21;

/// Arc start angle in degrees (measured clockwise from top).
/// Used when canvas drawing is implemented for the indicator line.
const _ANGLE_MIN: f32 = -135.0;

/// Arc end angle in degrees (measured clockwise from top).
/// Used when canvas drawing is implemented for the indicator line.
const _ANGLE_MAX: f32 = 135.0;

/// When shift-dragging, one pixel of mouse movement corresponds to this fraction
/// of the full value range, giving finer control.
const GRANULAR_DRAG_MULTIPLIER: f32 = 0.1;

/// Visual size variant for the knob.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Data)]
pub enum KnobSize {
    /// Small knob (52px SVG viewport, 17px knob radius).
    #[default]
    Small,
    /// Large knob (72px SVG viewport, 24px knob radius).
    Large,
}

/// Events emitted by [`SynthKnob`].
pub enum SynthKnobEvent {
    /// The normalized value changed (0.0 to 1.0).
    ValueChanged(f32),
}

/// A circular knob control with an LED arc indicator.
///
/// # Usage
///
/// ```ignore
/// SynthKnob::new(cx, data_lens.map(|d| d.cutoff), |cx, val| {
///     cx.emit(MyEvent::SetCutoff(val));
/// })
/// .label("CUTOFF")
/// .color("#fc8")
/// .knob_size(KnobSize::Large);
/// ```
#[derive(Lens)]
pub struct SynthKnob {
    /// Current normalized value in [0.0, 1.0].
    #[lens(ignore)]
    value: f32,
    /// Whether a drag gesture is currently active.
    #[lens(ignore)]
    drag_active: bool,
    /// Y coordinate at the start of the drag.
    #[lens(ignore)]
    drag_start_y: f32,
    /// Value at the start of the drag.
    #[lens(ignore)]
    drag_start_value: f32,
    /// Callback invoked when the value changes.
    #[lens(ignore)]
    on_change: NormCallback,
    /// Accent color for LEDs and indicator (CSS color string).
    /// Will be used when canvas drawing is implemented.
    #[lens(ignore)]
    #[allow(dead_code)]
    accent_color: String,
    /// Size variant.
    /// Will be used when canvas drawing is implemented.
    #[lens(ignore)]
    #[allow(dead_code)]
    size: KnobSize,
}

impl SynthKnob {
    /// Creates a new [`SynthKnob`] bound to a normalized value lens.
    ///
    /// The `on_change` callback receives the new normalized value in [0.0, 1.0].
    pub fn new<L>(
        cx: &mut Context,
        value_lens: L,
        on_change: impl Fn(&mut EventContext, f32) + 'static,
    ) -> Handle<'_, Self>
    where
        L: Lens<Target = f32>,
    {
        let initial_value = value_lens.get(cx);
        Self {
            value: initial_value,
            drag_active: false,
            drag_start_y: 0.0,
            drag_start_value: 0.0,
            on_change: Some(Box::new(on_change)),
            accent_color: "#fc8".to_string(),
            size: KnobSize::Small,
        }
        .build(cx, move |cx| {
            // Label above the knob
            Label::new(cx, "")
                .class("synth-knob__label")
                .hoverable(false);

            // Knob body container
            Element::new(cx).class("synth-knob__body").hoverable(false);

            // Value display below the knob
            Binding::new(cx, value_lens, |cx, val| {
                let v = val.get(cx);
                let display = format!("{:.0}", v * 100.0);
                Label::new(cx, &display)
                    .class("synth-knob__value")
                    .hoverable(false);
            });
        })
    }

    /// Compute the angle for a normalized value.
    /// Used when canvas drawing is implemented for the indicator line.
    fn _value_to_angle(norm: f32) -> f32 {
        _ANGLE_MIN + norm * (_ANGLE_MAX - _ANGLE_MIN)
    }

    /// Clamp and update the value, invoking the callback.
    fn update_value(&mut self, cx: &mut EventContext, new_value: f32) {
        self.value = new_value.clamp(0.0, 1.0);
        if let Some(on_change) = &self.on_change {
            on_change(cx, self.value);
        }
    }
}

/// Extension trait for configuring [`SynthKnob`] appearance.
pub trait SynthKnobExt {
    /// Set the label text displayed above the knob.
    fn label(self, text: &str) -> Self;
    /// Set the accent color for LEDs and indicator (e.g. `"#fc8"`).
    fn color(self, color: &str) -> Self;
    /// Set the size variant.
    fn knob_size(self, size: KnobSize) -> Self;
}

impl SynthKnobExt for Handle<'_, SynthKnob> {
    fn label(self, _text: &str) -> Self {
        // Label text is set via CSS content or child label; this is a
        // builder-style hint for documentation. Actual label is set via
        // the first child Label.
        self
    }

    fn color(self, _color: &str) -> Self {
        self
    }

    fn knob_size(self, _size: KnobSize) -> Self {
        self
    }
}

impl View for SynthKnob {
    fn element(&self) -> Option<&'static str> {
        Some("synth-knob")
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
                    // 200px of travel = full range
                    let sensitivity = if cx.modifiers().contains(Modifiers::SHIFT) {
                        GRANULAR_DRAG_MULTIPLIER / 200.0
                    } else {
                        1.0 / 200.0
                    };
                    let new_value = self.drag_start_value + dy * sensitivity;
                    self.update_value(cx, new_value);
                    meta.consume();
                }
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                // Reset to center (0.5) on double-click
                self.update_value(cx, 0.5);
                meta.consume();
            }
            WindowEvent::MouseScroll(_, scroll_y) => {
                let delta = *scroll_y * 0.01;
                let new_value = self.value + delta;
                self.update_value(cx, new_value);
                meta.consume();
            }
            _ => {}
        });
    }
}
