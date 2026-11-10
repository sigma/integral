//! How the device's side changes an instance's host parameters.
//!
//! # What nih-plug allows (rev `28b149e`, VST3 wrapper)
//!
//! A plugin cannot set a parameter, or tell the host that one changed, except
//! through a [`GuiContext`]; every setter is `pub(crate)`. Upstream creates a
//! context only for [`Editor::spawn`], so a plugin with its editor closed has
//! none. The vendored nih-plug (`crates/patches/PATCHES.md`) adds
//! `InitContext::gui_context()`: the instance keeps that context from
//! `initialize()`, and the device's changes reach the host whether or not an
//! editor is open. (Another wrapper, or an unpatched one, gives `None`, and
//! then there is no way in: the host's own changes still go out.)
//!
//! - **A gesture** (`beginEdit`, `performEdit`, `endEdit`) is a [`ParamSetter`]
//!   over that context. While the host is processing audio the wrapper does not
//!   touch the value itself; the host passes it back in the next audio
//!   callback.
//! - **A silent bulk update with one `kParamValuesChanged`** is
//!   [`GuiContext::set_state`] with a state holding only the changed values:
//!   the wrapper applies them (at the end of the current process cycle if
//!   audio is running) and schedules `restartComponent(kParamValuesChanged)`
//!   once. `performEdit` without a gesture would break the protocol. The
//!   restore also runs `initialize()` and `reset()` again, so `initialize()`
//!   must be idempotent (it is: it keeps the connection it has), and it calls
//!   `deserialize_fields` with no fields (see [`Instance::replacing`]).
//!
//! # Threads
//!
//! The context's calls are for the host's main thread (nih-plug says it takes
//! this on trust, and a debug build checks gestures). The instance's worker
//! makes none of them. It posts a [`HostTask`] ([`Instance::post`]) to the
//! vendored nih-plug's `InitContext::async_executor()`, which queues it on
//! the host's event loop; the plugin's task executor then runs [`run`] on the
//! main thread. That needs no `process()` call, keeps the order posted, and
//! holds the wrapper only weakly (the editor's `AsyncExecutor` holds it
//! strongly, so the plugin never keeps that one). Because the worker is never
//! the main thread, a task is never run inline on it: a bulk update waits for
//! the audio thread, and must not run on a thread that is the audio thread.
//! A bulk replace waits at most [`REPLACE_WITHIN`] for it (the patched
//! `try_set_state`), and is retried by the worker if the audio thread did not
//! take it. The event loop's queue holds 4096 tasks and refuses the rest;
//! the patched executor says so, and [`Instance::post`] returns `false`. The
//! follower posts a refused gesture's tasks and replaces again, and the
//! Binding's sync is queued again at the next tick.
//!
//! [`HostEdits`] is the seam: tests put a recorder in its place and observe
//! the gestures and bulk updates a host would see.

use std::collections::BTreeMap;
use std::sync::Arc;

use integral_core::host_params::HostParam;
use nih_plug::prelude::*;
use nih_plug::wrapper::state::ParamValue;

use super::bank::{Gesturer, ParamBank};
use super::instance::Instance;

/// How long a bulk replace waits for the audio thread to pick the state up
/// before it gives the values back (`set_state` would wait for ever).
pub const REPLACE_WITHIN: std::time::Duration = std::time::Duration::from_millis(50);

