//! Vizia-based editor window for the Integral VST plugin.

use nih_plug::prelude::Editor;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::{create_vizia_editor, ViziaState, ViziaTheming};
use std::sync::Arc;

use crate::views::{MixerPage, RoutingPage, SurroundPage};
use crate::SharedState;

// ---------------------------------------------------------------------------
// Tab state
// ---------------------------------------------------------------------------

/// The active page tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Data)]
pub(crate) enum PageTab {
    /// Mixer page.
    Mixer,
    /// Surround page.
    Surround,
    /// Routing page.
    Routing,
    /// Tone Edit page (placeholder).
    ToneEdit,
}

/// Editor data exposed to Vizia views via the Lens system.
#[derive(Lens)]
pub(crate) struct EditorData {
    #[allow(dead_code)]
    pub shared: Arc<SharedState>,
    /// Currently active page tab.
    pub active_tab: PageTab,
}

/// Events for the editor.
pub(crate) enum EditorEvent {
    /// Switch to a different page tab.
    SetTab(PageTab),
}

impl Model for EditorData {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|e, _| match e {
            EditorEvent::SetTab(tab) => {
                self.active_tab = *tab;
            }
        });
    }
}

/// Default editor window size (1200 x 800).
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (1200, 800))
}

/// Create the Vizia editor.
pub(crate) fn create(
    shared: Arc<SharedState>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        // Dark theme matching the web app.
        cx.add_stylesheet(include_str!("theme.css"))
            .expect("failed to load theme CSS");

        EditorData {
            shared: shared.clone(),
            active_tab: PageTab::Mixer,
        }
        .build(cx);

        // Root layout: tab bar + page content.
        VStack::new(cx, |cx| {
            // Tab bar
            HStack::new(cx, |cx| {
                Label::new(cx, "INTEGRAL").class("logo");

                Binding::new(cx, EditorData::active_tab, |cx, tab_lens| {
                    let active = tab_lens.get(cx);

                    Label::new(cx, "Mixer")
                        .class("tab")
                        .toggle_class("tab-active", active == PageTab::Mixer)
                        .cursor(CursorIcon::Hand)
                        .on_press(|cx| cx.emit(EditorEvent::SetTab(PageTab::Mixer)));

                    Label::new(cx, "Surround")
                        .class("tab")
                        .toggle_class("tab-active", active == PageTab::Surround)
                        .cursor(CursorIcon::Hand)
                        .on_press(|cx| cx.emit(EditorEvent::SetTab(PageTab::Surround)));

                    Label::new(cx, "Routing")
                        .class("tab")
                        .toggle_class("tab-active", active == PageTab::Routing)
                        .cursor(CursorIcon::Hand)
                        .on_press(|cx| cx.emit(EditorEvent::SetTab(PageTab::Routing)));

                    Label::new(cx, "Tone Edit")
                        .class("tab")
                        .toggle_class("tab-active", active == PageTab::ToneEdit)
                        .cursor(CursorIcon::Hand)
                        .on_press(|cx| cx.emit(EditorEvent::SetTab(PageTab::ToneEdit)));
                });
            })
            .class("tab-bar");

            // Page content — switched based on active tab.
            let shared_mixer = shared.clone();
            let shared_surround = shared.clone();
            let shared_routing = shared.clone();

            Binding::new(cx, EditorData::active_tab, move |cx, tab_lens| {
                let tab = tab_lens.get(cx);
                match tab {
                    PageTab::Mixer => {
                        MixerPage::new(cx, shared_mixer.clone()).class("page-content");
                    }
                    PageTab::Surround => {
                        SurroundPage::new(cx, shared_surround.clone())
                            .class("page-content");
                    }
                    PageTab::Routing => {
                        RoutingPage::new(cx, shared_routing.clone())
                            .class("page-content");
                    }
                    PageTab::ToneEdit => {
                        // Placeholder for future tone edit page.
                        VStack::new(cx, |cx| {
                            Label::new(cx, "Tone Edit (coming soon)")
                                .class("placeholder-label");
                        })
                        .class("page-content");
                    }
                }
            });
        })
        .class("root");
    })
}
