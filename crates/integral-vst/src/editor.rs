//! Vizia-based editor window for the Integral VST plugin.

use nih_plug::prelude::Editor;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::{create_vizia_editor, ViziaState, ViziaTheming};
use std::sync::Arc;

use crate::views::MixerPage;
use crate::SharedState;

/// Editor data exposed to Vizia views via the Lens system.
#[derive(Lens)]
pub(crate) struct EditorData {
    #[allow(dead_code)]
    pub shared: Arc<SharedState>,
}

impl Model for EditorData {}

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
        }
        .build(cx);

        // Root layout: tab bar + page content.
        VStack::new(cx, |cx| {
            // Tab bar placeholder.
            HStack::new(cx, |cx| {
                Label::new(cx, "INTEGRAL")
                    .class("logo");
                Label::new(cx, "Mixer")
                    .class("tab")
                    .class("tab-active");
                Label::new(cx, "Surround")
                    .class("tab");
                Label::new(cx, "Routing")
                    .class("tab");
                Label::new(cx, "Tone Edit")
                    .class("tab");
            })
            .class("tab-bar");

            // Page content — Mixer page.
            MixerPage::new(cx, shared.clone())
                .class("page-content");
        })
        .class("root");
    })
}
