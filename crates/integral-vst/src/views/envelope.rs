//! ADSR and AD envelope views with fader-based controls.
//!
//! Mirrors the `ADSREnvelope` and `ADEnvelope` React components from
//! `web/src/synth-ui/ADSREnvelope.tsx` and `web/src/synth-ui/ADEnvelope.tsx`.
//! Each envelope is a horizontal row of [`SynthFader`] controls.
//! The SVG curve visualization is deferred to a future canvas-drawing pass.

use nih_plug_vizia::vizia::prelude::*;

use super::{SynthFader, SynthFaderExt};

/// Data model for an ADSR envelope, used as a lens source.
#[derive(Debug, Clone, Lens)]
pub struct AdsrData {
    /// Attack time (normalized 0.0-1.0).
    pub attack: f32,
    /// Decay time (normalized 0.0-1.0).
    pub decay: f32,
    /// Sustain level (normalized 0.0-1.0).
    pub sustain: f32,
    /// Release time (normalized 0.0-1.0).
    pub release: f32,
    /// Optional depth/extra parameter (normalized 0.0-1.0).
    pub depth: f32,
}

impl Model for AdsrData {}

/// Data model for an AD envelope, used as a lens source.
#[derive(Debug, Clone, Lens)]
pub struct AdData {
    /// Attack time (normalized 0.0-1.0).
    pub attack: f32,
    /// Decay time (normalized 0.0-1.0).
    pub decay: f32,
    /// Optional depth/extra parameter (normalized 0.0-1.0).
    pub depth: f32,
}

impl Model for AdData {}

/// An ADSR envelope view with 4 faders (Attack, Decay, Sustain, Release)
/// and an optional Depth fader.
///
/// # Usage
///
/// ```ignore
/// ADSREnvelope::new(cx, adsr_lens, true, true, |cx, val| {
///     // handle parameter change
/// });
/// ```
pub struct ADSREnvelope;

impl ADSREnvelope {
    /// Creates a new [`ADSREnvelope`] view.
    ///
    /// `show_depth` controls whether the optional depth fader is shown.
    /// `compact` uses shorter fader tracks.
    pub fn new<'a, L>(
        cx: &'a mut Context,
        data_lens: L,
        show_depth: bool,
        compact: bool,
    ) -> Handle<'a, Self>
    where
        L: Lens<Target = AdsrData>,
    {
        Self.build(cx, move |cx| {
            HStack::new(cx, move |cx| {
                SynthFader::new(cx, data_lens.map(|d| d.attack), 0.0, |_cx, _val| {})
                    .compact(compact);
                SynthFader::new(cx, data_lens.map(|d| d.decay), 0.0, |_cx, _val| {})
                    .compact(compact);
                SynthFader::new(cx, data_lens.map(|d| d.sustain), 1.0, |_cx, _val| {})
                    .compact(compact);
                SynthFader::new(cx, data_lens.map(|d| d.release), 0.0, |_cx, _val| {})
                    .compact(compact);

                if show_depth {
                    SynthFader::new(cx, data_lens.map(|d| d.depth), 0.5, |_cx, _val| {})
                        .compact(compact);
                }
            })
            .class("envelope__fader-row");
        })
    }
}

impl View for ADSREnvelope {
    fn element(&self) -> Option<&'static str> {
        Some("adsr-envelope")
    }
}

/// An AD envelope view with 2 faders (Attack, Decay) and an optional Depth fader.
///
/// # Usage
///
/// ```ignore
/// ADEnvelope::new(cx, ad_lens, true, false);
/// ```
pub struct ADEnvelope;

impl ADEnvelope {
    /// Creates a new [`ADEnvelope`] view.
    ///
    /// `show_depth` controls whether the optional depth fader is shown.
    /// `compact` uses shorter fader tracks.
    pub fn new<'a, L>(
        cx: &'a mut Context,
        data_lens: L,
        show_depth: bool,
        compact: bool,
    ) -> Handle<'a, Self>
    where
        L: Lens<Target = AdData>,
    {
        Self.build(cx, move |cx| {
            HStack::new(cx, move |cx| {
                SynthFader::new(cx, data_lens.map(|d| d.attack), 0.0, |_cx, _val| {})
                    .compact(compact);
                SynthFader::new(cx, data_lens.map(|d| d.decay), 0.0, |_cx, _val| {})
                    .compact(compact);

                if show_depth {
                    SynthFader::new(cx, data_lens.map(|d| d.depth), 0.5, |_cx, _val| {})
                        .compact(compact);
                }
            })
            .class("envelope__fader-row");
        })
    }
}

impl View for ADEnvelope {
    fn element(&self) -> Option<&'static str> {
        Some("ad-envelope")
    }
}
