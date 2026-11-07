//! A context passed during plugin initialization.

use std::sync::Arc;

use super::gui::{AsyncExecutor, GuiContext};
use super::PluginApi;
use crate::prelude::Plugin;

/// Callbacks the plugin can make while it is being initialized. This is passed to the plugin during
/// [`Plugin::initialize()`][crate::plugin::Plugin::initialize()].
//
// # Safety
//
// The implementing wrapper needs to be able to handle concurrent requests, and it should perform
// the actual callback within [MainThreadQueue::schedule_gui].
pub trait InitContext<P: Plugin> {
    /// Get the current plugin API.
    fn plugin_api(&self) -> PluginApi;

    /// Run a task directly on this thread. This ensures that the task has finished executing before
    /// the plugin finishes initializing.
    ///
    /// # Note
    ///
    /// There is no asynchronous alternative for this function as that may result in incorrect
    /// behavior when doing offline rendering.
    fn execute(&self, task: P::BackgroundTask);

    /// A [`GuiContext`] the plugin may keep, to set parameter values and tell the host about
    /// them (gestures, or a whole [`PluginState`][crate::prelude::PluginState]) without an
    /// editor. Its calls are meant for the host's main thread, so a plugin on another thread
    /// should hand them to [`ProcessContext::execute_gui()`][crate::prelude::ProcessContext::execute_gui()]'s
    /// task executor. It refers to the plugin weakly: keeping it does not keep the plugin alive,
    /// and once the plugin is gone its calls do nothing.
    ///
    /// Only the VST3 wrapper provides this so far; the others return `None`.
    ///
    /// *Patch (see `crates/patches/PATCHES.md` in Integral): not in upstream nih-plug.*
    fn gui_context(&self) -> Option<Arc<dyn GuiContext>> {
        None
    }

    /// An [`AsyncExecutor`] the plugin may keep and use from any thread, to have the host's main
    /// thread (or a background thread) run its tasks without going through `process()`. Like
    /// [`gui_context()`][Self::gui_context()] it refers to the plugin weakly: keeping it does not
    /// keep the plugin alive, and once the plugin is gone the tasks are dropped. Unlike the one
    /// passed to [`Plugin::editor()`][crate::prelude::Plugin::editor()], which holds the wrapper
    /// strongly.
    ///
    /// Only the VST3 wrapper provides this so far; the others return `None`.
    ///
    /// *Patch (see `crates/patches/PATCHES.md` in Integral): not in upstream nih-plug.*
    fn async_executor(&self) -> Option<AsyncExecutor<P>> {
        None
    }

    /// Update the current latency of the plugin. If the plugin is currently processing audio, then
    /// this may cause audio playback to be restarted.
    fn set_latency_samples(&self, samples: u32);

    /// Set the current voice **capacity** for this plugin (so not the number of currently active
    /// voices). This may only be called if
    /// [`ClapPlugin::CLAP_POLY_MODULATION_CONFIG`][crate::prelude::ClapPlugin::CLAP_POLY_MODULATION_CONFIG]
    /// is set. `capacity` must be between 1 and the configured maximum capacity. Changing this at
    /// runtime allows the host to better optimize polyphonic modulation, or to switch to strictly
    /// monophonic modulation when dropping the capacity down to 1.
    fn set_current_voice_capacity(&self, capacity: u32);
}
