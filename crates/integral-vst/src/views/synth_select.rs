//! A styled dropdown selector matching the dark synth theme.
//!
//! Mirrors the `SynthSelect` React component from `web/src/synth-ui/SynthSelect.tsx`.
//! Uses Vizia's built-in `Dropdown` / `List` or a custom popup for
//! selection from a list of labeled options.

use nih_plug_vizia::vizia::prelude::*;

use super::IndexCallback;

/// Events emitted by [`SynthSelect`].
pub enum SynthSelectEvent {
    /// The selected index changed.
    SelectionChanged(usize),
}

/// Internal event for toggling the dropdown open/closed.
enum SynthSelectInternal {
    /// Toggle dropdown visibility.
    Toggle,
    /// Select an option by index.
    Select(usize),
}

/// A dropdown selector styled for the dark synth control surface theme.
///
/// # Usage
///
/// ```ignore
/// SynthSelect::new(
///     cx,
///     data_lens.map(|d| d.wave_type),
///     &["Saw", "Square", "Sine", "Triangle"],
///     |cx, idx| cx.emit(MyEvent::SetWaveType(idx)),
/// );
/// ```
#[derive(Lens)]
pub struct SynthSelect {
    /// Current selected index.
    #[lens(ignore)]
    value: usize,
    /// Option labels (retained for programmatic access).
    #[lens(ignore)]
    #[allow(dead_code)]
    options: Vec<String>,
    /// Callback for selection changes.
    #[lens(ignore)]
    on_change: IndexCallback,
    /// Whether the dropdown list is currently visible.
    is_open: bool,
}

impl SynthSelect {
    /// Creates a new [`SynthSelect`] dropdown.
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
        let option_vec: Vec<String> = options.iter().map(|s| s.to_string()).collect();
        let display_options = option_vec.clone();

        Self {
            value: initial_value,
            options: option_vec,
            on_change: Some(Box::new(on_change)),
            is_open: false,
        }
        .build(cx, move |cx| {
            // Current selection display / button
            Binding::new(cx, value_lens, {
                let opts = display_options.clone();
                move |cx, val| {
                    let idx = val.get(cx);
                    let text = opts.get(idx).map(|s| s.as_str()).unwrap_or("—");

                    HStack::new(cx, |cx| {
                        Label::new(cx, text)
                            .class("synth-select__text")
                            .hoverable(false);
                        // Dropdown arrow
                        Label::new(cx, "\u{25BC}")
                            .class("synth-select__arrow")
                            .hoverable(false);
                    })
                    .class("synth-select__button")
                    .cursor(CursorIcon::Hand);
                }
            });

            // Dropdown list (visibility controlled by is_open lens)
            Binding::new(cx, SynthSelect::is_open, {
                let opts = display_options.clone();
                move |cx, is_open| {
                    if is_open.get(cx) {
                        VStack::new(cx, {
                            let opts = opts.clone();
                            move |cx| {
                                for (i, opt_label) in opts.iter().enumerate() {
                                    let label = opt_label.clone();
                                    Label::new(cx, &label)
                                        .class("synth-select__option")
                                        .cursor(CursorIcon::Hand)
                                        .on_press(move |cx| {
                                            cx.emit(SynthSelectInternal::Select(i));
                                        });
                                }
                            }
                        })
                        .class("synth-select__dropdown");
                    }
                }
            });
        })
    }
}

impl View for SynthSelect {
    fn element(&self) -> Option<&'static str> {
        Some("synth-select")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|internal_event, meta| match internal_event {
            SynthSelectInternal::Toggle => {
                self.is_open = !self.is_open;
                meta.consume();
            }
            SynthSelectInternal::Select(idx) => {
                self.value = *idx;
                self.is_open = false;
                if let Some(on_change) = &self.on_change {
                    on_change(cx, *idx);
                }
                meta.consume();
            }
        });

        event.map(|window_event, meta| {
            if let WindowEvent::MouseDown(MouseButton::Left) = window_event {
                cx.emit(SynthSelectInternal::Toggle);
                meta.consume();
            }
        });
    }
}
