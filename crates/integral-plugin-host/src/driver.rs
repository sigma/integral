//! What every scenario shares: finding a control through Accessibility,
//! trusting its frame only once it is stable, dragging it with real pointer
//! events that are re-checked against the window server, screenshots, and
//! the single closing `result` line.
//!
//! The driver runs on its own thread while the main thread keeps AppKit
//! alive (see [`crate::host`]).

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::thread::{self, sleep};
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow, bail};
use objc2_core_foundation::{CGPoint, CGRect};
use serde_json::{Value, json};
use vst3_host::{ParameterEdit, ParameterEditKind, Plugin};

use crate::ax::{self, Element, Pick};
use crate::cgwindow::{self, WindowInfo};
use crate::host::{Host, Shared};
use crate::pointer;
use crate::report::Report;
use crate::screenshot;

/// How long to wait for the window, the control and a stable frame.
const WAIT: Duration = Duration::from_secs(20);
/// How long to wait for the application to become frontmost.
const ACTIVATION_WAIT: Duration = Duration::from_secs(3);
/// Frame reads that must agree before the element counts as laid out.
const STABLE_READS: usize = 3;
const STABLE_INTERVAL: Duration = Duration::from_millis(250);
const DRAG_STEPS: u32 = 20;

/// Exit status of a run whose assertions failed.
pub const EXIT_ASSERTION: u8 = 1;
/// Exit status of a run that aborted (guard, timeout, setup error).
pub const EXIT_ABORT: u8 = 3;

/// A rectangle as JSON.
pub fn rect(r: CGRect) -> Value {
    json!({ "x": r.origin.x, "y": r.origin.y, "w": r.size.width, "h": r.size.height })
}

/// The `IComponentHandler` call an edit corresponds to.
fn kind_name(kind: &ParameterEditKind) -> &'static str {
    match kind {
        ParameterEditKind::BeginGesture => "beginEdit",
        ParameterEditKind::ValueChange => "performEdit",
        ParameterEditKind::EndGesture => "endEdit",
    }
}

/// Reports an edit as a `gesture` line and appends it to `edits`.
pub fn record_edit(report: &Report, edits: &mut Vec<ParameterEdit>, edit: ParameterEdit) {
    report.emit(
        "gesture",
        json!({ "id": edit.id, "kind": kind_name(&edit.kind), "value": edit.value }),
    );
    edits.push(edit);
}

/// Closes a run: emits the one `result` line and picks the exit status.
///
/// `Ok(true)` passed, `Ok(false)` failed an assertion, `Err` aborted.
pub fn conclude(report: &Report, scenario: &str, outcome: &Result<bool>) -> ExitCode {
    match outcome {
        Ok(ok) => {
            report.emit("result", json!({ "scenario": scenario, "ok": ok }));
            if *ok {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(EXIT_ASSERTION)
            }
        }
        Err(error) => {
            report.emit(
                "result",
                json!({ "scenario": scenario, "ok": false, "aborted": format!("{error:#}") }),
            );
            ExitCode::from(EXIT_ABORT)
        }
    }
}

/// Polls `f` until it yields a value or [`WAIT`] runs out.
pub fn wait_for<T>(what: &str, mut f: impl FnMut() -> Option<T>) -> Result<T> {
    let deadline = Instant::now() + WAIT;
    loop {
        if let Some(value) = f() {
            return Ok(value);
        }
        if Instant::now() >= deadline {
            bail!("timed out waiting for {what}");
        }
        sleep(Duration::from_millis(250));
    }
}

/// What a drag looked like from the driver's side.
pub struct Driven {
    /// The editor window as the window server saw it.
    pub window: WindowInfo,
    /// The dragged control's frame.
    pub frame: CGRect,
}

/// A driver bound to the editor window of this process.
pub struct Session {
    /// The host's own process id.
    pub pid: i32,
    /// The editor's window-server id, taken from AppKit on the main thread.
    pub window_id: u32,
    /// How the id was found: `"title"` or `"key"`.
    pub via: &'static str,
    /// Main-thread state: active and cancelled flags.
    pub shared: Arc<Shared>,
    /// Where events go.
    pub report: Arc<Report>,
    /// Where screenshots go.
    pub out_dir: PathBuf,
}

