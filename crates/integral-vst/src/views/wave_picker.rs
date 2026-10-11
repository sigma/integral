//! Wave picker: a `‹ name ›` stepper with a family-filtered popover list.
//!
//! The arrows step the Wave Number; clicking the name opens a popover that
//! lists `<number> <name>` for every Wave of a [`WaveGroup`], filtered by the
//! family list from core. There is no text search: hosts often swallow
//! keystrokes. The list follows [`ToneSelector`](super::ToneSelector).

use integral_core::waves::{WaveGroup, families, name, waves};
use nih_plug_vizia::vizia::prelude::*;

/// Callback receiving a newly picked Wave Number.
type WaveCallback = Box<dyn Fn(&mut EventContext, u16)>;

/// Internal events of the [`WavePicker`].
enum WavePickerInternal {
    /// Open or close the popover.
    Toggle,
    /// Set Wave `n` from the stepper, leaving the popover as is.
    Step(u16),
    /// Pick Wave `n` from the popover list, closing it.
    Pick(u16),
    /// Show only the family at this index of the family list (0 is "All").
    Filter(usize),
}

/// Stepper plus popover list for picking a Wave of one [`WaveGroup`].
#[derive(Lens)]
pub struct WavePicker {
    /// Called with the new Wave Number.
    #[lens(ignore)]
    on_change: WaveCallback,
    /// Whether the popover is open.
    open: bool,
    /// Selected entry of the family list: 0 is "All", `i` is family `i - 1`.
    family: usize,
}

impl WavePicker {
    /// Creates a [`WavePicker`] over the Waves of `group`.
    ///
    /// `value` holds the current Wave Number (0 is OFF); `on_change` receives
    /// the newly picked one.
    pub fn new<'a, L>(
        cx: &'a mut Context,
        group: WaveGroup,
        value: L,
        on_change: impl Fn(&mut EventContext, u16) + 'static,
    ) -> Handle<'a, Self>
    where
        L: Lens<Target = u16>,
    {
        Self {
            on_change: Box::new(on_change),
            open: false,
            family: 0,
        }
        .build(cx, move |cx| {
            let last = waves(group).last().map_or(0, |w| w.number);

            Binding::new(cx, value, move |cx, value| {
                let n = value.get(cx);
                HStack::new(cx, |cx| {
                    Label::new(cx, "\u{2039}")
                        .class("wave-picker__arrow")
                        .cursor(CursorIcon::Hand)
                        .on_press(move |cx| {
                            cx.emit(WavePickerInternal::Step(n.saturating_sub(1)));
                        });
                    Label::new(cx, name(group, n).unwrap_or("?"))
                        .class("synth-select__text")
                        .class("wave-picker__name")
                        .cursor(CursorIcon::Hand)
                        .on_press(|cx| cx.emit(WavePickerInternal::Toggle));
                    Label::new(cx, "\u{203A}")
                        .class("wave-picker__arrow")
                        .cursor(CursorIcon::Hand)
                        .on_press(move |cx| {
                            cx.emit(WavePickerInternal::Step(n.saturating_add(1).min(last)));
                        });
                })
                .class("synth-select__button")
                .class("wave-picker__stepper");
            });

            Binding::new(cx, WavePicker::open, move |cx, open| {
                if open.get(cx) {
                    build_popover(cx, group, value);
                }
            });
        })
    }
}

impl View for WavePicker {
    fn element(&self) -> Option<&'static str> {
        Some("wave-picker")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|e, meta| {
            match e {
                WavePickerInternal::Toggle => self.open = !self.open,
                WavePickerInternal::Step(n) => (self.on_change)(cx, *n),
                WavePickerInternal::Pick(n) => {
                    self.open = false;
                    (self.on_change)(cx, *n);
                }
                WavePickerInternal::Filter(i) => self.family = *i,
            }
            meta.consume();
        });
    }
}

/// Build the popover: family list on the left, Wave list on the right.
fn build_popover<L>(cx: &mut Context, group: WaveGroup, value: L)
where
    L: Lens<Target = u16>,
{
    let families = families(group);
    HStack::new(cx, |cx| {
        let family_labels = families.clone();
        ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
            Binding::new(cx, WavePicker::family, move |cx, sel| {
                let sel = sel.get(cx);
                VStack::new(cx, |cx| {
                    let entries = std::iter::once("All").chain(family_labels.iter().copied());
                    for (i, label) in entries.enumerate() {
                        Label::new(cx, label)
                            .class("tone-selector__bank-btn")
                            .toggle_class("tone-selector__bank-btn--active", i == sel)
                            .cursor(CursorIcon::Hand)
                            .on_press(move |cx| cx.emit(WavePickerInternal::Filter(i)));
                    }
                });
            });
        })
        .class("wave-picker__families");

        ScrollView::new(cx, 0.0, 0.0, false, true, move |cx| {
            Binding::new(cx, WavePicker::family, move |cx, sel| {
                let filter = sel.get(cx).checked_sub(1).map(|i| families[i]);
                Binding::new(cx, value, move |cx, value| {
                    let current = value.get(cx);
                    VStack::new(cx, |cx| {
                        for wave in waves(group)
                            .iter()
                            .filter(|w| filter.is_none_or(|f| w.family() == f))
                        {
                            let number = wave.number;
                            Label::new(cx, &format!("{number:03}  {}", wave.name))
                                .class("tone-selector__tone-item")
                                .toggle_class("tone-selector__tone-item--active", number == current)
                                .cursor(CursorIcon::Hand)
                                .on_press(move |cx| cx.emit(WavePickerInternal::Pick(number)));
                        }
                    });
                });
            });
        })
        .class("wave-picker__waves");
    })
    .class("wave-picker__popover");
}
