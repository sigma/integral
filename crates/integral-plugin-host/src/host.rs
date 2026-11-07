//! The test host proper: an AppKit application on the main thread that
//! shows the plugin's editor and records the edits it reports.
//!
//! `vst3-host` does not run the AppKit loop (`service_platform_events` only
//! handles resizes), so this pumps `nextEventMatchingMask`/`sendEvent`
//! itself, as the #444 spike showed is needed for the webview to draw and
//! take input.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSEventMask};
use objc2_foundation::{NSDate, NSDefaultRunLoopMode};
use vst3_host::{ParameterEdit, Plugin, simple, window::PluginWindow};

/// State the main thread publishes for the driver thread, which must not
/// touch AppKit itself.
#[derive(Default)]
pub struct Shared {
    /// The application is the active (frontmost) one; refreshed every pump.
    active: AtomicBool,
    /// The driver is waiting for the application to become frontmost: the
    /// main loop keeps asking while it is not.
    want_active: AtomicBool,
    /// The main loop has ended: stop driving.
    cancelled: AtomicBool,
}

impl Shared {
    /// Whether the application was active at the last pump.
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    /// Whether the main loop has ended (editor closed, error, shutdown).
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Asks the main loop to keep requesting activation until it is active.
    pub fn request_activation(&self) {
        self.want_active.store(true, Ordering::Relaxed);
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

/// A loaded plugin with its editor open.
pub struct Host {
    app: Retained<NSApplication>,
    plugin: Arc<Mutex<Plugin>>,
    window: PluginWindow,
    shared: Arc<Shared>,
}

/// Loads the VST3 bundle at `bundle` (no editor yet).
pub fn load(bundle: &Path) -> Result<Plugin> {
    simple::load_plugin(bundle).map_err(|e| anyhow!("loading {}: {e}", bundle.display()))
}

impl Host {
    /// Starts the AppKit application and opens `plugin`'s editor.
    ///
    /// Must be called on the main thread.
    pub fn open(plugin: Plugin) -> Result<Self> {
        let mtm = MainThreadMarker::new().context("the host must run on the main thread")?;
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
        app.finishLaunching();
        // Activate while launching: a later request is more likely to be
        // refused (macOS's cooperative activation), and an inactive app
        // exposes no windows to Accessibility.
        #[allow(deprecated)]
        app.activateIgnoringOtherApps(true);

        let plugin = Arc::new(Mutex::new(plugin));
        let mut window = PluginWindow::new(plugin.clone());
        window
            .open()
            .map_err(|e| anyhow!("opening the editor: {e}"))?;
        Ok(Self {
            app,
            plugin,
            window,
            shared: Arc::new(Shared::default()),
        })
    }

    /// What the driver thread may read.
    pub fn shared(&self) -> Arc<Shared> {
        Arc::clone(&self.shared)
    }

    /// The window-server id of the editor's window.
    ///
    /// Found by its `"<plugin name> - VST3"` title among the application's
    /// windows, else the key window; the second value says which (`"title"`
    /// or `"key"`). Main thread only.
    pub fn editor_window_id(&self) -> Result<(u32, &'static str)> {
        let title = format!("{} - VST3", self.plugin().info().name);
        let windows = self.app.windows();
        let (window, via) = match windows.iter().find(|w| w.title().to_string() == title) {
            Some(window) => (window, "title"),
            None => (
                self.app
                    .keyWindow()
                    .ok_or_else(|| anyhow!("no editor window titled {title:?}"))?,
                "key",
            ),
        };
        let id = u32::try_from(window.windowNumber())
            .context("the editor window has no window number")?;
        Ok((id, via))
    }

    /// Brings the application to the front. Pointer events go to the
    /// frontmost app, and a CLI process is not frontmost by default. The
    /// request can be refused, so the driver checks [`Shared::is_active`].
    pub fn activate(&self) {
        #[allow(deprecated)]
        self.app.activateIgnoringOtherApps(true);
    }

    /// The plugin, for reads between pumps.
    pub fn plugin(&self) -> MutexGuard<'_, Plugin> {
        self.plugin.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Runs the AppKit loop while `keep_going` holds, then `settle` longer,
    /// passing every edit the editor reports to `on_edit`.
    ///
    /// When it ends with an error the driver is cancelled.
    pub fn run(
        &self,
        keep_going: impl FnMut() -> bool,
        settle: Duration,
        on_edit: impl FnMut(ParameterEdit),
    ) -> Result<()> {
        let result = self.pump(keep_going, settle, on_edit);
        if result.is_err() {
            self.shared.cancel();
        }
        result
    }

    fn pump(
        &self,
        mut keep_going: impl FnMut() -> bool,
        settle: Duration,
        mut on_edit: impl FnMut(ParameterEdit),
    ) -> Result<()> {
        let mut settle_until: Option<Instant> = None;
        loop {
            if self.window.closed_by_user() {
                return Err(anyhow!("the editor window was closed"));
            }
            self.pump_events();
            let active = self.app.isActive();
            self.shared.active.store(active, Ordering::Relaxed);
            if !active && self.shared.want_active.load(Ordering::Relaxed) {
                self.activate();
            }
            self.window
                .service_platform_events()
                .map_err(|e| anyhow!("servicing window events: {e}"))?;
            // The mutex is only contended by the window's own callbacks.
            if let Ok(mut plugin) = self.plugin.try_lock() {
                for edit in plugin.take_parameter_edits() {
                    on_edit(edit);
                }
            }
            if !keep_going() {
                let until = *settle_until.get_or_insert_with(|| Instant::now() + settle);
                if Instant::now() >= until {
                    return Ok(());
                }
            }
        }
    }

    fn pump_events(&self) {
        loop {
            let until = NSDate::dateWithTimeIntervalSinceNow(0.005);
            // SAFETY: main thread, standard mode, dequeueing.
            let event = unsafe {
                self.app.nextEventMatchingMask_untilDate_inMode_dequeue(
                    NSEventMask::Any,
                    Some(&until),
                    NSDefaultRunLoopMode,
                    true,
                )
            };
            match event {
                Some(event) => self.app.sendEvent(&event),
                None => break,
            }
        }
    }

    /// Closes the editor.
    pub fn close(&mut self) {
        self.shared.cancel();
        self.window.close();
    }
}
