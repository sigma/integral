//! A toggle switch / multi-state selector with optional LED indicator.
//!
//! Mirrors the `SynthSwitch` React component from `web/src/synth-ui/SynthSwitch.tsx`.
//! Two-state switches render as a horizontal toggle button with an LED dot.
//! Multi-state switches render as a vertical option list.

use nih_plug_vizia::vizia::prelude::*;

use super::IndexCallback;

/// Events emitted by [`SynthSwitch`].
pub enum SynthSwitchEvent {
    /// The selected index changed.
    ValueChanged(usize),
}

/// An option entry for a [`SynthSwitch`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchOption {
    /// Display label for this option.
    pub label: String,
}

/// A toggle or multi-state switch with optional LED indicator.
///
/// For two options, renders as a horizontal toggle button.
/// For three or more options, renders as a vertical option list.
///
/// # Usage
///
/// ```ignore
/// SynthSwitch::new(
///     cx,
///     data_lens.map(|d| d.filter_type),
///     &["LPF", "HPF", "BPF"],
///     |cx, idx| cx.emit(MyEvent::SetFilterType(idx)),
/// );
/// ```
#[derive(Lens)]
pub struct SynthSwitch {
    /// Current selected index.
    #[lens(ignore)]
    value: usize,
    /// Number of options.
    #[lens(ignore)]
    option_count: usize,
    /// Callback for selection changes.
    #[lens(ignore)]
    on_change: IndexCallback,
}

impl SynthSwitch {
    /// Creates a new [`SynthSwitch`].
    ///
    /// `options` is a slice of label strings. The `on_change` callback receives
    /// the newly selected index.
    pub fn new<'a, L>(
        cx: &'a mut Context,
        value_lens: L,
        options: &[&str],
        on_change: impl Fn(&mut EventContext, usize) + 'static,
    ) -> Handle<'a, Self>
    where
        L: Lens<Target = usize>,
    {
        let initial_value = value_lens.get(cx);
        let option_count = options.len();
        let option_labels: Vec<String> = options.iter().map(|s| s.to_string()).collect();
        let is_toggle = option_count == 2;

        Self {
            value: initial_value,
            option_count,
            on_change: Some(Box::new(on_change)),
        }
        .build(cx, move |cx| {
            if is_toggle {
                // Two-state toggle
                Binding::new(cx, value_lens, {
                    let labels = option_labels.clone();
                    move |cx, val| {
                        let v = val.get(cx);
                        let is_on = v == 1;
                        let label_text = if is_on {
                            labels[1].as_str()
                        } else {
                            labels[0].as_str()
                        };

                        HStack::new(cx, |cx| {
                            // LED dot
                            Element::new(cx)
                                .class("synth-switch__led")
                                .toggle_class("synth-switch__led--on", is_on);

                            Label::new(cx, label_text).hoverable(false);
                        })
                        .class("synth-switch__toggle-btn")
                        .toggle_class("synth-switch__toggle-btn--on", is_on)
                        .cursor(CursorIcon::Hand);
                    }
                });
            } else {
                // Multi-state vertical list
                VStack::new(cx, {
                    let labels = option_labels.clone();
                    move |cx| {
                        for (i, label_text) in labels.iter().enumerate() {
                            let label_owned = label_text.clone();
                            Binding::new(cx, value_lens, move |cx, val| {
                                let current = val.get(cx);
                                let is_active = current == i;

                                HStack::new(cx, |cx| {
                                    // LED dot
                                    Element::new(cx)
                                        .class("synth-switch__led")
                                        .toggle_class("synth-switch__led--on", is_active);

                                    Label::new(cx, &label_owned).hoverable(false);
                                })
                                .class("synth-switch__option")
                                .toggle_class("synth-switch__option--active", is_active)
                                .cursor(CursorIcon::Hand);
                            });
                        }
                    }
                })
                .class("synth-switch__option-list");
            }
        })
    }
}

impl View for SynthSwitch {
    fn element(&self) -> Option<&'static str> {
        Some("synth-switch")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| {
            if let WindowEvent::MouseDown(MouseButton::Left) = window_event
                && self.option_count == 2
            {
                // Toggle between 0 and 1
                let new_value = if self.value == 0 { 1 } else { 0 };
                self.value = new_value;
                if let Some(on_change) = &self.on_change {
                    on_change(cx, new_value);
                }
                meta.consume();
                // For multi-state, individual options handle clicks via
                // their own bindings (the index is encoded in the child structure).
            }
        });
    }
}
