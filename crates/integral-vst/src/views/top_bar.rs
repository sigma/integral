//! Top bar view with studio set name, selected part tone info, and preview toggle.
//!
//! Mirrors the `TopBar` React component from `web/src/TopBar.tsx`.

use nih_plug_vizia::vizia::prelude::*;

use super::MixerData;

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Events emitted by the [`TopBar`] view.
pub enum TopBarEvent {
    /// Toggle the preview mode on/off.
    TogglePreview,
    /// Open the tone selector modal for the currently selected part.
    OpenToneSelector,
}

// ---------------------------------------------------------------------------
// TopBar view
// ---------------------------------------------------------------------------

/// Horizontal bar at the top of the mixer page.
///
/// Displays the studio set name, selected part tone info (clickable to open
/// the tone selector), and a preview toggle button.
pub struct TopBar;

impl TopBar {
    /// Creates a new [`TopBar`] view.
    ///
    /// Reads from [`MixerData`] lenses for studio set name, part info, and
    /// preview state.
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self.build(cx, |cx| {
            // Studio set name label.
            Binding::new(cx, MixerData::studio_set_name, |cx, name_lens| {
                let name = name_lens.get(cx);
                let display = if name.is_empty() {
                    "Studio Set".to_string()
                } else {
                    name
                };
                Label::new(cx, &display).class("top-bar__studio-set");
            });

            // Selected part tone info (clickable).
            Binding::new(cx, MixerData::selected_part_info, |cx, info_lens| {
                let text = info_lens.get(cx);
                Label::new(cx, &text)
                    .class("top-bar__tone-info")
                    .cursor(CursorIcon::Hand)
                    .on_press(|cx| {
                        cx.emit(TopBarEvent::OpenToneSelector);
                    });
            });

            // Spacer to push preview button to the right.
            Element::new(cx).class("top-bar__spacer");

            // Preview toggle button.
            Binding::new(cx, MixerData::preview_active, |cx, preview_lens| {
                let active = preview_lens.get(cx);
                Label::new(cx, "PREVIEW")
                    .class("preview-btn")
                    .toggle_class("preview-btn--active", active)
                    .cursor(CursorIcon::Hand)
                    .on_press(|cx| {
                        cx.emit(TopBarEvent::TogglePreview);
                    });
            });
        })
    }
}

impl View for TopBar {
    fn element(&self) -> Option<&'static str> {
        Some("top-bar")
    }
}
