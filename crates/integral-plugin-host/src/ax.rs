//! A small Accessibility (AX) client: find a labelled control in the
//! plugin's window and read its frame and value.
//!
//! It runs in the host process, so there is one process and one
//! Accessibility grant. Calls must come from a thread that is *not* the one
//! pumping AppKit: the AX server answers through that run loop.

use std::ffi::c_void;
use std::ptr::NonNull;

use accessibility_sys::{
    AXIsProcessTrusted, AXUIElementCopyAttributeValue, AXUIElementCreateApplication,
    AXUIElementPerformAction, AXUIElementRef, AXUIElementSetAttributeValue,
    AXUIElementSetMessagingTimeout, AXValueGetType, AXValueGetTypeID, AXValueGetValue, AXValueRef,
    kAXChildrenAttribute, kAXDescriptionAttribute, kAXErrorSuccess, kAXHelpAttribute,
    kAXPositionAttribute, kAXRoleAttribute, kAXSizeAttribute, kAXTitleAttribute, kAXValueAttribute,
    kAXValueTypeCGPoint, kAXValueTypeCGSize,
};
use core_foundation_sys::base::CFGetTypeID;
use objc2_core_foundation::{
    CFArray, CFBoolean, CFNumber, CFRetained, CFString, CFType, CGPoint, CGRect, CGSize,
};

/// The role WebKit gives an `<input type="range">`.
pub const SLIDER_ROLE: &str = "AXSlider";

/// The role WebKit gives text on the page.
pub const STATIC_TEXT_ROLE: &str = "AXStaticText";

/// Cap on the tree walk: the page is a few hundred nodes, not thousands.
const MAX_NODES: usize = 20_000;
const MAX_DEPTH: usize = 60;

/// Whether this process may use Accessibility (never prompts).
pub fn trusted() -> bool {
    // SAFETY: plain query, no arguments.
    unsafe { AXIsProcessTrusted() }
}

/// An AX element (`AXUIElementRef`).
pub struct Element(CFRetained<CFType>);

impl Element {
    /// The application element of `pid`, with AX timeouts so a stalled app
    /// cannot hang the run, and enhanced AX on so WebKit builds its tree.
    pub fn application(pid: i32) -> Option<Self> {
        // SAFETY: returns a +1 retained element, or null.
        let raw = unsafe { AXUIElementCreateApplication(pid) };
        let element = NonNull::new(raw.cast::<CFType>())?;
        // SAFETY: owned (+1) CF object.
        let app = Self(unsafe { CFRetained::from_raw(element) });
        let timeout = unsafe { AXUIElementSetMessagingTimeout(app.raw(), 2.0) };
        debug_assert_eq!(timeout, kAXErrorSuccess);
        let name = CFString::from_str("AXEnhancedUserInterface");
        // SAFETY: valid element, CFString name and CFBoolean value.
        unsafe {
            AXUIElementSetAttributeValue(
                app.raw(),
                (&raw const *name).cast(),
                (&raw const *CFBoolean::new(true)).cast(),
            );
        }
        Some(app)
    }

    fn raw(&self) -> AXUIElementRef {
        (&raw const *self.0).cast_mut().cast()
    }

    fn attribute(&self, name: &str) -> Option<CFRetained<CFType>> {
        let name = CFString::from_str(name);
        let mut out: *const c_void = std::ptr::null();
        // SAFETY: valid element and name; `out` receives a +1 value on success.
        let status = unsafe {
            AXUIElementCopyAttributeValue(self.raw(), (&raw const *name).cast(), &raw mut out)
        };
        if status != kAXErrorSuccess {
            return None;
        }
        let value = NonNull::new(out.cast_mut().cast::<CFType>())?;
        // SAFETY: owned (+1) CF object.
        Some(unsafe { CFRetained::from_raw(value) })
    }

    fn string(&self, name: &str) -> Option<String> {
        Some(
            self.attribute(name)?
                .downcast_ref::<CFString>()?
                .to_string(),
        )
    }

    /// The AX role, e.g. `AXSlider`.
    pub fn role(&self) -> Option<String> {
        self.string(kAXRoleAttribute)
    }

    /// Each non-empty label attribute (title, description, help).
    pub fn labels(&self) -> Vec<String> {
        [kAXTitleAttribute, kAXDescriptionAttribute, kAXHelpAttribute]
            .iter()
            .filter_map(|a| self.string(a))
            .filter(|l| !l.is_empty())
            .collect()
    }

    /// Child elements.
    pub fn children(&self) -> Vec<Element> {
        let Some(value) = self.attribute(kAXChildrenAttribute) else {
            return Vec::new();
        };
        let Some(array) = value.downcast_ref::<CFArray>() else {
            return Vec::new();
        };
        // SAFETY: the AX children array holds AX elements, which are CF types.
        let array = unsafe { array.cast_unchecked::<CFType>() };
        array.iter().map(Element).collect()
    }

    /// Reads an `AXValue` attribute of the given AX value type into `out`.
    ///
    /// # Safety
    ///
    /// `out` must point to storage for a value of `ax_type` (`CGPoint` for
    /// `kAXValueTypeCGPoint`, `CGSize` for `kAXValueTypeCGSize`).
    unsafe fn ax_value(&self, name: &str, ax_type: u32, out: *mut c_void) -> bool {
        let Some(value) = self.attribute(name) else {
            return false;
        };
        let raw: AXValueRef = (&raw const *value).cast_mut().cast();
        // SAFETY: the type ids are checked before the AXValue calls.
        unsafe {
            CFGetTypeID(raw.cast()) == AXValueGetTypeID()
                && AXValueGetType(raw) == ax_type
                && AXValueGetValue(raw, ax_type, out)
        }
    }