/// What the device's side does to the host's view of the parameters. Called on
/// the host's main thread.
pub trait HostEdits: Send + Sync {
    /// The host is told a gesture on `param` begins.
    fn begin(&self, param: &IntParam);
    /// `param` takes `value` within the gesture, and the host hears of it.
    fn perform(&self, param: &IntParam, value: i32);
    /// The gesture on `param` ends.
    fn end(&self, param: &IntParam);
    /// Replace many values with no gesture and tell the host once that
    /// values changed. Returns false if the host did not take them (the
    /// audio thread did not pick the state up in time): none is applied.
    fn replace(&self, changes: &[Replace<'_>]) -> bool;
}

/// One value of a [`HostEdits::replace`].
#[derive(Clone, Copy)]
pub struct Replace<'a> {
    /// The parameter in the list.
    pub host: &'a HostParam,
    /// Its nih-plug counterpart.
    pub param: &'a IntParam,
    /// The new raw value.
    pub value: i32,
}

/// Something for the host's main thread to do, posted by the instance's
/// worker (nih-plug's `BackgroundTask`). `Copy`, so a ring carries it without
/// allocating; a parameter is its position in the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostTask {
    /// Begin a gesture on a parameter.
    Begin(u32),
    /// Set a parameter within its gesture.
    Perform(u32, i32),
    /// End the gesture on a parameter.
    End(u32),
    /// Replace the values the bank has queued
    /// ([`ParamBank::queue_replace`](super::bank::ParamBank::queue_replace)).
    Replace,
    /// Set the Binding's host parameters to the instance's Binding, as one
    /// gesture each ([`Instance::follow_binding_params`]).
    SyncBinding,
}

/// Carry out `task` for `instance` through its [`HostEdits`], if it has any:
/// nih-plug's task executor, on the main thread.
pub fn run(instance: &Instance, task: HostTask) {
    let bank = &instance.bank;
    let edits = bank.edits();
    // The host sees one gesture per parameter. While the editor has it (a
    // Control drag), the follower's begin, performs and end for the same
    // parameter are left out: the drag is the edit, and the device's changes
    // reach the follower from the drag's own writes.
    match task {
        HostTask::Begin(i) => {
            if let (Some(edits), None) = (&edits, bank.gesturer(i as usize)) {
                edits.begin(&bank.list()[i as usize].1);
                bank.set_gesturer(i as usize, Some(Gesturer::Follower));
            }
        }
        HostTask::Perform(i, value) => {
            if let (Some(edits), Some(Gesturer::Follower)) = (&edits, bank.gesturer(i as usize)) {
                edits.perform(&bank.list()[i as usize].1, value);
            }
        }
        HostTask::End(i) => {
            if let (Some(edits), Some(Gesturer::Follower)) = (&edits, bank.gesturer(i as usize)) {
                edits.end(&bank.list()[i as usize].1);
                bank.set_gesturer(i as usize, None);
            }
            if bank.gesturer(i as usize) != Some(Gesturer::Editor) {
                bank.settle(i as usize);
            }
        }
        HostTask::SyncBinding => {
            let params = &instance.binding_params;
            params.sync_done();
            let (Some(edits), Some(binding)) = (&edits, instance.binding.get()) else {
                return;
            };
            // A host edit waiting to be bound wins over the Binding it
            // is about to replace.
            if params.edited() {
                return;
            }
            for (i, value) in params.differing(&binding) {
                let param = &params.list()[i].1;
                params.expect(i, value);
                edits.begin(param);
                edits.perform(param, value);
                edits.end(param);
            }
        }
        HostTask::Replace => {
            let changes = bank.take_replace();
            let mut taken = true;
            if let Some(edits) = &edits {
                let replace: Vec<Replace<'_>> = changes
                    .iter()
                    .map(|(i, value)| Replace {
                        host: &bank.list()[*i].0,
                        param: &bank.list()[*i].1,
                        value: *value,
                    })
                    .collect();
                // The wrapper restores a state through `deserialize_fields`,
                // which must not take this for the host's restore.
                instance.set_replacing(true);
                taken = edits.replace(&replace);
                instance.set_replacing(false);
            }
            if !taken {
                // The audio thread is not taking states (the host stopped
                // calling `process()` without saying so): give the values
                // back, for the worker to try again later. The main thread
                // is not kept waiting.
                bank.requeue_replace(changes);
                return;
            }
            // Nothing to wait for where the value is there already (or
            // cannot be set).
            for (i, value) in changes {
                if edits.is_none() || bank.value(i) == value {
                    bank.settle(i);
                }
            }
        }
    }
}

