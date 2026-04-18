//! A tall mixer-style volume fader with ruler marks.
//!
//! Mirrors the `VolumeFader` React component from `web/src/VolumeFader.tsx`.
//! Features a 588px track with ruler marks at standard MIDI values
//! (127, 100, 80, 60, 40, 20, 0) and a "0dB" label at the 100 mark.

use nih_plug_vizia::vizia::prelude::*;

use super::MidiCallback;

/// Track height matching the React implementation.
const TRACK_HEIGHT: f32 = 588.0;

/// When shift-dragging, mouse movement is scaled by this factor.
const GRANULAR_DRAG_MULTIPLIER: f32 = 0.1;

/// MIDI value ruler marks displayed along the track.
const RULER_MARKS: &[(u8, &str)] = &[
    (127, "127"),
    (100, "0dB"),
    (80, "80"),
    (60, "60"),
    (40, "40"),
    (20, "20"),
    (0, "0"),
];

/// Events emitted by [`VolumeFader`].
pub enum VolumeFaderEvent {
    /// The MIDI volume value changed (0 to 127).
    ValueChanged(u8),
}

/// A tall mixer-channel volume fader with ruler marks.
///
/// The fader operates in MIDI value space (0-127) with ruler marks at
/// standard reference points. The 100 mark is labeled "0dB".
///
/// # Usage
///
/// ```ignore
/// VolumeFader::new(cx, data_lens.map(|d| d.volume), |cx, val| {
///     cx.emit(MyEvent::SetVolume(val));
/// });
/// ```
#[derive(Lens)]
pub struct VolumeFader {
    /// Current MIDI value (0-127).
    #[lens(ignore)]
    value: u8,
    /// Default MIDI value for double-click reset.
    #[lens(ignore)]
    default_value: u8,
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

impl VolumeFader {
    /// Creates a new [`VolumeFader`] bound to a MIDI value (0-127) lens.
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
            default_value: 100,
            drag_active: false,
            drag_start_y: 0.0,
            drag_start_value: initial_value,
            on_change: Some(Box::new(on_change)),
        }
        .build(cx, move |cx| {
            // Track container
            ZStack::new(cx, |cx| {
                // Ruler marks
                for &(midi_val, label_text) in RULER_MARKS {
                    let norm = midi_val as f32 / 127.0;
                    let top_pct = (1.0 - norm) * 100.0;
                    let is_zero_db = midi_val == 100;

                    HStack::new(cx, |cx| {
                        // Tick mark
                        Element::new(cx)
                            .class("volume-fader__ruler-tick")
                            .toggle_class("volume-fader__ruler-tick--zero", is_zero_db)
                            .hoverable(false);

                        // Label
                        Label::new(cx, label_text)
                            .class("volume-fader__ruler-label")
                            .toggle_class("volume-fader__ruler-label--zero", is_zero_db)
                            .hoverable(false);
                    })
                    .class("volume-fader__ruler-mark")
                    .top(Percentage(top_pct))
                    .hoverable(false);
                }

                // Groove
                Element::new(cx)
                    .class("volume-fader__groove")
                    .hoverable(false);

                // Cap / thumb
                Binding::new(cx, value_lens, move |cx, val| {
                    let v = val.get(cx);
                    let norm = v as f32 / 127.0;
                    let top_pct = (1.0 - norm) * 100.0;
                    Element::new(cx)
                        .class("volume-fader__cap")
                        .top(Percentage(top_pct))
                        .hoverable(false);
                });
            })
            .class("volume-fader__track");

            // Value display
            Binding::new(cx, value_lens, |cx, val| {
                let v = val.get(cx);
                Label::new(cx, &v.to_string())
                    .class("volume-fader__value")
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

impl View for VolumeFader {
    fn element(&self) -> Option<&'static str> {
        Some("volume-fader")
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
                        GRANULAR_DRAG_MULTIPLIER * 127.0 / TRACK_HEIGHT
                    } else {
                        127.0 / TRACK_HEIGHT
                    };
                    let new_value = self.drag_start_value as f32 + dy * sensitivity;
                    self.update_value(cx, new_value.round() as i32);
                    meta.consume();
                }
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                self.update_value(cx, self.default_value as i32);
                meta.consume();
            }
            WindowEvent::MouseScroll(_, scroll_y) => {
                let delta = if cx.modifiers().contains(Modifiers::SHIFT) {
                    (*scroll_y * 10.0) as i32
                } else {
                    *scroll_y as i32
                };
                self.update_value(cx, self.value as i32 + delta);
                meta.consume();
            }
            _ => {}
        });
    }
}
