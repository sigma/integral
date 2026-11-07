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
use vst3_host::{
    ClassInfo, ParameterEdit, Plugin, Vst3Host, get_detailed_plugin_info, simple,
    window::PluginWindow,
};

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

/// Loads the VST3 bundle at `bundle` (no editor yet): the class named
/// `class` if given, else the bundle's first.
pub fn load(bundle: &Path, class: Option<&str>) -> Result<Plugin> {
    let Some(name) = class else {
        return simple::load_plugin(bundle)
            .map_err(|e| anyhow!("loading {}: {e}", bundle.display()));
    };
    let found = classes(bundle)?
        .into_iter()
        .find(|c| c.name == name)
        .ok_or_else(|| anyhow!("{} has no plugin class named {name:?}", bundle.display()))?;
    load_class(bundle, &found.class_id)
}

/// The audio plugin classes the bundle's factory exports, in factory order.
pub fn classes(bundle: &Path) -> Result<Vec<ClassInfo>> {
    let info = get_detailed_plugin_info(bundle)
        .map_err(|e| anyhow!("reading {}: {e}", bundle.display()))?;
    Ok(info
        .classes
        .into_iter()
        .filter(|c| c.category == AUDIO_CLASS_CATEGORY)
        .collect())
}

/// Loads the class with the hex ID `class_id` from the bundle (no editor).
pub fn load_class(bundle: &Path, class_id: &str) -> Result<Plugin> {
    let mut host = Vst3Host::builder()
        .sample_rate(SAMPLE_RATE)
        .block_size(BLOCK_SIZE)
        .input_channels(2)
        .output_channels(2)
        .build()
        .map_err(|e| anyhow!("creating the host: {e}"))?;
    host.load_plugin_class(bundle, class_id)
        .map_err(|e| anyhow!("loading class {class_id} of {}: {e}", bundle.display()))
}

/// The category the VST3 SDK gives a factory's audio component classes.
const AUDIO_CLASS_CATEGORY: &str = "Audio Module Class";

/// What `vst3_host::simple::load_plugin` loads with.
const SAMPLE_RATE: f64 = 44100.0;
const BLOCK_SIZE: usize = 512;

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

/// Refuses to go on if a Device service is listening at the default socket,
/// unless `allow` is set.
///
/// Loading a plugin activates it, and an activated instance connects to the
/// user's service as a client (and, from the recall slice on, pushes state
/// to the device). The socket path does not come from the environment, so a
/// run cannot be pointed elsewhere: it has to be kept away from a live one.
pub fn refuse_live_service(allow: bool) -> Result<()> {
    // No path means no service could be listening.
    match integral_device_service::default_socket_path() {
        Ok(path) => refuse_service_at(&path, allow),
        Err(_) => Ok(()),
    }
}

/// [`refuse_live_service`] for the socket at `path`. A refused or missing
/// socket is a service that is not there (a stale file).
fn refuse_service_at(path: &Path, allow: bool) -> Result<()> {
    if !allow && std::os::unix::net::UnixStream::connect(path).is_ok() {
        return Err(anyhow!(
            "a Device service is running at {}: the plugins would connect to it as clients \
             and could write to the device. Stop it, or pass --allow-service",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::refuse_service_at;
    use std::os::unix::net::UnixListener;

    #[test]
    fn a_listening_socket_is_refused_unless_allowed_and_a_stale_one_is_not() {
        let dir = std::env::temp_dir().join(format!("ph{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("s.sock");
        assert!(refuse_service_at(&path, false).is_ok(), "no socket");
        let listener = UnixListener::bind(&path).unwrap();
        let error = refuse_service_at(&path, false).unwrap_err().to_string();
        assert!(error.contains("--allow-service"), "{error}");
        assert!(refuse_service_at(&path, true).is_ok());
        drop(listener);
        assert!(refuse_service_at(&path, false).is_ok(), "stale socket file");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