/// The editor's `context` becomes the way to the host when the instance has
/// none yet (the host opened the editor before activating the plugin).
/// Returns whether it was taken. The editor withdraws it when its window
/// closes ([`ParamBank::withdraw_editor_edits`]), since this context, unlike
/// the one kept from `initialize()`, is not held weakly; `initialize()`
/// replaces it. Main thread.
pub fn editor_context(bank: &ParamBank, context: Arc<dyn GuiContext>) -> bool {
    bank.set_editor_edits(Arc::new(GuiEdits::new(context)))
}

/// The editor begins a gesture on parameter `i`: a Control drag. If the
/// follower has the host's gesture on it (another client's edit), that one
/// ends first, so the host sees one gesture per parameter. Returns false if
/// the instance has no way to tell the host ([`ParamBank::edits`]).
/// Main thread.
pub fn editor_begin(bank: &ParamBank, i: usize) -> bool {
    let Some(edits) = bank.edits() else {
        return false;
    };
    let param = &bank.list()[i].1;
    match bank.gesturer(i) {
        Some(Gesturer::Editor) => return true,
        Some(Gesturer::Follower) => edits.end(param),
        None => {}
    }
    edits.begin(param);
    bank.set_gesturer(i, Some(Gesturer::Editor));
    true
}

/// The editor sets parameter `i` to `value` within its gesture. The page
/// writes the device itself, so the value is noted as one the device's side
/// gave ([`ParamBank::expect`]): the audio thread does not take it for the
/// host's edit and write it again. Main thread.
pub fn editor_perform(bank: &ParamBank, i: usize, value: i32) {
    if let (Some(edits), Some(Gesturer::Editor)) = (bank.edits(), bank.gesturer(i)) {
        bank.expect(i, value);
        edits.perform(&bank.list()[i].1, value);
    }
}

/// The editor ends its gesture on parameter `i`, which last took `last`. If
/// the host already holds that value it is settled; else the host's echo is
/// still to come and the bank recognises it. Main thread.
pub fn editor_end(bank: &ParamBank, i: usize, last: Option<i32>) {
    let (Some(edits), Some(Gesturer::Editor)) = (bank.edits(), bank.gesturer(i)) else {
        return;
    };
    edits.end(&bank.list()[i].1);
    bank.set_gesturer(i, None);
    if last.is_none_or(|last| bank.value(i) == last) {
        bank.settle(i);
    }
}

/// Tells the host through the [`GuiContext`] the plugin kept from
/// `initialize()`.
pub struct GuiEdits {
    context: Arc<dyn GuiContext>,
}

impl GuiEdits {
    /// Edits through `context`. The context refers to the wrapper weakly, so
    /// an instance holding it does not keep its own wrapper alive.
    pub fn new(context: Arc<dyn GuiContext>) -> Self {
        Self { context }
    }
}

impl HostEdits for GuiEdits {
    fn begin(&self, param: &IntParam) {
        ParamSetter::new(&*self.context).begin_set_parameter(param);
    }

    fn perform(&self, param: &IntParam, value: i32) {
        ParamSetter::new(&*self.context).set_parameter(param, value);
    }

    fn end(&self, param: &IntParam) {
        ParamSetter::new(&*self.context).end_set_parameter(param);
    }

    fn replace(&self, changes: &[Replace<'_>]) -> bool {
        // Only the changed values: the wrapper leaves every other parameter
        // as it is.
        let params = changes
            .iter()
            .map(|change| (change.host.id.clone(), ParamValue::I32(change.value)))
            .collect();
        self.context
            .try_set_state(
                PluginState {
                    version: String::new(),
                    params,
                    fields: BTreeMap::new(),
                },
                REPLACE_WITHIN,
            )
            .is_none()
    }
}
