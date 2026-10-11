//! A mixer channel strip view with variant-based layout.
//!
//! Mirrors the `ChannelStrip` React component from `web/src/ChannelStrip.tsx`.
//! Supports Part, Ext, Master, and CompEq variants, each with a different
//! subset of controls arranged in a vertical strip.

use nih_plug_vizia::vizia::prelude::*;

use super::{PanKnob, SynthKnob, SynthSelect, VolumeFader};

/// The variant of a channel strip, determining which controls are shown.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Data)]
pub enum StripVariant {
    /// Full part strip: channel label, pan, FX sends, mute/solo, fader.
    #[default]
    Part,
    /// External input: label, fader, mute button.
    Ext,
    /// Master output: label, fader only.
    Master,
    /// Compressor + EQ unit strip: label and route chip.
    CompEq,
}

/// An edit made on a [`ChannelStrip`] control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripEdit {
    /// Volume fader moved (0-127).
    Volume(u8),
    /// Pan knob moved (0-127, 64 = centre).
    Pan(u8),
    /// FX1 send knob moved (0-127).
    Fx1Send(u8),
    /// FX2 send knob moved (0-127).
    Fx2Send(u8),
    /// Output route picked (index into the strip's route names).
    Route(u8),
    /// Mute button pressed.
    ToggleMute,
    /// Solo button pressed.
    ToggleSolo,
}

/// Data model for a channel strip, used as a lens source.
#[derive(Debug, Clone, Lens)]
pub struct ChannelStripData {
    /// Display label (e.g. "Ch 1", "Ext", "Master").
    pub label: String,
    /// MIDI volume value (0-127).
    pub volume: u8,
    /// MIDI pan value (0-127, 64 = center).
    pub pan: u8,
    /// FX1 send level (normalized 0.0-1.0).
    pub fx1_send: f32,
    /// FX2 send level (normalized 0.0-1.0).
    pub fx2_send: f32,
    /// Whether the channel is muted.
    pub muted: bool,
    /// Whether the channel is soloed.
    pub soloed: bool,
    /// Output route index into `route_names`.
    pub route: u8,
    /// Device-spec output names for the route chip; empty for strips
    /// without an output.
    pub route_names: &'static [&'static str],
    /// Whether the output is overridden by Motional Surround.
    pub route_overridden: bool,
}

impl Model for ChannelStripData {}

/// A mixer channel strip view.
///
/// Layout and controls depend on the [`StripVariant`]:
/// - **Part**: channel label, pan knob, FX1/FX2 send knobs, mute/solo buttons, volume fader
/// - **Ext**: label, volume fader, mute button
/// - **Master**: label, volume fader
/// - **CompEq**: label, route chip
///
/// Every control edit is reported to `on_edit` as a [`StripEdit`].
///
/// # Usage
///
/// ```ignore
/// ChannelStrip::new(cx, StripVariant::Part, strip_data_lens, move |cx, edit| {
///     cx.emit(MyEvent::Strip(part, edit));
/// });
/// ```
pub struct ChannelStrip;

