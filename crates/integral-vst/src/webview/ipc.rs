//! The page ↔ plugin message codec.
//!
//! One JSON object per message, tagged by `type`. The page's half is
//! `web/src/plugin.ts`; the wire strings in the tests here are the ones its
//! tests use, so a change to either side breaks one of them.
//!
//! The page posts with `window.ipc.postMessage`; the plugin delivers by
//! evaluating a script that dispatches [`PLUGIN_EVENT`] on `window` with the
//! message as a JSON string `detail` ([`delivery_script`]).

use integral_core::host_params::{Binding, PluginKind};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::family::{Notice, Status};

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
    /// Bind the instance to a Part and, for a Kit plugin, a key. The plugin
    /// accepts what its kind can have and answers with the new `instance`;
    /// anything else is an `error`.
    Bind {
        /// The Part (0-based); absent for the Studio Set plugin.
        part: Option<u8>,
        /// The drum key; absent for the whole kit, and for a Tone plugin.
        key: Option<u8>,
    },
    /// Begin a host gesture on the parameter `id`: the first `input` of a
    /// Control drag.
    BeginEdit {
        /// The host parameter ID (ADR 0008).
        id: String,
    },
    /// Move the parameter `id`, which has a gesture open, to the raw `value`.
    PerformEdit {
        /// The host parameter ID.
        id: String,
        /// The raw value, in the parameter's range.
        value: i32,
    },
    /// End the gesture on the parameter `id`: the `change` of a Control drag.
    EndEdit {
        /// The host parameter ID.
        id: String,
    },
    /// Load the plugin's default tone onto the bound Part, which holds
    /// another tone type (the mismatch notice's button). Refused, as
    /// `refused`, if the instance is not on another type.
    LoadDefaultTone,
}

/// A [`Notice`] with its text, as the page shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct NoticeView {
    /// Which notice.
    pub(crate) notice: Notice,
    /// What it says.
    pub(crate) text: &'static str,
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
    /// What the instance is and how it is doing. Sent on `ready` and
    /// whenever any of it changes.
    Instance {
        /// The plugin the page runs in.
        plugin: PluginKind,
        /// What the instance is bound to, once it is.
        binding: Option<Binding>,
        /// The connection status.
        status: Status,
        /// The status as the page shows it.
        #[serde(rename = "statusText")]
        status_text: &'static str,
        /// Whether the instance can tell the host of edits. If not, the page
        /// sends no host gestures: every one would be refused.
        #[serde(rename = "hostEdits")]
        host_edits: bool,
        /// What else the instance reports, in listing order.
        notices: Vec<NoticeView>,
        /// The name of the tone the plugin loads onto a Part that holds
        /// another tone type; none for the Studio Set plugin.
        #[serde(rename = "defaultTone")]
        default_tone: Option<&'static str>,
        /// A request for the default tone is pending: the page disables its
        /// button until the Part holds the tone or the request is dropped.
        #[serde(rename = "defaultTonePending")]
        default_tone_pending: bool,
    },
    /// The plugin refused a request about the instance: a `bind` its kind
    /// cannot have, or a host gesture out of order or on a parameter it
    /// lacks. Not about the device link, so not an `error`.
    Refused {
        /// Why, as the page shows it.
        message: String,
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
    fn decodes_the_binding_and_gesture_messages() {
        // As `plugin.test.ts` encodes them.
        assert_eq!(
            decode(r#"{"type":"bind","part":3,"key":36}"#).unwrap(),
            PageMessage::Bind {
                part: Some(3),
                key: Some(36)
            }
        );
        assert_eq!(
            decode(r#"{"type":"bind","part":3}"#).unwrap(),
            PageMessage::Bind {
                part: Some(3),
                key: None
            }
        );
        assert_eq!(
            decode(r#"{"type":"bind"}"#).unwrap(),
            PageMessage::Bind {
                part: None,
                key: None
            }
        );
        assert_eq!(
            decode(r#"{"type":"beginEdit","id":"part07/0x09"}"#).unwrap(),
            PageMessage::BeginEdit {
                id: "part07/0x09".into()
            }
        );
        assert_eq!(
            decode(r#"{"type":"performEdit","id":"part07/0x09","value":-12}"#).unwrap(),
            PageMessage::PerformEdit {
                id: "part07/0x09".into(),
                value: -12
            }
        );
        assert_eq!(
            decode(r#"{"type":"endEdit","id":"part07/0x09"}"#).unwrap(),
            PageMessage::EndEdit {
                id: "part07/0x09".into()
            }
        );
        assert_eq!(
            decode(r#"{"type":"loadDefaultTone"}"#).unwrap(),
            PageMessage::LoadDefaultTone
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
            r#"{"type":"bind","part":256}"#,
            r#"{"type":"bind","part":"1"}"#,
            r#"{"type":"beginEdit"}"#,
            r#"{"type":"performEdit","id":"a"}"#,
            r#"{"type":"performEdit","id":"a","value":1.5}"#,
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
    fn encodes_a_refusal_apart_from_the_links_errors() {
        assert_eq!(
            encode(&PluginMessage::Refused {
                message: "no".into()
            }),
            r#"{"type":"refused","message":"no"}"#
        );
    }

    #[test]
    fn encodes_the_instance_message() {
        // As `plugin.test.ts` decodes it.
        let json = |message: &PluginMessage| -> serde_json::Value {
            serde_json::from_str(&encode(message)).expect("valid JSON")
        };
        assert_eq!(
            json(&PluginMessage::Instance {
                plugin: PluginKind::PcmDrum,
                binding: Some(Binding::of_part(2).with_key(36)),
                status: Status::DeviceOffline,
                status_text: Status::DeviceOffline.text(),
                host_edits: true,
                notices: vec![NoticeView {
                    notice: Notice::Recalling,
                    text: Notice::Recalling.text()
                }],
                default_tone: Some("Studio Kit"),
                default_tone_pending: true,
            }),
            serde_json::json!({
                "type": "instance",
                "plugin": "pcm_drum",
                "binding": { "part": 2, "key": 36 },
                "status": "deviceOffline",
                "statusText": "Device offline",
                "hostEdits": true,
                "notices": [{ "notice": "recalling", "text": Notice::Recalling.text() }],
                "defaultTone": "Studio Kit",
                "defaultTonePending": true,
            })
        );
        assert_eq!(
            encode(&PluginMessage::Instance {
                plugin: PluginKind::StudioSet,
                binding: None,
                status: Status::ServiceNotRunning,
                status_text: Status::ServiceNotRunning.text(),
                host_edits: false,
                notices: vec![],
                default_tone: None,
                default_tone_pending: false,
            }),
            r#"{"type":"instance","plugin":"studio_set","binding":null,"status":"serviceNotRunning","statusText":"Device service not running","hostEdits":false,"notices":[],"defaultTone":null,"defaultTonePending":false}"#
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
