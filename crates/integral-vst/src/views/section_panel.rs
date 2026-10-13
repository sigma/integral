//! A bordered panel container with a family-coloured top bar and label.
//!
//! Mirrors the `SectionPanel` React component from `web/src/synth-ui/SectionPanel.tsx`.
//! Used to group related controls under a labeled section with a dark background.
//! The top bar and the header underline take the colour of the section's
//! Parameter family.

use integral_core::family::ParameterFamily;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg;

use super::family_colour;

/// A panel container with a family-coloured top bar and title label.
///
/// Child views are placed inside the panel body. The panel has a dark background,
/// thin border, and a thicker top bar; the top bar and the header underline are
/// coloured by [`SectionPanelExt::family`], neutral (theme CSS) without one.
///
/// # Usage
///
/// ```ignore
/// SectionPanel::new(cx, "FILTER", |cx| {
///     SynthKnob::new(cx, lens, callback);
///     SynthKnob::new(cx, lens2, callback2);
/// })
/// .family(ParameterFamily::Filter);
/// ```
pub struct SectionPanel {
    /// Parameter family colouring the top bar and header underline.
    family: Option<ParameterFamily>,
}

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
        Self { family: None }.build(cx, move |cx| {
            FamilyRule.build(cx, |_| {}).class("section-panel__top");

            // Header bar
            Label::new(cx, &title_owned)
                .class("section-panel__header")
                .hoverable(false);
            FamilyRule
                .build(cx, |_| {})
                .class("section-panel__underline");

            // Body containing child views
            VStack::new(cx, content).class("section-panel__body");
        })
    }
}

/// Extension trait for configuring [`SectionPanel`] appearance.
pub trait SectionPanelExt {
    /// Colour the top bar and header underline by `family`, from the core palette.
    fn family(self, family: ParameterFamily) -> Self;
}

impl SectionPanelExt for Handle<'_, SectionPanel> {
    fn family(self, family: ParameterFamily) -> Self {
        self.modify(|panel| panel.family = Some(family))
    }
}

impl View for SectionPanel {
    fn element(&self) -> Option<&'static str> {
        Some("section-panel")
    }
}

/// A rule of a [`SectionPanel`] filled with the panel's family colour, or
/// with its theme `background-color` when the panel has no family.
struct FamilyRule;

impl View for FamilyRule {
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        if bounds.w < 1.0 || bounds.h <= 0.0 {
            return;
        }
        let colour: vg::Color = cx
            .data::<SectionPanel>()
            .and_then(|panel| panel.family)
            .map_or(cx.background_color(), family_colour)
            .into();
        let mut path = vg::Path::new();
        path.rect(bounds.x, bounds.y, bounds.w, bounds.h);
        canvas.fill_path(&path, &vg::Paint::color(colour));
    }
}
