//! `wave` commands: browse the Wave catalog offline.
//!
//! The catalog lives in `integral-core` ([`integral_core::waves`]); this
//! module parses arguments and prints. It never opens a MIDI port.

use anyhow::{Context, Result};
use integral_core::waves::{self, Wave, WaveGroup};
use serde_json::{Value, json};

use super::expansion::OutputFormat;

/// What `wave list` prints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Listing {
    /// One line per Wave Group with its Wave count.
    Groups,
    /// The Waves of one group.
    Group(WaveGroup),
    /// The Waves of every group, each line prefixed with its group.
    All,
}

/// Parse a Wave Group id (`INT`, `SRX-07`, `SN-Synth`; case-insensitive).
pub fn parse_group(id: &str) -> Result<WaveGroup> {
    WaveGroup::from_id(id).with_context(|| {
        let ids: Vec<&str> = WaveGroup::ALL.iter().map(|g| g.id()).collect();
        format!(
            "unknown Wave Group `{id}` (expected one of {})",
            ids.join(", ")
        )
    })
}

/// Wave Number as printed: zero-padded to 4 digits.
pub fn format_number(number: u16) -> String {
    format!("{number:04}")
}

/// The Expansion title of `group` (SRX only).
fn title(group: WaveGroup) -> Option<&'static str> {
    group.expansion().map(|e| e.title)
}

fn waves_json(list: &[Wave]) -> Vec<Value> {
    list.iter()
        .map(|w| json!({ "number": w.number, "name": w.name }))
        .collect()
}

/// The `wave list` output.
pub fn render_list(listing: Listing, format: OutputFormat) -> String {
    match (listing, format) {
        (Listing::Groups, OutputFormat::Text) => {
            let width = WaveGroup::ALL
                .iter()
                .map(|g| g.id().len())
                .max()
                .unwrap_or(0);
            let mut out = String::new();
            for group in WaveGroup::ALL {
                let line = format!("{:width$}  {:>4}", group.id(), waves::waves(group).len());
                match title(group) {
                    Some(t) => out.push_str(&format!("{line}  {t}\n")),
                    None => out.push_str(&format!("{line}\n")),
                }
            }
            out
        }
        (Listing::Group(group), OutputFormat::Text) => waves::waves(group)
            .iter()
            .map(|w| format!("{} {}\n", format_number(w.number), w.name))
            .collect(),
        (Listing::All, OutputFormat::Text) => WaveGroup::ALL
            .into_iter()
            .flat_map(|g| {
                waves::waves(g)
                    .iter()
                    .map(move |w| format!("{} {} {}\n", g.id(), format_number(w.number), w.name))
            })
            .collect(),
        (Listing::Groups, OutputFormat::Json) => {
            let groups: Vec<Value> = WaveGroup::ALL
                .into_iter()
                .map(|g| {
                    json!({
                        "id": g.id(),
                        "waves": waves::waves(g).len(),
                        "title": title(g),
                    })
                })
                .collect();
            format!("{:#}\n", json!({ "groups": groups }))
        }
        (Listing::Group(group), OutputFormat::Json) => {
            let doc = json!({
                "group": group.id(),
                "waves": waves_json(waves::waves(group)),
            });
            format!("{doc:#}\n")
        }
        (Listing::All, OutputFormat::Json) => {
            let groups: Vec<Value> = WaveGroup::ALL
                .into_iter()
                .map(|g| json!({ "group": g.id(), "waves": waves_json(waves::waves(g)) }))
                .collect();
            format!("{:#}\n", json!({ "groups": groups }))
        }
    }
}

/// `wave list`: print the Wave Groups, one group's Waves, or every Wave.
pub fn list(group: Option<&str>, all: bool, format: OutputFormat) -> Result<()> {
    let listing = match (group, all) {
        (Some(id), _) => Listing::Group(parse_group(id)?),
        (None, true) => Listing::All,
        (None, false) => Listing::Groups,
    };
    print!("{}", render_list(listing, format));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_group_ids_case_insensitively() {
        assert_eq!(parse_group("srx-07").unwrap(), WaveGroup::Srx(7));
        assert_eq!(parse_group("int").unwrap(), WaveGroup::Int);
        assert_eq!(parse_group("sn-synth").unwrap(), WaveGroup::SnSynth);
        let err = format!("{:#}", parse_group("SRX-13").unwrap_err());
        assert!(err.contains("unknown Wave Group `SRX-13`"), "{err}");
        assert!(err.contains("INT, SRX-01"), "{err}");
    }

    #[test]
    fn pads_numbers_to_four_digits() {
        assert_eq!(format_number(1), "0001");
        assert_eq!(format_number(1209), "1209");
    }
}
