//! The webview editor: a wry webview in a baseview child window (ADR 0004).
//!
//! macOS only for now: the parent is an AppKit view. Windows needs a
//! WebView2 user data folder outside the host's directory first (PRD #349
//! leaves that out of scope).

use std::any::Any;
use std::borrow::Cow;
use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use nih_plug::prelude::{Editor, GuiContext, ParentWindowHandle, nih_error};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use wry::dpi::{LogicalPosition, LogicalSize};
use wry::http::{Response, header};
use wry::raw_window_handle as rwh;
use wry::{Rect, WebView, WebViewBuilder};

use super::assets;
use super::bridge::Bridge;
use super::ipc;
use super::link::DeviceLink;

/// Makes the link each editor opens the device through.
pub(crate) type LinkFactory = Arc<dyn Fn() -> Box<dyn DeviceLink> + Send + Sync>;

/// How often the editor re-lists the link's ports for the page.
const PORT_POLL: Duration = Duration::from_secs(1);

/// The page's background, shown until it paints (`index.html`'s).
const BACKGROUND: (u8, u8, u8, u8) = (0x0f, 0x0f, 0x1a, 0xff);

/// A nih-plug [`Editor`] that runs the web UI.
pub(crate) struct WebviewEditor {
    /// The window size in logical px, persisted with the plugin's state.
    size: Arc<RwLock<(u32, u32)>>,
    link: LinkFactory,
}

impl WebviewEditor {
    /// An editor of the persisted `size` whose pages reach the device
    /// through links made by `link`.
    pub(crate) fn new(size: Arc<RwLock<(u32, u32)>>, link: LinkFactory) -> Self {
        Self { size, link }
    }
}

impl Editor for WebviewEditor {
    fn spawn(
        &self,
        parent: ParentWindowHandle,
        context: Arc<dyn GuiContext>,
    ) -> Box<dyn Any + Send> {
        let (width, height) = *self.size.read().expect("size lock");
        let options = baseview::WindowOpenOptions {
            title: "Integral".into(),
            size: baseview::Size::new(width.into(), height.into()),
            scale: baseview::WindowScalePolicy::SystemScaleFactor,
            gl_config: None,
        };
        let size = self.size.clone();
        let link = self.link.clone();
        let window = baseview::Window::open_parented(&parent, options, move |_| View {
            size,
            context,
            link,
            page: Page::Pending,
            next_poll: Instant::now(),
        });
        Box::new(EditorHandle(window))
    }

    fn size(&self) -> (u32, u32) {
        *self.size.read().expect("size lock")
    }

    fn set_scale_factor(&self, _factor: f32) -> bool {
        // nih-plug never calls this on macOS, where sizes are logical.
        false
    }

    fn param_value_changed(&self, _id: &str, _normalized_value: f32) {}

    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {}

    fn param_values_changed(&self) {}
}

/// The open editor window; dropping it closes the window, the page and its
/// link.
struct EditorHandle(baseview::WindowHandle);

// SAFETY: hosts create and destroy editors on the GUI thread, so the window
// handle is only touched there; nih-plug only requires `Send` to store it.
unsafe impl Send for EditorHandle {}

impl Drop for EditorHandle {
    fn drop(&mut self) {
        self.0.close();
    }
}

/// The page's state in the window.
enum Page {
    /// Not built yet: the host's view isn't in a window until after spawn.
    Pending,
    /// Up, with the bridge its IPC handler shares.
    Up {
        webview: WebView,
        bridge: Rc<RefCell<Bridge>>,
    },
    /// The webview failed to build; logged, not retried.
    Failed,
}

/// The baseview window handler that owns the webview.
struct View {
    size: Arc<RwLock<(u32, u32)>>,
    context: Arc<dyn GuiContext>,
    link: LinkFactory,
    page: Page,
    next_poll: Instant,
}

/// The webview's bounds for a window of `width` × `height` logical px.
fn bounds(width: f64, height: f64) -> Rect {
    Rect {
        position: LogicalPosition::new(0.0, 0.0).into(),
        size: LogicalSize::new(width, height).into(),
    }
}

/// The response to a request for `path` on the custom protocol.
fn respond(path: &str) -> Response<Cow<'static, [u8]>> {
    let served = assets::serve(path);
    Response::builder()
        .status(served.status)
        .header(header::CONTENT_TYPE, served.mime)
        .header(header::CONTENT_SECURITY_POLICY, assets::CSP)
        .body(Cow::Borrowed(served.body))
        .expect("a status, two static headers and a body make a response")
}

/// The baseview window's AppKit view as a parent for wry, which takes
/// raw-window-handle 0.6 where baseview gives 0.5.
struct AppKitParent(NonNull<c_void>);