impl Session {
    /// Whether a pointer event at `point` would reach the editor now: the
    /// run is live, the application is frontmost, and the topmost visible
    /// window at `point`, over every layer, is the editor. Says why not.
    fn safe(&self, point: CGPoint) -> bool {
        let blocked = if self.shared.is_cancelled() {
            json!({ "why": "the run ended" })
        } else if !self.shared.is_active() {
            json!({ "why": "the host application is not frontmost" })
        } else {
            match cgwindow::topmost_at(point) {
                Some(w) if w.pid == self.pid && w.id == self.window_id => return true,
                Some(w) => json!({
                    "why": "another window is on top",
                    "owner": w.owner, "layer": w.layer, "window": w.id, "pid": w.pid,
                }),
                None => json!({ "why": "no window at the point" }),
            }
        };
        self.report.emit(
            "blocked",
            json!({ "point": { "x": point.x, "y": point.y }, "reason": blocked }),
        );
        false
    }

    fn window(&self) -> Option<WindowInfo> {
        cgwindow::window(self.window_id)
    }

    /// Finds the one slider labelled `control`, once [`STABLE_READS`] reads
    /// in a row agree on the same pick among the same number of sliders
    /// (the page is still building its tree before that).
    fn locate(&self, control: &str) -> Result<Element> {
        let app = Element::application(self.pid)
            .ok_or_else(|| anyhow!("no accessibility element for pid {}", self.pid))?;
        let deadline = Instant::now() + WAIT;
        let mut last: Option<(usize, Vec<String>)> = None;
        let mut agreeing = 0;
        loop {
            let candidates: Vec<(Element, Vec<String>)> = app
                .descendants(ax::SLIDER_ROLE)
                .into_iter()
                .map(|e| {
                    let labels = e.labels();
                    (e, labels)
                })
                .collect();
            let count = candidates.len();
            match ax::pick(candidates, control) {
                Pick::Ambiguous(n) => bail!("{n} sliders match {control:?}; give an exact label"),
                Pick::One(element) => {
                    let seen = (count, element.labels());
                    if last.as_ref() == Some(&seen) {
                        agreeing += 1;
                    } else {
                        agreeing = 1;
                        last = Some(seen);
                    }
                    if agreeing >= STABLE_READS {
                        return Ok(element);
                    }
                }
                Pick::None => {
                    agreeing = 0;
                    last = None;
                }
            }
            if Instant::now() >= deadline {
                bail!("timed out waiting for a stable slider labelled {control:?}");
            }
            sleep(STABLE_INTERVAL);
        }
    }

    /// Reads `element`'s frame until [`STABLE_READS`] in a row agree and lie
    /// inside the editor window; aborts otherwise.
    ///
    /// WebKit reports stale frames until it has laid the page out. An early
    /// click once landed in another application's window (#444), so this
    /// guard is not optional.
    fn stable_frame(&self, element: &Element) -> Result<(WindowInfo, CGRect)> {
        let deadline = Instant::now() + WAIT;
        let mut last: Option<CGRect> = None;
        let mut agreeing = 0;
        while Instant::now() < deadline {
            sleep(STABLE_INTERVAL);
            let (Some(frame), Some(window)) = (element.frame(), self.window()) else {
                agreeing = 0;
                continue;
            };
            if last == Some(frame) {
                agreeing += 1;
            } else {
                agreeing = 1;
                last = Some(frame);
            }
            if agreeing >= STABLE_READS && cgwindow::contains(window.bounds, frame) {
                self.report.emit(
                    "frame",
                    json!({ "stable": true, "element": rect(frame), "window": rect(window.bounds) }),
                );
                return Ok((window, frame));
            }
        }
        self.report
            .emit("frame", json!({ "stable": false, "last": last.map(rect) }));
        bail!("the control's frame never became stable inside the plugin's own window")
    }

    fn shot(&self, name: &str) {
        let path = self.out_dir.join(name);
        let ok = screenshot::window(self.window_id, &path);
        self.report.emit(
            "screenshot",
            json!({ "window": self.window_id, "path": path.display().to_string(), "ok": ok }),
        );
    }

    /// The editor's window once it shows. Needs the Accessibility grant.
    fn editor_window(&self) -> Result<WindowInfo> {
        if !ax::trusted() {
            bail!(
                "Accessibility is not granted to this process (macOS attributes it to the \
                 terminal that launched it); grant it in System Settings > Privacy & Security > \
                 Accessibility"
            );
        }
        let window = wait_for("the plugin window", || self.window())?;
        self.report.emit(
            "window",
            json!({ "id": window.id, "via": self.via, "frame": rect(window.bounds) }),
        );
        Ok(window)
    }

