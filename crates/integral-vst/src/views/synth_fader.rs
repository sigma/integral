//! A vertical synth-style fader with tick marks and non-default highlight.
//!
//! Mirrors the `SynthFader` React component from `web/src/synth-ui/SynthFader.tsx`.
//! The fader shows a vertical track with a groove, a thumb that moves vertically,
//! and a fill region showing deviation from the default value.

use nih_plug_vizia::vizia::prelude::*;

use super::NormCallback;

/// Number of tick marks along the fader track.
const TICK_COUNT: usize = 11;

/// When shift-dragging, mouse movement is scaled by this factor.
const GRANULAR_DRAG_MULTIPLIER: f32 = 0.1;

/// Events emitted by [`SynthFader`].
pub enum SynthFaderEvent {
    /// The normalized value changed (0.0 to 1.0).
    ValueChanged(f32),
}

/// A vertical fader control with tick marks and a deviation-from-default fill.
///
/// # Usage
///
/// ```ignore
/// SynthFader::new(cx, data_lens.map(|d| d.attack), 0.0, |cx, val| {
///     cx.emit(MyEvent::SetAttack(val));
/// })
/// .compact(true);
/// ```
#[derive(Lens)]
pub struct SynthFader {
    /// Current normalized value in [0.0, 1.0].
    #[lens(ignore)]
    value: f32,
    /// Default normalized value for computing fill deviation.
    #[lens(ignore)]
    default_value: f32,
    /// Whether a drag gesture is active.
    #[lens(ignore)]
    drag_active: bool,
    /// Y coordinate at drag start.
    #[lens(ignore)]
    drag_start_y: f32,
    /// Value at drag start.
    #[lens(ignore)]
    drag_start_value: f32,
    /// Callback for value changes.
    #[lens(ignore)]
    on_change: NormCallback,
}

impl SynthFader {
    /// Creates a new [`SynthFader`].
    ///
    /// `default_value` is the normalized default (used for fill region computation).
    /// `on_change` receives the new normalized value in [0.0, 1.0].
    pub fn new<L>(
        cx: &mut Context,
        value_lens: L,
        default_value: f32,
        on_change: impl Fn(&mut EventContext, f32) + 'static,
    ) -> Handle<'_, Self>
    where
        L: Lens<Target = f32>,
    {
        let initial_value = value_lens.get(cx);
        Self {
            value: initial_value,
            default_value,
            drag_active: false,
            drag_start_y: 0.0,
            drag_start_value: 0.0,
            on_change: Some(Box::new(on_change)),
        }
        .build(cx, move |cx| {
            // Label
            Label::new(cx, "")
                .class("synth-fader__label")
                .hoverable(false);

            // Track area with ticks, groove, fill, and thumb
            VStack::new(cx, |cx| {
                // Ruler ticks
                for i in 0..TICK_COUNT {
                    let is_major = i == 0 || i == TICK_COUNT - 1 || i == TICK_COUNT / 2;
                    let class = if is_major {
                        "synth-fader__tick synth-fader__tick--major"
                    } else {
                        "synth-fader__tick"
                    };
                    Element::new(cx).class(class).hoverable(false);
                }

                // Track groove
                Element::new(cx)
                    .class("synth-fader__track")
                    .hoverable(false);

                // Fill (deviation from default)
                Binding::new(cx, value_lens, move |cx, val| {
                    let _v = val.get(cx);
                    Element::new(cx).class("synth-fader__fill").hoverable(false);
                });

                // Thumb
                Binding::new(cx, value_lens, move |cx, val| {
                    let v = val.get(cx);
                    let top_pct = (1.0 - v) * 100.0;
                    Element::new(cx)
                        .class("synth-fader__thumb")
                        .top(Percentage(top_pct))
                        .hoverable(false);
                });
            })
            .class("synth-fader__track-area");

            // Value display
            Binding::new(cx, value_lens, |cx, val| {
                let v = val.get(cx);
                let display = format!("{:.0}", v * 127.0);
                Label::new(cx, &display)
                    .class("synth-fader__value")
                    .hoverable(false);
            });
        })
    }

    /// Clamp and update the value, invoking the callback.
    fn update_value(&mut self, cx: &mut EventContext, new_value: f32) {
        self.value = new_value.clamp(0.0, 1.0);
        if let Some(on_change) = &self.on_change {
            on_change(cx, self.value);
        }
    }
}

/// Extension trait for configuring [`SynthFader`] appearance.
pub trait SynthFaderExt {
    /// Set the label text displayed above the fader.
    fn label(self, text: &str) -> Self;
    /// Use compact track height.
    fn compact(self, compact: bool) -> Self;
}

impl SynthFaderExt for Handle<'_, SynthFader> {
    fn label(self, _text: &str) -> Self {
        self
    }

    fn compact(self, compact: bool) -> Self {
        if compact {
            self.class("synth-fader--compact")
        } else {
            self
        }
    }
}

impl View for SynthFader {
    fn element(&self) -> Option<&'static str> {
        Some("synth-fader")
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
                self.update_value(cx, self.default_value);
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
