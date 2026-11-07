//! Real pointer events through `CGEvent`, posted at the HID tap, so the
//! page sees them as a user's.

use std::thread::sleep;
use std::time::Duration;

use objc2_core_foundation::{CFRetained, CGPoint};
use objc2_core_graphics::{CGEvent, CGEventTapLocation, CGEventType, CGMouseButton};

fn event(kind: CGEventType, at: CGPoint) -> Result<CFRetained<CGEvent>, String> {
    CGEvent::new_mouse_event(None, kind, at, CGMouseButton::Left)
        .ok_or_else(|| format!("could not create a {kind:?} event"))
}

fn post(event: &CGEvent) {
    CGEvent::post(CGEventTapLocation::HIDEventTap, Some(event));
}

/// The points of a straight drag: `from`, then `steps` moves ending on `to`.
fn path(from: CGPoint, to: CGPoint, steps: u32) -> Vec<CGPoint> {
    (0..=steps)
        .map(|step| {
            let t = f64::from(step) / f64::from(steps);
            CGPoint::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t)
        })
        .collect()
}

/// Presses at `from`, drags to `to` in `steps` moves, and releases.
///
/// Every event is built before the first is posted, so a failed creation
/// cannot leave the button pressed. `safe` is asked about the point of each
/// event right before it is posted; when it says no the drag stops and the
/// button is released at the last point that passed, if that still does.
pub fn drag(
    from: CGPoint,
    to: CGPoint,
    steps: u32,
    safe: impl Fn(CGPoint) -> bool,
) -> Result<(), String> {
    let points = path(from, to, steps);
    let moved = event(CGEventType::MouseMoved, from)?;
    let down = event(CGEventType::LeftMouseDown, from)?;
    let drags = points[1..]
        .iter()
        .map(|p| event(CGEventType::LeftMouseDragged, *p))
        .collect::<Result<Vec<_>, _>>()?;
    let ups = points
        .iter()
        .map(|p| event(CGEventType::LeftMouseUp, *p))
        .collect::<Result<Vec<_>, _>>()?;

    if !safe(from) || !safe(to) {
        return Err("the control is not under the plugin's window at the drag points".into());
    }
    post(&moved);
    sleep(Duration::from_millis(100));
    // Layout or focus may have changed while the pointer settled.
    if !safe(from) {
        return Err("the plugin's window moved or lost focus before the press".into());
    }
    post(&down);
    let mut last = 0;
    for (i, drag_event) in drags.iter().enumerate() {
        if !safe(points[i + 1]) {
            return release(&ups[last], points[last], &safe);
        }
        post(drag_event);
        last = i + 1;
        sleep(Duration::from_millis(20));
    }
    if safe(to) {
        post(&ups[last]);
        Ok(())
    } else {
        release(&ups[last], points[last], &safe)
    }
}

/// Ends an interrupted drag, and says why it was interrupted.
fn release(up: &CGEvent, at: CGPoint, safe: &impl Fn(CGPoint) -> bool) -> Result<(), String> {
    if safe(at) {
        post(up);
        Err("the drag was interrupted (window closed, moved or covered); button released".into())
    } else {
        Err(
            "the drag was interrupted and no safe point remains: the button may stay pressed"
                .into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_runs_from_start_to_end() {
        let points = path(CGPoint::new(0.0, 10.0), CGPoint::new(100.0, 10.0), 4);
        assert_eq!(points.len(), 5);
        assert_eq!((points[0].x, points[4].x), (0.0, 100.0));
        assert!(points.iter().all(|p| p.y == 10.0));
    }
}
