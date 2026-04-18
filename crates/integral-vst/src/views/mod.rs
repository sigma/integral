//! Synth-style Vizia controls for the Integra-7 VST plugin.
//!
//! These views mirror the React synth-ui components from the web frontend,
//! providing a consistent dark-themed control surface aesthetic across both
//! the web and VST plugin interfaces.

use nih_plug_vizia::vizia::prelude::*;

/// Type alias for a boxed value-change callback taking a normalized `f32`.
pub(crate) type NormCallback = Option<Box<dyn Fn(&mut EventContext, f32)>>;

/// Type alias for a boxed value-change callback taking a `u8` (MIDI value).
pub(crate) type MidiCallback = Option<Box<dyn Fn(&mut EventContext, u8)>>;

/// Type alias for a boxed value-change callback taking a `usize` (index).
pub(crate) type IndexCallback = Option<Box<dyn Fn(&mut EventContext, usize)>>;

mod channel_strip;
mod envelope;
mod eq_section;
mod mixer_page;
mod pan_knob;
mod section_panel;
mod synth_fader;
mod synth_knob;
mod synth_select;
mod synth_switch;
pub(crate) mod tone_selector;
pub(crate) mod top_bar;
mod volume_fader;

pub use channel_strip::*;
pub use envelope::*;
pub use eq_section::*;
pub use mixer_page::*;
pub use pan_knob::*;
pub use section_panel::*;
pub use synth_fader::*;
pub use synth_knob::*;
pub use synth_select::*;
pub use synth_switch::*;
pub use tone_selector::ToneSelector;
pub use top_bar::TopBar;
pub use volume_fader::*;
