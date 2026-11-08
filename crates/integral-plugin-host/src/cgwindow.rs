//! The plugin's own window as the window server sees it (`CGWindow`).
//!
//! The window server is the ground truth for what is under a point:
//! Accessibility frames lag the layout, and bounds containment does not
//! prove that nothing floats above the window.

use objc2_core_foundation::{
    CFDictionary, CFNumber, CFRetained, CFString, CFType, CGPoint, CGRect, CGSize,
};
use objc2_core_graphics::{
    CGRectMakeWithDictionaryRepresentation, CGWindowListCopyWindowInfo, CGWindowListOption,
    kCGWindowAlpha, kCGWindowBounds, kCGWindowLayer, kCGWindowNumber, kCGWindowOwnerName,
    kCGWindowOwnerPID,
};

/// One on-screen window.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowInfo {
    /// The window server's id (what `screencapture -l` takes).
    pub id: u32,
    /// The owning process.
    pub pid: i32,
    /// The owning process's name.
    pub owner: String,
    /// The window layer (0 is a normal window).
    pub layer: i32,
    /// Bounds in global top-left screen coordinates.
    pub bounds: CGRect,
}

/// Whether `inner` lies fully inside `outer`, and is not degenerate.
pub fn contains(outer: CGRect, inner: CGRect) -> bool {
    inner.size.width > 0.0
        && inner.size.height > 0.0
        && inner.origin.x >= outer.origin.x
        && inner.origin.y >= outer.origin.y
        && inner.origin.x + inner.size.width <= outer.origin.x + outer.size.width
        && inner.origin.y + inner.size.height <= outer.origin.y + outer.size.height
}

/// Whether `point` lies inside `rect`.
pub fn contains_point(rect: CGRect, point: CGPoint) -> bool {
    point.x >= rect.origin.x
        && point.y >= rect.origin.y
        && point.x < rect.origin.x + rect.size.width
        && point.y < rect.origin.y + rect.size.height
}

/// All visible on-screen windows, front to back, over every layer.
/// Fully transparent windows are skipped: they take no pointer events.
fn on_screen() -> Vec<WindowInfo> {
    let Some(list) = CGWindowListCopyWindowInfo(
        CGWindowListOption::OptionOnScreenOnly | CGWindowListOption::ExcludeDesktopElements,
        0,
    ) else {
        return Vec::new();
    };
    // SAFETY: the window list is an array of dictionaries keyed by strings.
    let list = unsafe {
        CFRetained::cast_unchecked::<objc2_core_foundation::CFArray<CFDictionary<CFString, CFType>>>(
            list,
        )
    };
    // SAFETY: immutable CFString constants that live for the process.
    let (owner_pid, number_key, bounds_key, alpha_key, name_key, layer_key) = unsafe {
        (
            kCGWindowOwnerPID,
            kCGWindowNumber,
            kCGWindowBounds,
            kCGWindowAlpha,
            kCGWindowOwnerName,
            kCGWindowLayer,
        )
    };
    let mut windows = Vec::new();
    for dict in list.iter() {
        let number = |key: &CFString| -> Option<CFRetained<CFNumber>> {
            dict.get(key)?.downcast::<CFNumber>().ok()
        };
        let (Some(pid), Some(id), Some(alpha)) = (
            number(owner_pid).and_then(|n| n.as_i32()),
            number(number_key).and_then(|n| n.as_i64()),
            number(alpha_key).and_then(|n| n.as_f64()),
        ) else {
            continue;
        };
        let Some(id) = u32::try_from(id).ok().filter(|_| alpha > 0.0) else {
            continue;
        };
        let Some(Ok(bounds)) = dict.get(bounds_key).map(|b| b.downcast::<CFDictionary>()) else {
            continue;
        };
        let mut rect = CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(0.0, 0.0));
        // SAFETY: `rect` is a valid out pointer.
        let ok = unsafe { CGRectMakeWithDictionaryRepresentation(Some(&bounds), &raw mut rect) };
        if ok {
            let owner = dict
                .get(name_key)
                .and_then(|n| n.downcast::<CFString>().ok())
                .map_or_else(String::new, |n| n.to_string());
            let layer = number(layer_key)
                .and_then(|n| n.as_i32())
                .unwrap_or_default();
            windows.push(WindowInfo {
                id,
                pid,
                owner,
                layer,
                bounds: rect,
            });
        }
    }
    windows
}

/// The on-screen window with the given id.
pub fn window(id: u32) -> Option<WindowInfo> {
    on_screen().into_iter().find(|w| w.id == id)
}

/// The frontmost visible window (any layer, any process) under `point`.
pub fn topmost_at(point: CGPoint) -> Option<WindowInfo> {
    on_screen()
        .into_iter()
        .find(|w| contains_point(w.bounds, point))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_and_bottom_edges_are_outside() {
        let rect = CGRect::new(CGPoint::new(10.0, 20.0), CGSize::new(100.0, 50.0));
        assert!(contains_point(rect, CGPoint::new(10.0, 20.0)));
        assert!(contains_point(rect, CGPoint::new(109.9, 69.9)));
        assert!(!contains_point(rect, CGPoint::new(110.0, 30.0)));
        assert!(!contains_point(rect, CGPoint::new(50.0, 70.0)));
    }
}
