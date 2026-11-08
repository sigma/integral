//! Judging a recorded stream of `IComponentHandler` edits.

use vst3_host::{ParameterEdit, ParameterEditKind};

/// What a drag on one parameter produced.
#[derive(Debug, PartialEq)]
pub struct Summary {
    /// `beginEdit` calls on the target.
    pub begins: usize,
    /// `performEdit` calls on the target.
    pub changes: usize,
    /// `endEdit` calls on the target.
    pub ends: usize,
    /// The last normalized value performed on the target.
    pub last_value: Option<f64>,
}

/// Checks that `edits` hold balanced gestures on `target` and on nothing else.
///
/// Balanced means every `beginEdit` is followed by at least one `performEdit`
/// and then its `endEdit`, with no gesture opened inside another and none left
/// open. Edits on any other parameter fail the check: a drag moves one control.
pub fn check(edits: &[ParameterEdit], target: u32) -> Result<Summary, String> {
    let mut summary = Summary {
        begins: 0,
        changes: 0,
        ends: 0,
        last_value: None,
    };
    let mut open = false;
    let mut performed_in_gesture = 0;
    for edit in edits {
        if edit.id != target {
            return Err(format!(
                "edit on parameter {} while dragging {target}",
                edit.id
            ));
        }
        match edit.kind {
            ParameterEditKind::BeginGesture => {
                if open {
                    return Err("beginEdit inside an open gesture".into());
                }
                open = true;
                performed_in_gesture = 0;
                summary.begins += 1;
            }
            ParameterEditKind::ValueChange => {
                if !open {
                    return Err("performEdit outside a gesture".into());
                }
                performed_in_gesture += 1;
                summary.changes += 1;
                summary.last_value = edit.value;
            }
            ParameterEditKind::EndGesture => {
                if !open {
                    return Err("endEdit without beginEdit".into());
                }
                if performed_in_gesture == 0 {
                    return Err("gesture ended without a performEdit".into());
                }
                open = false;
                summary.ends += 1;
            }
        }
    }
    if open {
        return Err("gesture left open (no endEdit)".into());
    }
    if summary.begins == 0 {
        return Err("no gesture recorded".into());
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(id: u32, kind: ParameterEditKind, value: Option<f64>) -> ParameterEdit {
        ParameterEdit { id, kind, value }
    }

    fn begin(id: u32) -> ParameterEdit {
        edit(id, ParameterEditKind::BeginGesture, None)
    }

    fn change(id: u32, v: f64) -> ParameterEdit {
        edit(id, ParameterEditKind::ValueChange, Some(v))
    }

    fn end(id: u32) -> ParameterEdit {
        edit(id, ParameterEditKind::EndGesture, None)
    }

    #[test]
    fn balanced_drag_passes() {
        let edits = [begin(7), change(7, 0.2), change(7, 0.4), end(7)];
        let summary = check(&edits, 7).expect("balanced");
        assert_eq!(
            summary,
            Summary {
                begins: 1,
                changes: 2,
                ends: 1,
                last_value: Some(0.4)
            }
        );
    }

    #[test]
    fn two_gestures_pass() {
        let edits = [
            begin(7),
            change(7, 0.2),
            end(7),
            begin(7),
            change(7, 0.3),
            end(7),
        ];
        assert_eq!(check(&edits, 7).expect("balanced").begins, 2);
    }

    #[test]
    fn empty_stream_fails() {
        assert!(check(&[], 7).is_err());
    }

    #[test]
    fn unclosed_gesture_fails() {
        assert!(check(&[begin(7), change(7, 0.2)], 7).is_err());
    }

    #[test]
    fn change_outside_gesture_fails() {
        assert!(check(&[change(7, 0.2), begin(7), change(7, 0.3), end(7)], 7).is_err());
    }

    #[test]
    fn nested_begin_fails() {
        assert!(check(&[begin(7), begin(7), change(7, 0.1), end(7)], 7).is_err());
    }

    #[test]
    fn empty_gesture_fails() {
        assert!(check(&[begin(7), end(7)], 7).is_err());
    }

    #[test]
    fn other_parameter_fails() {
        assert!(check(&[begin(7), change(8, 0.1), end(7)], 7).is_err());
    }
}
