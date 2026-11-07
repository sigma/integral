//! Names a plugin of the family at the type level, so one generic plugin
//! serves all six.

use integral_core::host_params::PluginKind;

/// What differs between the six plugins besides their parameter list.
pub trait Kind: 'static + Send + Sync {
    /// The list and role in core.
    const KIND: PluginKind;
    /// The name in the host's plugin list: [`PluginKind::name`].
    const NAME: &'static str;
    /// The VST3 class ID: it identifies the plugin in saved projects, so it
    /// never changes.
    const CLASS_ID: [u8; 16];
}

/// The class ID with `tag` after the family's prefix. A tag too long for the
/// 16 bytes fails to compile.
const fn class_id(tag: &[u8]) -> [u8; 16] {
    const PREFIX: &[u8] = b"IntgrI7";
    let mut id = [0; 16];
    let mut i = 0;
    while i < PREFIX.len() {
        id[i] = PREFIX[i];
        i += 1;
    }
    let mut j = 0;
    while j < tag.len() {
        id[PREFIX.len() + j] = tag[j];
        j += 1;
    }
    id
}

macro_rules! kinds {
    ($($(#[$doc:meta])* $name:ident, $tag:literal;)+) => {$(
        $(#[$doc])*
        pub struct $name;

        impl Kind for $name {
            const KIND: PluginKind = PluginKind::$name;
            const NAME: &'static str = PluginKind::$name.name();
            const CLASS_ID: [u8; 16] = class_id($tag);
        }
    )+};
}

kinds! {
    /// The Studio Set plugin's kind.
    StudioSet, b"StudioSet";
    /// The SN Synth Tone plugin's kind.
    SnSynth, b"SnSynth";
    /// The SN Acoustic Tone plugin's kind.
    SnAcoustic, b"SnAcoust";
    /// The PCM Synth Tone plugin's kind.
    PcmSynth, b"PcmSynth";
    /// The SN Drum Kit plugin's kind.
    SnDrum, b"SnDrum";
    /// The PCM Drum Kit plugin's kind.
    PcmDrum, b"PcmDrum";
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn all() -> [(PluginKind, &'static str, [u8; 16]); 6] {
        [
            (StudioSet::KIND, StudioSet::NAME, StudioSet::CLASS_ID),
            (SnSynth::KIND, SnSynth::NAME, SnSynth::CLASS_ID),
            (SnAcoustic::KIND, SnAcoustic::NAME, SnAcoustic::CLASS_ID),
            (PcmSynth::KIND, PcmSynth::NAME, PcmSynth::CLASS_ID),
            (SnDrum::KIND, SnDrum::NAME, SnDrum::CLASS_ID),
            (PcmDrum::KIND, PcmDrum::NAME, PcmDrum::CLASS_ID),
        ]
    }

    #[test]
    fn the_six_kinds_cover_the_family_in_order() {
        let kinds: Vec<_> = all().iter().map(|(k, ..)| *k).collect();
        assert_eq!(kinds, PluginKind::ALL);
    }

    #[test]
    fn names_and_class_ids_are_unique() {
        let names: HashSet<_> = all().iter().map(|(_, n, _)| *n).collect();
        let ids: HashSet<_> = all().iter().map(|(.., id)| *id).collect();
        assert_eq!(names.len(), 6);
        assert_eq!(ids.len(), 6);
    }

    #[test]
    fn class_ids_are_the_prefix_and_a_tag() {
        assert_eq!(&StudioSet::CLASS_ID, b"IntgrI7StudioSet");
        assert_eq!(&SnDrum::CLASS_ID, b"IntgrI7SnDrum\0\0\0");
    }
}