impl ChannelStrip {
    /// Creates a new [`ChannelStrip`] view.
    ///
    /// The `variant` determines which controls are rendered.
    /// `data_lens` must resolve to a [`ChannelStripData`] model; `on_edit`
    /// receives every control edit.
    pub fn new<'a, L, F>(
        cx: &'a mut Context,
        variant: StripVariant,
        data_lens: L,
        on_edit: F,
    ) -> Handle<'a, Self>
    where
        L: Lens<Target = ChannelStripData>,
        F: Fn(&mut EventContext, StripEdit) + Copy + Send + Sync + 'static,
    {
        Self.build(cx, move |cx| match variant {
            StripVariant::Part => Self::build_part(cx, data_lens, on_edit),
            StripVariant::Ext => Self::build_ext(cx, data_lens, on_edit),
            StripVariant::Master => Self::build_master(cx, data_lens, on_edit),
            StripVariant::CompEq => Self::build_comp_eq(cx, data_lens, on_edit),
        })
    }

    /// Build the full Part variant strip.
    fn build_part<L, F>(cx: &mut Context, data_lens: L, on_edit: F)
    where
        L: Lens<Target = ChannelStripData>,
        F: Fn(&mut EventContext, StripEdit) + Copy + Send + Sync + 'static,
    {
        // Channel label
        Binding::new(cx, data_lens.map(|d| d.label.clone()), |cx, label| {
            let text = label.get(cx);
            Label::new(cx, &text)
                .class("channel-strip__label")
                .hoverable(false);
        });

        // Pan knob
        PanKnob::new(cx, data_lens.map(|d| d.pan), move |cx, val| {
            on_edit(cx, StripEdit::Pan(val));
        });

        // FX sends row
        HStack::new(cx, move |cx| {
            SynthKnob::new(cx, data_lens.map(|d| d.fx1_send), move |cx, val| {
                on_edit(cx, StripEdit::Fx1Send(norm_to_midi(val)));
            })
            .class("channel-strip__send-knob");
            SynthKnob::new(cx, data_lens.map(|d| d.fx2_send), move |cx, val| {
                on_edit(cx, StripEdit::Fx2Send(norm_to_midi(val)));
            })
            .class("channel-strip__send-knob");
        })
        .class("channel-strip__sends-row");

        Self::strip_route_chip(cx, data_lens, on_edit);

        // Mute / Solo row
        HStack::new(cx, move |cx| {
            Binding::new(cx, data_lens.map(|d| d.muted), move |cx, muted| {
                let is_muted = muted.get(cx);
                mute_button(cx, is_muted, on_edit);
            });
            Binding::new(cx, data_lens.map(|d| d.soloed), move |cx, soloed| {
                let is_soloed = soloed.get(cx);
                Label::new(cx, "S")
                    .class("solo-btn")
                    .toggle_class("solo-btn--active", is_soloed)
                    .cursor(CursorIcon::Hand)
                    .on_press(move |cx| on_edit(cx, StripEdit::ToggleSolo));
            });
        })
        .class("channel-strip__mute-row");

        Self::volume_fader(cx, data_lens, on_edit);
    }

    /// Build the Ext variant strip.
    fn build_ext<L, F>(cx: &mut Context, data_lens: L, on_edit: F)
    where
        L: Lens<Target = ChannelStripData>,
        F: Fn(&mut EventContext, StripEdit) + Copy + Send + Sync + 'static,
    {
        // Label
        Binding::new(cx, data_lens.map(|d| d.label.clone()), |cx, label| {
            let text = label.get(cx);
            Label::new(cx, &text)
                .class("channel-strip__label")
                .hoverable(false);
        });

        Self::volume_fader(cx, data_lens, on_edit);

        // Mute button
        Binding::new(cx, data_lens.map(|d| d.muted), move |cx, muted| {
            mute_button(cx, muted.get(cx), on_edit);
        });
    }

    /// Build the Master variant strip.
    fn build_master<L, F>(cx: &mut Context, data_lens: L, on_edit: F)
    where
        L: Lens<Target = ChannelStripData>,
        F: Fn(&mut EventContext, StripEdit) + Copy + Send + Sync + 'static,
    {
        // Label
        Binding::new(cx, data_lens.map(|d| d.label.clone()), |cx, label| {
            let text = label.get(cx);
            Label::new(cx, &text)
                .class("channel-strip__label")
                .hoverable(false);
        });

        Self::volume_fader(cx, data_lens, on_edit);
    }

    /// Build the CompEq variant strip.
    fn build_comp_eq<L, F>(cx: &mut Context, data_lens: L, on_edit: F)
    where
        L: Lens<Target = ChannelStripData>,
        F: Fn(&mut EventContext, StripEdit) + Copy + Send + Sync + 'static,
    {
        Binding::new(cx, data_lens.map(|d| d.label.clone()), |cx, label| {
            let text = label.get(cx);
            Label::new(cx, &text)
                .class("channel-strip__label")
                .hoverable(false);
        });

        Self::strip_route_chip(cx, data_lens, on_edit);
    }

    /// Volume fader reporting [`StripEdit::Volume`].
    fn volume_fader<L, F>(cx: &mut Context, data_lens: L, on_edit: F)
    where
        L: Lens<Target = ChannelStripData>,
        F: Fn(&mut EventContext, StripEdit) + Copy + Send + Sync + 'static,
    {
        VolumeFader::new(cx, data_lens.map(|d| d.volume), move |cx, val| {
            on_edit(cx, StripEdit::Volume(val));
        });
    }

    /// Route chip reporting [`StripEdit::Route`].
    fn strip_route_chip<L, F>(cx: &mut Context, data_lens: L, on_edit: F)
    where
        L: Lens<Target = ChannelStripData>,
        F: Fn(&mut EventContext, StripEdit) + Copy + Send + Sync + 'static,
    {
        let names = data_lens.get(cx).route_names;
        route_chip(
            cx,
            names,
            data_lens.map(|d| d.route),
            data_lens.map(|d| d.route_overridden),
            move |cx, idx| on_edit(cx, StripEdit::Route(idx)),
        );
    }
}

