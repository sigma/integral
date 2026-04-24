//! A mixer channel strip view with variant-based layout.
//!
//! Mirrors the `ChannelStrip` React component from `web/src/ChannelStrip.tsx`.
//! Supports Part, Ext, Master, and CompEq variants, each with a different
//! subset of controls arranged in a vertical strip.

use nih_plug_vizia::vizia::prelude::*;

use super::{PanKnob, SynthKnob, VolumeFader};

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
    /// Compressor + EQ strip (stub).
    CompEq,
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
}

impl Model for ChannelStripData {}

/// A mixer channel strip view.
///
/// Layout and controls depend on the [`StripVariant`]:
/// - **Part**: channel label, pan knob, FX1/FX2 send knobs, mute/solo buttons, volume fader
/// - **Ext**: label, volume fader, mute button
/// - **Master**: label, volume fader
/// - **CompEq**: stub (empty)
///
/// # Usage
///
/// ```ignore
/// ChannelStrip::new(cx, StripVariant::Part, strip_data_lens)
///     .on_volume(|cx, v| { /* ... */ })
///     .on_mute(|cx| { /* ... */ });
/// ```
pub struct ChannelStrip;

impl ChannelStrip {
    /// Creates a new [`ChannelStrip`] view.
    ///
    /// The `variant` determines which controls are rendered.
    /// `data_lens` must resolve to a [`ChannelStripData`] model.
    pub fn new<'a, L>(cx: &'a mut Context, variant: StripVariant, data_lens: L) -> Handle<'a, Self>
    where
        L: Lens<Target = ChannelStripData>,
    {
        Self.build(cx, move |cx| match variant {
            StripVariant::Part => Self::build_part(cx, data_lens),
            StripVariant::Ext => Self::build_ext(cx, data_lens),
            StripVariant::Master => Self::build_master(cx, data_lens),
            StripVariant::CompEq => Self::build_comp_eq(cx),
        })
    }

    /// Build the full Part variant strip.
    fn build_part<L>(cx: &mut Context, data_lens: L)
    where
        L: Lens<Target = ChannelStripData>,
    {
        // Channel label
        Binding::new(cx, data_lens.map(|d| d.label.clone()), |cx, label| {
            let text = label.get(cx);
            Label::new(cx, &text)
                .class("channel-strip__label")
                .hoverable(false);
        });

        // Pan knob
        PanKnob::new(cx, data_lens.map(|d| d.pan), |_cx, _val| {});

        // FX sends row
        HStack::new(cx, move |cx| {
            SynthKnob::new(cx, data_lens.map(|d| d.fx1_send), |_cx, _val| {})
                .class("channel-strip__send-knob");
            SynthKnob::new(cx, data_lens.map(|d| d.fx2_send), |_cx, _val| {})
                .class("channel-strip__send-knob");
        })
        .class("channel-strip__sends-row");

        // Mute / Solo row
        HStack::new(cx, move |cx| {
            Binding::new(cx, data_lens.map(|d| d.muted), |cx, muted| {
                let is_muted = muted.get(cx);
                Label::new(cx, "M")
                    .class("mute-btn")
                    .toggle_class("mute-btn--active", is_muted)
                    .cursor(CursorIcon::Hand);
            });
            Binding::new(cx, data_lens.map(|d| d.soloed), |cx, soloed| {
                let is_soloed = soloed.get(cx);
                Label::new(cx, "S")
                    .class("solo-btn")
                    .toggle_class("solo-btn--active", is_soloed)
                    .cursor(CursorIcon::Hand);
            });
        })
        .class("channel-strip__mute-row");

        // Volume fader
        VolumeFader::new(cx, data_lens.map(|d| d.volume), |_cx, _val| {});
    }

    /// Build the Ext variant strip.
    fn build_ext<L>(cx: &mut Context, data_lens: L)
    where
        L: Lens<Target = ChannelStripData>,
    {
        // Label
        Binding::new(cx, data_lens.map(|d| d.label.clone()), |cx, label| {
            let text = label.get(cx);
            Label::new(cx, &text)
                .class("channel-strip__label")
                .hoverable(false);
        });

        // Volume fader
        VolumeFader::new(cx, data_lens.map(|d| d.volume), |_cx, _val| {});

        // Mute button
        Binding::new(cx, data_lens.map(|d| d.muted), |cx, muted| {
            let is_muted = muted.get(cx);
            Label::new(cx, "M")
                .class("mute-btn")
                .toggle_class("mute-btn--active", is_muted)
                .cursor(CursorIcon::Hand);
        });
    }

    /// Build the Master variant strip.
    fn build_master<L>(cx: &mut Context, data_lens: L)
    where
        L: Lens<Target = ChannelStripData>,
    {
        // Label
        Binding::new(cx, data_lens.map(|d| d.label.clone()), |cx, label| {
            let text = label.get(cx);
            Label::new(cx, &text)
                .class("channel-strip__label")
                .hoverable(false);
        });

        // Volume fader
        VolumeFader::new(cx, data_lens.map(|d| d.volume), |_cx, _val| {});
    }

    /// Build the CompEq variant stub.
    fn build_comp_eq(_cx: &mut Context) {
        // Stub — will be implemented when Comp+EQ controls are added.
    }
}

/// Extension trait for configuring [`ChannelStrip`] callbacks.
pub trait ChannelStripExt {
    /// Set the callback for volume fader changes.
    fn on_volume(self, cb: impl Fn(&mut EventContext, u8) + 'static) -> Self;
    /// Set the callback for pan knob changes.
    fn on_pan(self, cb: impl Fn(&mut EventContext, u8) + 'static) -> Self;
    /// Set the callback for FX1 send changes.
    fn on_fx1(self, cb: impl Fn(&mut EventContext, f32) + 'static) -> Self;
    /// Set the callback for FX2 send changes.
    fn on_fx2(self, cb: impl Fn(&mut EventContext, f32) + 'static) -> Self;
    /// Set the callback for mute toggle.
    fn on_mute(self, cb: impl Fn(&mut EventContext) + 'static) -> Self;
    /// Set the callback for solo toggle.
    fn on_solo(self, cb: impl Fn(&mut EventContext) + 'static) -> Self;
}

impl ChannelStripExt for Handle<'_, ChannelStrip> {
    fn on_volume(self, _cb: impl Fn(&mut EventContext, u8) + 'static) -> Self {
        // Callbacks will be wired through events in the MixerPage integration.
        self
    }

    fn on_pan(self, _cb: impl Fn(&mut EventContext, u8) + 'static) -> Self {
        self
    }

    fn on_fx1(self, _cb: impl Fn(&mut EventContext, f32) + 'static) -> Self {
        self
    }

    fn on_fx2(self, _cb: impl Fn(&mut EventContext, f32) + 'static) -> Self {
        self
    }

    fn on_mute(self, _cb: impl Fn(&mut EventContext) + 'static) -> Self {
        self
    }

    fn on_solo(self, _cb: impl Fn(&mut EventContext) + 'static) -> Self {
        self
    }
}

impl View for ChannelStrip {
    fn element(&self) -> Option<&'static str> {
        Some("channel-strip")
    }
}
