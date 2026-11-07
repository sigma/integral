//! VST3 plugins for the Integral Integra-7 control surface.
//!
//! One crate exports the six plugins of the family ([`family`], ADR 0007) from
//! one bundle: the Studio Set plugin, a Tone plugin per synth tone type and
//! a Kit plugin per drum kit type. Their editor runs the web UI in a webview
//! ([`webview`], ADR 0004), and the page reaches the device through the
//! plugin transport: today a direct MIDI link the plugin opens itself, since
//! hosts drop plugin SysEx. Each instance connects to the Device service
//! (ADR 0006) as a client, and reports its status.

use nih_plug::prelude::*;

pub mod family;

// Only the macOS editor uses the transport so far; elsewhere it is built
// and tested, but unused.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod webview;

#[cfg(test)]
mod host_param_probe;

pub use family::{
    Family, PcmDrumPlugin, PcmSynthPlugin, SnAcousticPlugin, SnDrumPlugin, SnSynthPlugin,
    StudioSetPlugin,
};

nih_export_vst3!(
    StudioSetPlugin,
    SnSynthPlugin,
    SnAcousticPlugin,
    PcmSynthPlugin,
    SnDrumPlugin,
    PcmDrumPlugin
);
