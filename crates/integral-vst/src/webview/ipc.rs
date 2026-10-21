//! The page ↔ plugin message codec.
//!
//! One JSON object per message, tagged by `type`. The page's half is
//! `web/src/plugin.ts`; the wire strings in the tests here are the ones its
//! tests use, so a change to either side breaks one of them.
//!
//! The page posts with `window.ipc.postMessage`; the plugin delivers by
//! evaluating a script that dispatches [`PLUGIN_EVENT`] on `window` with the
//! message as a JSON string `detail` ([`delivery_script`]).

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The window event the page listens on (`PLUGIN_EVENT` in `plugin.ts`).
pub(crate) const PLUGIN_EVENT: &str = "integral-plugin";

/// A message from the page.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum PageMessage {
    /// The page is up: answer with the port list, and deliver from now on.
    Ready,
    /// Open the pair `port`, closing any other.
    Open {
        /// The pair's name, as listed.
        port: String,
    },
    /// Send one MIDI message to the open pair `port`.
    Send {
        /// The pair's name, as opened.
        port: String,
        /// The message bytes.
        data: Vec<u8>,
    },
    /// Resize the window to `width` × `height` logical px.
    Resize {
        /// Logical px.
        width: u32,
        /// Logical px.
        height: u32,
    },
}

/// A message to the page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum PluginMessage {
    /// The pairs the link can open, by name.
    Ports {
        /// Pair names.
        ports: Vec<String>,
    },
    /// The pair `port` is open.
    Opened {
        /// The pair's name.
        port: String,
    },
    /// The pair `port` would not open.
    OpenFailed {
        /// The pair's name.
        port: String,
        /// Why, for the page's log.
        message: String,
    },
    /// A message the open pair `port` received.
    Receive {
        /// The pair's name.
        port: String,
        /// The message bytes.
        data: Vec<u8>,
    },
    /// A request other than an open failed.
    Error {
        /// The pair the request was about, if any.
        port: Option<String>,
        /// What went wrong, for the page's log.
        message: String,
    },
}

/// A page message that doesn't decode.
#[derive(Debug, Error)]
#[error("malformed page message: {0}")]
pub(crate) struct DecodeError(#[from] serde_json::Error);

/// Decode one message from the page.
pub(crate) fn decode(text: &str) -> Result<PageMessage, DecodeError> {
    Ok(serde_json::from_str(text)?)
}

/// Encode one message for the page.
pub(crate) fn encode(message: &PluginMessage) -> String {
    serde_json::to_string(message).expect("plugin messages always serialize")
}

/// A script that delivers `messages` to the page, in order, or `None` when
/// there are none.
pub(crate) fn delivery_script(messages: &[PluginMessage]) -> Option<String> {
    if messages.is_empty() {
        return None;
    }
    let mut script = String::new();
    for message in messages {
        // A JSON string literal is a JS string literal.
        let detail = serde_json::to_string(&encode(message)).expect("strings serialize");
        script.push_str(&format!(
            "window.dispatchEvent(new CustomEvent({PLUGIN_EVENT:?},{{detail:{detail}}}));"
        ));
    }
    Some(script)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_every_page_message() {
        assert_eq!(decode(r#"{"type":"ready"}"#).unwrap(), PageMessage::Ready);
        assert_eq!(
            decode(r#"{"type":"open","port":"INTEGRA-7"}"#).unwrap(),
            PageMessage::Open {
                port: "INTEGRA-7".into()
            }
        );
        // As `plugin.test.ts` encodes it.
        assert_eq!(
            decode(r#"{"type":"send","port":"I7","data":[240,247]}"#).unwrap(),
            PageMessage::Send {
                port: "I7".into(),
                data: vec![0xF0, 0xF7]
            }
        );
        assert_eq!(
            decode(r#"{"type":"resize","width":800,"height":600}"#).unwrap(),
            PageMessage::Resize {
                width: 800,
                height: 600
            }
        );
    }

    #[test]
    fn rejects_malformed_page_messages() {
        for text in [
            "",
            "null",
            r#"{"type":"nope"}"#,
            r#"{"type":"open"}"#,
            r#"{"type":"send","port":"A","data":[256]}"#,
            r#"{"type":"send","port":"A","data":[-1]}"#,
            r#"{"type":"resize","width":-1,"height":1}"#,
            r#"{"type":"open","port":"A","extra":1}"#,
        ] {
            assert!(decode(text).is_err(), "{text}");
        }
    }

    #[test]
    fn encodes_every_plugin_message() {
        // As `plugin.test.ts` decodes them.
        assert_eq!(
            encode(&PluginMessage::Ports {
                ports: vec!["A".into(), "B".into()]
            }),
            r#"{"type":"ports","ports":["A","B"]}"#
        );
        assert_eq!(
            encode(&PluginMessage::Opened { port: "A".into() }),
            r#"{"type":"opened","port":"A"}"#
        );
        assert_eq!(
            encode(&PluginMessage::OpenFailed {
                port: "A".into(),
                message: "gone".into()
            }),
            r#"{"type":"openFailed","port":"A","message":"gone"}"#
        );
        assert_eq!(
            encode(&PluginMessage::Receive {
                port: "A".into(),
                data: vec![240, 1, 247]
            }),
            r#"{"type":"receive","port":"A","data":[240,1,247]}"#
        );
        assert_eq!(
            encode(&PluginMessage::Error {
                port: None,
                message: "x".into()
            }),
            r#"{"type":"error","port":null,"message":"x"}"#
        );
    }

    #[test]
    fn delivers_each_message_as_a_json_string_event() {
        assert_eq!(delivery_script(&[]), None);
        let script = delivery_script(&[
            PluginMessage::Opened { port: "A\"".into() },
            PluginMessage::Ports { ports: vec![] },
        ])
        .unwrap();
        assert_eq!(
            script,
            concat!(
                r#"window.dispatchEvent(new CustomEvent("integral-plugin",{detail:"{\"type\":\"opened\",\"port\":\"A\\\"\"}"}));"#,
                r#"window.dispatchEvent(new CustomEvent("integral-plugin",{detail:"{\"type\":\"ports\",\"ports\":[]}"}));"#,
            )
        );
    }
}