    /// Waits for text containing `needle` to show in the editor, and takes a
    /// screenshot called `shot` either way. Whether it showed within
    /// [`WAIT`]: that it did not is a finding, not an abort.
    pub fn wait_for_text(&self, needle: &str, shot: &str) -> Result<bool> {
        self.editor_window()?;
        let app = Element::application(self.pid)
            .ok_or_else(|| anyhow!("no accessibility element for pid {}", self.pid))?;
        let deadline = Instant::now() + WAIT;
        let shown = loop {
            let found = app.descendants_where(|e| {
                e.role().as_deref() == Some(ax::STATIC_TEXT_ROLE)
                    && e.texts().iter().any(|t| t.contains(needle))
            });
            if !found.is_empty() {
                break found.len();
            }
            if Instant::now() >= deadline {
                break 0;
            }
            sleep(Duration::from_millis(250));
        };
        self.report
            .emit("text", json!({ "contains": needle, "elements": shown }));
        self.shot(shot);
        Ok(shown > 0)
    }

    /// Finds the slider labelled `control` and drags it from its centre to
    /// `to` (a fraction of its width), with screenshots before and after.
    pub fn drag_slider(&self, control: &str, to: f64) -> Result<Driven> {
        self.editor_window()?;
        let slider = self.locate(control)?;
        self.report.emit(
            "control",
            json!({ "labels": slider.labels(), "value": slider.number() }),
        );
        let (window, frame) = self.stable_frame(&slider)?;
        self.shot("drag-before.png");

        // Re-check right before the events leave: layout can still move.
        if slider.frame() != Some(frame) {
            bail!("the control moved before the drag");
        }
        let y = frame.origin.y + frame.size.height / 2.0;
        let from = CGPoint::new(frame.origin.x + frame.size.width / 2.0, y);
        let end = CGPoint::new(frame.origin.x + frame.size.width * to, y);
        if !cgwindow::contains_point(window.bounds, end) {
            bail!("the drag's end point is outside the plugin window");
        }
        self.shared.request_activation();
        let deadline = Instant::now() + ACTIVATION_WAIT;
        while !self.shared.is_active() && Instant::now() < deadline {
            sleep(Duration::from_millis(50));
        }
        if !self.shared.is_active() {
            bail!("the host application is not frontmost (activation was refused)");
        }
        pointer::drag(from, end, DRAG_STEPS, |p| self.safe(p)).map_err(|e| anyhow!("{e}"))?;
        sleep(Duration::from_millis(300));
        self.report
            .emit("drag", json!({ "to": to, "ax_value": slider.number() }));
        self.shot("drag-after.png");
        Ok(Driven { window, frame })
    }
}

/// Opens `plugin`'s editor in this process and brings it to the front: what
/// every scenario that drives the editor starts with. Edits the plugin
/// reports while loading are recorded in `edits`, so a spurious one fails
/// the run.
pub fn open_editor(
    plugin: Plugin,
    report: &Arc<Report>,
    out_dir: &Path,
    edits: &mut Vec<ParameterEdit>,
) -> Result<(Host, Session)> {
    std::fs::create_dir_all(out_dir)?;
    let host = Host::open(plugin)?;
    // Let the window show and come to the front.
    for _ in 0..2 {
        let until = Instant::now() + Duration::from_millis(500);
        host.run(
            || Instant::now() < until,
            Duration::ZERO,
            |e| record_edit(report, edits, e),
        )?;
        host.activate();
    }
    let (window_id, via) = host.editor_window_id()?;
    let session = Session {
        pid: i32::try_from(std::process::id())?,
        window_id,
        via,
        shared: host.shared(),
        report: Arc::clone(report),
        out_dir: out_dir.to_owned(),
    };
    Ok((host, session))
}

/// Runs `work` on its own thread (it may use AX, which must not run on the
/// thread pumping AppKit) while the main thread pumps `host`, recording
/// edits. Returns how the pump ended and what `work` returned.
pub fn drive<T: Send + 'static>(
    host: &Host,
    report: &Report,
    edits: &mut Vec<ParameterEdit>,
    work: impl FnOnce() -> Result<T> + Send + 'static,
) -> (Result<()>, Result<T>) {
    let worker = thread::spawn(work);
    let pumped = host.run(
        || !worker.is_finished(),
        Duration::from_millis(600),
        |e| record_edit(report, edits, e),
    );
    let driven = worker
        .join()
        .map_err(|_| anyhow!("the driver panicked"))
        .and_then(|r| r);
    (pumped, driven)
}
