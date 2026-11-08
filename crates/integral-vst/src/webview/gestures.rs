//! The page's host gestures, forwarded to the host.
//!
//! A DAW records automation only for a parameter whose edit is bracketed by
//! `begin` and `end`, so the editor's Control drags reach the host as
//! gestures on the parameter the Control's field has for the instance's
//! Binding (ADR 0008). The page names the parameter by its host ID and sends
//! the raw value.
//!
//! The gestures go through the instance's own [`HostEdits`]
//! ([`ParamBank::edits`](crate::family::bank::ParamBank::edits), the one
//! context adapter the follower uses too), on the host's main thread, where
//! the editor's IPC handler runs. [`family::host`](crate::family::host) keeps
//! them to one gesture per parameter and keeps the follower from writing a
//! drag's values back to the device.
//!
//! [`HostEdits`]: crate::family::host::HostEdits

use thiserror::Error;

use crate::family::{Instance, host};

/// Why a gesture was refused.
#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum EditRefused {
    /// An ID that names no host parameter of the plugin.
    #[error("no host parameter {0:?}")]
    UnknownParam(String),
    /// The instance has no way to tell the host (no context was given to it).
    #[error("the host gives this plugin no way to record edits")]
    NoHost,
}

/// A gesture the page has begun on one parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Gesture {
    /// The host ID.
    pub(crate) id: String,
    /// The position in the instance's list.
    index: usize,
    /// The value the gesture last took.
    last: Option<i32>,
}

/// Begin a gesture on the parameter `id` of `instance`.
pub(crate) fn begin(instance: &Instance, id: &str) -> Result<Gesture, EditRefused> {
    let index = instance
        .bank
        .position(id)
        .ok_or_else(|| EditRefused::UnknownParam(id.into()))?;
    if !host::editor_begin(&instance.bank, index) {
        return Err(EditRefused::NoHost);
    }
    Ok(Gesture {
        id: id.into(),
        index,
        last: None,
    })
}

/// Move the parameter of `gesture` to the raw `value`, held to its range.
pub(crate) fn perform(instance: &Instance, gesture: &mut Gesture, value: i32) {
    let host_param = &instance.bank.list()[gesture.index].0;
    let value = value.clamp(host_param.min, host_param.max);
    gesture.last = Some(value);
    host::editor_perform(&instance.bank, gesture.index, value);
}

/// End `gesture`.
pub(crate) fn end(instance: &Instance, gesture: Gesture) {
    host::editor_end(&instance.bank, gesture.index, gesture.last);
}