impl rwh::HasWindowHandle for AppKitParent {
    fn window_handle(&self) -> Result<rwh::WindowHandle<'_>, rwh::HandleError> {
        let raw = rwh::RawWindowHandle::AppKit(rwh::AppKitWindowHandle::new(self.0));
        // SAFETY: the view is baseview's, alive for as long as the window
        // that owns this handler, which outlives the webview built on it.
        Ok(unsafe { rwh::WindowHandle::borrow_raw(raw) })
    }
}

impl View {
    /// Build the webview into `window`. Called on the first frame: wry
    /// panics building into a view that isn't in a window yet, as the
    /// host's view is when the editor spawns.
    fn build(&mut self, window: &mut baseview::Window) -> Page {
        let RawWindowHandle::AppKit(handle) = window.raw_window_handle() else {
            nih_error!("the webview editor needs an AppKit window");
            return Page::Failed;
        };
        let Some(view) = NonNull::new(handle.ns_view) else {
            nih_error!("baseview gave no NSView");
            return Page::Failed;
        };
        let bridge = Rc::new(RefCell::new(Bridge::new((self.link)())));
        let ipc_bridge = bridge.clone();
        let (width, height) = *self.size.read().expect("size lock");
        let built = WebViewBuilder::new()
            .with_bounds(bounds(width.into(), height.into()))
            .with_background_color(BACKGROUND)
            .with_accept_first_mouse(true)
            .with_custom_protocol(assets::SCHEME.into(), |_, request| {
                respond(request.uri().path())
            })
            .with_ipc_handler(move |request| ipc_bridge.borrow_mut().handle(request.body()))
            .with_devtools(cfg!(debug_assertions))
            .with_url(assets::START_URL)
            .build_as_child(&AppKitParent(view));
        match built {
            Ok(webview) => Page::Up { webview, bridge },
            Err(e) => {
                nih_error!("could not build the webview: {e}");
                Page::Failed
            }
        }
    }

    /// Apply a window size the page asked for, if the host agrees.
    fn resize(&self, window: &mut baseview::Window, webview: &WebView, bridge: &RefCell<Bridge>) {
        let Some(requested) = bridge.borrow_mut().take_resize() else {
            return;
        };
        let previous = std::mem::replace(&mut *self.size.write().expect("size lock"), requested);
        if previous == requested {
            return;
        }
        // The host reads the new size from `Editor::size`.
        if self.context.request_resize() {
            let (width, height) = (requested.0.into(), requested.1.into());
            window.resize(baseview::Size::new(width, height));
            if let Err(e) = webview.set_bounds(bounds(width, height)) {
                nih_error!("could not resize the webview: {e}");
            }
        } else {
            *self.size.write().expect("size lock") = previous;
        }
    }
}

impl baseview::WindowHandler for View {
    fn on_frame(&mut self, window: &mut baseview::Window) {
        if matches!(self.page, Page::Pending) {
            self.page = self.build(window);
        }
        let Page::Up { webview, bridge } = &self.page else {
            return;
        };
        self.resize(window, webview, bridge);
        let now = Instant::now();
        if now >= self.next_poll {
            self.next_poll = now + PORT_POLL;
            bridge.borrow_mut().refresh_ports();
        }
        // Release the bridge before calling into the webview.
        let outgoing = bridge.borrow().take_outgoing();
        if let Some(script) = ipc::delivery_script(&outgoing)
            && let Err(e) = webview.evaluate_script(&script)
        {
            nih_error!("could not deliver to the page: {e}");
        }
    }

    fn on_event(
        &mut self,
        _window: &mut baseview::Window,
        event: baseview::Event,
    ) -> baseview::EventStatus {
        if let (
            baseview::Event::Window(baseview::WindowEvent::Resized(info)),
            Page::Up { webview, .. },
        ) = (&event, &self.page)
        {
            let size = info.logical_size();
            if let Err(e) = webview.set_bounds(bounds(size.width, size.height)) {
                nih_error!("could not resize the webview: {e}");
            }
        }
        baseview::EventStatus::Ignored
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_response_carries_its_type_and_the_csp() {
        for path in ["/", "/missing.js"] {
            let response = respond(path);
            let served = assets::serve(path);
            assert_eq!(response.status().as_u16(), served.status, "{path}");
            assert_eq!(response.headers()[header::CONTENT_TYPE], served.mime);
            assert_eq!(
                response.headers()[header::CONTENT_SECURITY_POLICY],
                assets::CSP
            );
            assert_eq!(response.body().as_ref(), served.body);
        }
    }
}