    /// The element's frame in global top-left screen coordinates.
    pub fn frame(&self) -> Option<CGRect> {
        let mut origin = CGPoint::new(0.0, 0.0);
        let mut size = CGSize::new(0.0, 0.0);
        // SAFETY: `origin` and `size` match the requested AX value types.
        let ok = unsafe {
            self.ax_value(
                kAXPositionAttribute,
                kAXValueTypeCGPoint,
                (&raw mut origin).cast(),
            ) && self.ax_value(kAXSizeAttribute, kAXValueTypeCGSize, (&raw mut size).cast())
        };
        ok.then(|| CGRect::new(origin, size))
    }

    /// Asks the element's scroll view to bring it into view
    /// (`AXScrollToVisible`). Returns whether the action was accepted.
    pub fn scroll_to_visible(&self) -> bool {
        let action = CFString::from_str("AXScrollToVisible");
        // SAFETY: valid element and CFString action name.
        unsafe {
            AXUIElementPerformAction(self.raw(), (&raw const *action).cast()) == kAXErrorSuccess
        }
    }

    /// The numeric value (a slider's position), if it has one.
    pub fn number(&self) -> Option<f64> {
        self.attribute(kAXValueAttribute)?
            .downcast_ref::<CFNumber>()?
            .as_f64()
    }

    /// Every descendant (and `self`) with `role`, depth-first.
    pub fn descendants(&self, role: &str) -> Vec<Element> {
        self.descendants_where(|e| e.role().as_deref() == Some(role))
    }

    /// Every descendant (and `self`) that `keep` accepts, depth-first.
    pub fn descendants_where(&self, keep: impl Fn(&Element) -> bool) -> Vec<Element> {
        let mut found = Vec::new();
        let mut seen = 0;
        self.collect(&keep, 0, &mut seen, &mut found);
        found
    }

    /// The element's text: its value if that is a string (static text), and
    /// each label.
    pub fn texts(&self) -> Vec<String> {
        let mut texts = self.labels();
        texts.extend(self.string(kAXValueAttribute));
        texts
    }

    fn collect(
        &self,
        keep: &dyn Fn(&Element) -> bool,
        depth: usize,
        seen: &mut usize,
        found: &mut Vec<Element>,
    ) {
        *seen += 1;
        if *seen > MAX_NODES || depth > MAX_DEPTH {
            return;
        }
        if keep(self) {
            found.push(Element(self.0.clone()));
        }
        for child in self.children() {
            child.collect(keep, depth + 1, seen, found);
        }
    }
}

/// How a control was picked out of several candidates.
#[derive(Debug)]
pub enum Pick<T> {
    /// Exactly one candidate matched.
    One(T),
    /// Nothing matched (yet).
    None,
    /// More than one matched at the best level: the label is ambiguous.
    Ambiguous(usize),
}

/// Picks the candidate labelled `needle`: an exact match of any one label
/// if there is one, else a case-insensitive substring match. More than one
/// match at the chosen level is ambiguous, never "the first".
pub fn pick<T>(candidates: Vec<(T, Vec<String>)>, needle: &str) -> Pick<T> {
    let lower = needle.to_lowercase();
    let (exact, partial): (Vec<_>, Vec<_>) = candidates
        .into_iter()
        .partition(|(_, labels)| labels.iter().any(|l| l == needle));
    let mut level = if exact.is_empty() {
        partial
            .into_iter()
            .filter(|(_, labels)| labels.iter().any(|l| l.to_lowercase().contains(&lower)))
            .collect()
    } else {
        exact
    };
    match level.len() {
        0 => Pick::None,
        1 => Pick::One(level.remove(0).0),
        n => Pick::Ambiguous(n),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(id: u32, labels: &[&str]) -> (u32, Vec<String>) {
        (id, labels.iter().map(|l| (*l).to_owned()).collect())
    }

    #[test]
    fn exact_beats_substring() {
        let picked = pick(
            vec![c(1, &["Reverb Send Level"]), c(2, &["Level"])],
            "Level",
        );
        assert!(matches!(picked, Pick::One(2)));
    }

    #[test]
    fn single_substring_is_accepted() {
        let picked = pick(vec![c(1, &["Spike: Level"]), c(2, &["Pan"])], "Level");
        assert!(matches!(picked, Pick::One(1)));
    }

    #[test]
    fn several_substrings_are_ambiguous() {
        let picked = pick(vec![c(1, &["Send Level"]), c(2, &["Part Level"])], "Level");
        assert!(matches!(picked, Pick::Ambiguous(2)));
    }

    #[test]
    fn several_exact_are_ambiguous() {
        let picked = pick(vec![c(1, &["Level"]), c(2, &["Level"])], "Level");
        assert!(matches!(picked, Pick::Ambiguous(2)));
    }

    #[test]
    fn nothing_matching_is_none() {
        assert!(matches!(pick(vec![c(1, &["Pan"])], "Level"), Pick::None));
    }
}