/// Mute button reporting [`StripEdit::ToggleMute`].
fn mute_button<F>(cx: &mut Context, is_muted: bool, on_edit: F)
where
    F: Fn(&mut EventContext, StripEdit) + Copy + Send + Sync + 'static,
{
    Label::new(cx, "M")
        .class("mute-btn")
        .toggle_class("mute-btn--active", is_muted)
        .cursor(CursorIcon::Hand)
        .on_press(move |cx| on_edit(cx, StripEdit::ToggleMute));
}

/// Convert a normalized knob value (0.0-1.0) to a MIDI value (0-127).
fn norm_to_midi(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 127.0).round() as u8
}

/// Route chip text for output index `idx` in `names`, or `→ SURR` when
/// Motional Surround overrides the output.
pub fn route_label(names: &[&str], idx: u8, overridden: bool) -> String {
    let name = if overridden {
        "SURR"
    } else {
        names.get(idx as usize).copied().unwrap_or("---")
    };
    format!("\u{2192} {name}")
}

/// Output route chip shared by part, Comp+EQ and FX strips.
///
/// Picks one of the device-spec output `names`, reporting the new index to
/// `on_change`. Read-only, shown as `→ SURR`, while Motional Surround
/// overrides the output.
pub fn route_chip<F>(
    cx: &mut Context,
    names: &'static [&'static str],
    index: impl Lens<Target = u8>,
    overridden: impl Lens<Target = bool>,
    on_change: F,
) where
    F: Fn(&mut EventContext, u8) + Copy + Send + Sync + 'static,
{
    Binding::new(cx, overridden, move |cx, overridden| {
        if overridden.get(cx) {
            Label::new(cx, index.map(move |&i| route_label(names, i, true)))
                .class("route-chip")
                .class("route-chip--overridden");
        } else {
            let options: Vec<String> = (0..names.len())
                .map(|i| route_label(names, i as u8, false))
                .collect();
            let options: Vec<&str> = options.iter().map(String::as_str).collect();
            SynthSelect::new(
                cx,
                index.map(|&i| usize::from(i)),
                &options,
                move |cx, i| on_change(cx, i as u8),
            )
            .class("route-chip");
        }
    });
}

impl View for ChannelStrip {
    fn element(&self) -> Option<&'static str> {
        Some("channel-strip")
    }
}

#[cfg(test)]
mod tests {
    use super::{norm_to_midi, route_label};

    #[test]
    fn route_label_names_output_or_surround() {
        let names = ["A", "B"];
        assert_eq!(route_label(&names, 1, false), "\u{2192} B");
        assert_eq!(route_label(&names, 1, true), "\u{2192} SURR");
        assert_eq!(route_label(&names, 9, false), "\u{2192} ---");
    }

    #[test]
    fn knob_values_map_onto_midi_range() {
        assert_eq!(norm_to_midi(0.0), 0);
        assert_eq!(norm_to_midi(64.0 / 127.0), 64);
        assert_eq!(norm_to_midi(1.0), 127);
        assert_eq!(norm_to_midi(1.5), 127);
    }
}
