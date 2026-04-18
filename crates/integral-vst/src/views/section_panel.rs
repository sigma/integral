//! A bordered panel container with an accent-colored top bar and label.
//!
//! Mirrors the `SectionPanel` React component from `web/src/synth-ui/SectionPanel.tsx`.
//! Used to group related controls under a labeled section with a dark background
//! and colored accent border.

use nih_plug_vizia::vizia::prelude::*;

/// A panel container with an accent-colored top bar and title label.
///
/// Child views are placed inside the panel body. The panel has a dark background,
/// thin border, and a thicker accent-colored top border.
///
/// # Usage
///
/// ```ignore
/// SectionPanel::new(cx, "FILTER", |cx| {
///     SynthKnob::new(cx, lens, callback);
///     SynthKnob::new(cx, lens2, callback2);
/// });
/// ```
pub struct SectionPanel;

impl SectionPanel {
    /// Creates a new [`SectionPanel`] with a title label and child content.
    ///
    /// The `content` closure receives the context for building child views
    /// inside the panel body.
    pub fn new<'a>(
        cx: &'a mut Context,
        title: &str,
        content: impl FnOnce(&mut Context),
    ) -> Handle<'a, Self> {
        let title_owned = title.to_string();
        Self.build(cx, move |cx| {
            // Header bar
            Label::new(cx, &title_owned)
                .class("section-panel__header")
                .hoverable(false);

            // Body containing child views
            VStack::new(cx, content).class("section-panel__body");
        })
    }
}

/// Extension trait for configuring [`SectionPanel`] appearance.
pub trait SectionPanelExt {
    /// Set the accent color for the top border (e.g. `"#fc8"`).
    fn accent_color(self, color: &str) -> Self;
}

impl SectionPanelExt for Handle<'_, SectionPanel> {
    fn accent_color(self, _color: &str) -> Self {
        // Accent color would be applied via CSS custom property or inline style.
        // For now, the default accent from the theme CSS is used.
        self
    }
}

impl View for SectionPanel {
    fn element(&self) -> Option<&'static str> {
        Some("section-panel")
    }
}
