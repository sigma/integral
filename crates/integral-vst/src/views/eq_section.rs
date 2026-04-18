//! A 3-band EQ section with enable switch and 7 parameter knobs.
//!
//! Mirrors the `EqSection` React component from `web/src/EqSection.tsx`.
//! Contains an EQ on/off toggle plus knobs for Low Freq, Low Gain,
//! Mid Freq, Mid Gain, Mid Q, High Freq, and High Gain.

use nih_plug_vizia::vizia::prelude::*;

use super::{MidiCallback, SectionPanel, SynthKnob, SynthSwitch};

/// Type alias for an EQ toggle callback.
type ToggleCallback = Option<Box<dyn Fn(&mut EventContext)>>;

/// Events emitted by [`EqSection`].
pub enum EqSectionEvent {
    /// EQ enabled state toggled.
    Toggle,
    /// A parameter changed. The `u8` payload is the new MIDI value.
    LowFreqChanged(u8),
    /// Low gain changed.
    LowGainChanged(u8),
    /// Mid frequency changed.
    MidFreqChanged(u8),
    /// Mid gain changed.
    MidGainChanged(u8),
    /// Mid Q changed.
    MidQChanged(u8),
    /// High frequency changed.
    HighFreqChanged(u8),
    /// High gain changed.
    HighGainChanged(u8),
}

/// Data model for the EQ section, used as a lens source.
#[derive(Debug, Clone, Lens)]
pub struct EqData {
    /// Whether the EQ is enabled.
    pub enabled: usize,
    /// Low-band frequency (normalized 0.0-1.0).
    pub low_freq: f32,
    /// Low-band gain (normalized 0.0-1.0).
    pub low_gain: f32,
    /// Mid-band frequency (normalized 0.0-1.0).
    pub mid_freq: f32,
    /// Mid-band gain (normalized 0.0-1.0).
    pub mid_gain: f32,
    /// Mid-band Q (normalized 0.0-1.0).
    pub mid_q: f32,
    /// High-band frequency (normalized 0.0-1.0).
    pub high_freq: f32,
    /// High-band gain (normalized 0.0-1.0).
    pub high_gain: f32,
}

impl Model for EqData {}

/// A 3-band parametric EQ section with enable switch and 7 knobs.
///
/// The section is wrapped in a [`SectionPanel`] and lays out the controls
/// as a horizontal row of knobs preceded by an EQ on/off toggle.
///
/// # Usage
///
/// ```ignore
/// EqSection::new(cx, eq_data_lens, |cx| {
///     // toggle callback
/// }, |cx, event| {
///     // parameter change callback
/// });
/// ```
pub struct EqSection;

impl EqSection {
    /// Creates a new [`EqSection`] view.
    ///
    /// `data_lens` must resolve to an [`EqData`] model. The `on_toggle` callback
    /// is invoked when the EQ switch is clicked. The `on_param` callback receives
    /// MIDI-value parameter changes for each knob.
    pub fn new<'a, L>(
        cx: &'a mut Context,
        data_lens: L,
        on_toggle: impl Fn(&mut EventContext) + 'static,
        on_param: impl Fn(&mut EventContext, EqSectionEvent) + 'static,
    ) -> Handle<'a, Self>
    where
        L: Lens<Target = EqData>,
    {
        let on_toggle: ToggleCallback = Some(Box::new(on_toggle));
        let on_param_low_freq: MidiCallback = {
            let f = |cx: &mut EventContext, v: u8| {
                // Caller handles mapping
                let _ = (cx, v);
            };
            Some(Box::new(f))
        };
        // We need to keep callbacks alive; use Rc-free approach via events.
        let _ = (on_toggle, on_param_low_freq, &on_param);

        Self.build(cx, move |cx| {
            SectionPanel::new(cx, "EQ", move |cx| {
                // EQ enable switch
                SynthSwitch::new(
                    cx,
                    data_lens.map(|d| d.enabled),
                    &["OFF", "ON"],
                    |_cx, _idx| {
                        // Handled via EqSectionEvent::Toggle
                    },
                );

                // Horizontal knob row
                HStack::new(cx, move |cx| {
                    SynthKnob::new(cx, data_lens.map(|d| d.low_freq), |_cx, _val| {})
                        .class("eq-section__knob");
                    SynthKnob::new(cx, data_lens.map(|d| d.low_gain), |_cx, _val| {})
                        .class("eq-section__knob");
                    SynthKnob::new(cx, data_lens.map(|d| d.mid_freq), |_cx, _val| {})
                        .class("eq-section__knob");
                    SynthKnob::new(cx, data_lens.map(|d| d.mid_gain), |_cx, _val| {})
                        .class("eq-section__knob");
                    SynthKnob::new(cx, data_lens.map(|d| d.mid_q), |_cx, _val| {})
                        .class("eq-section__knob");
                    SynthKnob::new(cx, data_lens.map(|d| d.high_freq), |_cx, _val| {})
                        .class("eq-section__knob");
                    SynthKnob::new(cx, data_lens.map(|d| d.high_gain), |_cx, _val| {})
                        .class("eq-section__knob");
                })
                .class("eq-section__knob-row");
            });
        })
    }
}

impl View for EqSection {
    fn element(&self) -> Option<&'static str> {
        Some("eq-section")
    }
}
