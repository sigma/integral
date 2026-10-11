//! WASM bindings for Expansions: the catalog, the Expansion slots and the
//! load machine held by `DeviceState`.
//!
//! Slots cross the boundary as four slot values, one per slot A–D (`0` =
//! OFF, else the Expansion's load value; ExPCM reads as `13 13 13 13`).
//!
//! Reference: `docs/midi/99-undocumented.md`, section 5.

use integral_core::address::Address;
use integral_core::expansion_load::{LoadAction, LoadOutcome};
use integral_core::expansion_slots::{self, ExpansionSlots, SLOT_COUNT};
use integral_core::expansions::{self, Expansion};
use wasm_bindgen::prelude::*;

use crate::WasmDeviceState;

/// Family an Expansion belongs to.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpansionKind {
    /// SRX wave expansion (PCM tones and drum kits).
    Srx,
    /// SuperNATURAL expansion.
    ExSn,
    /// The ExPCM expansion.
    ExPcm,
}

impl From<expansions::ExpansionKind> for ExpansionKind {
    fn from(kind: expansions::ExpansionKind) -> Self {
        match kind {
            expansions::ExpansionKind::Srx => Self::Srx,
            expansions::ExpansionKind::ExSn => Self::ExSn,
            expansions::ExpansionKind::ExPcm => Self::ExPcm,
        }
    }
}

/// An Expansion exposed to JS.
#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmExpansion {
    name: String,
    /// Value selecting this Expansion in a slot read or load.
    #[wasm_bindgen(readonly, js_name = loadValue)]
    pub load_value: u8,
    /// Family.
    #[wasm_bindgen(readonly)]
    pub kind: ExpansionKind,
    /// Number of Expansion slots it takes when loaded.
    #[wasm_bindgen(readonly, js_name = slotCost)]
    pub slot_cost: u8,
}

impl From<&Expansion> for WasmExpansion {
    fn from(e: &Expansion) -> Self {
        Self {
            name: e.name.to_string(),
            load_value: e.load_value,
            kind: e.kind.into(),
            slot_cost: e.slot_cost,
        }
    }
}

#[wasm_bindgen]
impl WasmExpansion {
    /// Display name (e.g. "SRX-05", "ExSN1", "ExPCM").
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }
}

/// Every Expansion, in catalog (load value) order.
#[wasm_bindgen(js_name = expansionCatalog)]
pub fn expansion_catalog() -> Vec<WasmExpansion> {
    expansions::EXPANSIONS
        .iter()
        .map(WasmExpansion::from)
        .collect()
}

/// Address of the Expansion slot read and its reply DT1.
#[wasm_bindgen(js_name = expansionSlotsAddress)]
pub fn expansion_slots_address() -> Vec<u8> {
    expansion_slots::SLOT_READ_ADDR.as_bytes().to_vec()
}

/// `slots` with the Expansion of `load_value` (`0` = OFF) staged into
/// `slot` (0 = A). ExPCM fills all four slots; staging over it clears the
/// others; an Expansion already in another slot moves.
#[wasm_bindgen(js_name = stageExpansionSlot)]
pub fn stage_expansion_slot(slots: &[u8], slot: usize, load_value: u8) -> Result<Vec<u8>, JsError> {
    let slots = parse_slots(slots)?;
    if slot >= SLOT_COUNT {
        return Err(JsError::new(&format!(
            "slot {slot} out of range (0–{})",
            SLOT_COUNT - 1
        )));
    }
    let expansion = match load_value {
        0 => None,
        v => Some(
            expansions::by_load_value(v)
                .ok_or_else(|| JsError::new(&format!("unknown Expansion load value {v}")))?,
        ),
    };
    Ok(slots.with(slot, expansion).values().to_vec())
}

fn parse_slots(values: &[u8]) -> Result<ExpansionSlots, JsError> {
    ExpansionSlots::from_values(values).map_err(|e| JsError::new(&e.to_string()))
}

/// What the host should do next for a running load.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadActionKind {
    /// Send `bytes` now (the load request), bypassing the held queue.
    Send,
    /// Send `bytes` now (a slot read), bypassing the held queue.
    Poll,
    /// Ask again after `waitMs`, or on the next DT1.
    Wait,
    /// The load is over; see `outcome`.
    Done,
}

/// How a load ended.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadOutcomeKind {
    /// The target was already loaded; nothing was sent.
    NoOp,
    /// The slots now hold the target.
    Loaded,
    /// The device read back other slots than the target.
    Failed,
    /// No answer of any kind before the load timeout.
    TimedOut,
}

/// One step of a running load.
#[wasm_bindgen]
pub struct WasmLoadAction {
    /// What to do.
    #[wasm_bindgen(readonly)]
    pub kind: LoadActionKind,
    /// For `Wait`: how long until the next step (ms); `0` otherwise.
    #[wasm_bindgen(readonly, js_name = waitMs)]
    pub wait_ms: f64,
    /// For `Done`: how the load ended.
    #[wasm_bindgen(readonly)]
    pub outcome: Option<LoadOutcomeKind>,
    bytes: Vec<u8>,
    slots: Option<Vec<u8>>,
}

#[wasm_bindgen]
impl WasmLoadAction {
    /// For `Send` and `Poll`: the SysEx to send; empty otherwise.
    #[wasm_bindgen(getter)]
    pub fn bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    /// For `Done` with `Loaded` or `Failed`: the slot values read back.
    #[wasm_bindgen(getter)]
    pub fn slots(&self) -> Option<Vec<u8>> {
        self.slots.clone()
    }
}

impl From<LoadAction> for WasmLoadAction {
    fn from(action: LoadAction) -> Self {
        let mut out = Self {
            kind: LoadActionKind::Wait,
            wait_ms: 0.0,
            outcome: None,
            bytes: Vec::new(),
            slots: None,
        };
        match action {
            LoadAction::Send(bytes) => {
                out.kind = LoadActionKind::Send;
                out.bytes = bytes;
            }
            LoadAction::Poll(bytes) => {
                out.kind = LoadActionKind::Poll;
                out.bytes = bytes;
            }
            LoadAction::Wait(ms) => out.wait_ms = ms,
            LoadAction::Done(outcome) => {
                out.kind = LoadActionKind::Done;
                let (kind, slots) = match outcome {
                    LoadOutcome::NoOp => (LoadOutcomeKind::NoOp, None),
                    LoadOutcome::Loaded(s) => (LoadOutcomeKind::Loaded, Some(s)),
                    LoadOutcome::Failed(s) => (LoadOutcomeKind::Failed, Some(s)),
                    LoadOutcome::TimedOut => (LoadOutcomeKind::TimedOut, None),
                };
                out.outcome = Some(kind);
                out.slots = slots.map(|s| s.values().to_vec());
            }
        }
        out
    }
}

/// What a load would change.
#[wasm_bindgen]
pub struct WasmLoadPlan {
    changed_slots: Vec<u8>,
    silenced_parts: Vec<u8>,
}

#[wasm_bindgen]
impl WasmLoadPlan {
    /// Slots (0 = A) whose content changes, in slot order. Empty for a
    /// no-op.
    #[wasm_bindgen(getter, js_name = changedSlots)]
    pub fn changed_slots(&self) -> Vec<u8> {
        self.changed_slots.clone()
    }

    /// Parts (1-based) on a loaded Expansion that the load unloads.
    #[wasm_bindgen(getter, js_name = silencedParts)]
    pub fn silenced_parts(&self) -> Vec<u8> {
        self.silenced_parts.clone()
    }
}

#[wasm_bindgen]
impl WasmDeviceState {
    /// Build the RQ1 that reads the Expansion slots.
    #[wasm_bindgen(js_name = buildExpansionSlotsRequest)]
    pub fn build_expansion_slots_request(&self) -> Vec<u8> {
        self.inner.build_expansion_slots_request()
    }

    /// The loaded slot values A–D, or `undefined` while unknown.
    #[wasm_bindgen(js_name = expansionSlots)]
    pub fn expansion_slots(&self) -> Option<Vec<u8>> {
        self.inner.expansion_slots().map(|s| s.values().to_vec())
    }

    /// Record that a slot read got no answer: the slots become unknown.
    #[wasm_bindgen(js_name = expansionSlotsTimedOut)]
    pub fn expansion_slots_timed_out(&mut self) {
        self.inner.expansion_slots_timed_out();
    }

    /// Whether the Expansion of `load_value` is loaded, or `undefined`
    /// while the slots are unknown or for an unknown load value.
    #[wasm_bindgen(js_name = isExpansionLoaded)]
    pub fn is_expansion_loaded(&self, load_value: u8) -> Option<bool> {
        self.inner.is_loaded(expansions::by_load_value(load_value)?)
    }

    /// Feed an incoming DT1 that may concern the Expansion slots (a load
    /// reply or a slot read answer). Returns true if it did.
    #[wasm_bindgen(js_name = handleExpansionDt1)]
    pub fn handle_expansion_dt1(&mut self, addr: &[u8], data: &[u8], now_ms: f64) -> bool {
        let Ok(addr) = <[u8; 4]>::try_from(addr) else {
            return false;
        };
        let [a, b, c, d] = addr;
        self.inner
            .handle_expansion_dt1(&Address::new(a, b, c, d), data, now_ms)
    }

    /// What loading `target` (four slot values) would change. Throws if the
    /// current slots are unknown or `target` is invalid.
    #[wasm_bindgen(js_name = planExpansionLoad)]
    pub fn plan_expansion_load(&self, target: &[u8]) -> Result<WasmLoadPlan, JsError> {
        let target = parse_slots(target)?;
        let plan = self
            .inner
            .plan_expansion_load(&target)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(WasmLoadPlan {
            changed_slots: plan.changes.iter().map(|c| c.slot as u8).collect(),
            silenced_parts: plan.silenced_parts,
        })
    }

    /// Start loading `target` (four slot values). Holds the send queue until
    /// the load is over; drive it with `nextLoadAction`. Throws if the
    /// current slots are unknown, a load runs, or `target` is invalid.
    #[wasm_bindgen(js_name = startExpansionLoad)]
    pub fn start_expansion_load(&mut self, target: &[u8]) -> Result<(), JsError> {
        let target = parse_slots(target)?;
        self.inner
            .start_expansion_load(target)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// The running load's target slot values, or `undefined` if none runs.
    #[wasm_bindgen(js_name = expansionLoadTarget)]
    pub fn expansion_load_target(&self) -> Option<Vec<u8>> {
        self.inner
            .expansion_load_target()
            .map(|s| s.values().to_vec())
    }

    /// The next step of the running load at `now_ms`, or `undefined` if none
    /// runs. On `Done` the slots are updated and the queue is released.
    #[wasm_bindgen(js_name = nextLoadAction)]
    pub fn next_load_action(&mut self, now_ms: f64) -> Option<WasmLoadAction> {
        self.inner
            .next_load_action(now_ms)
            .map(WasmLoadAction::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STARTED: [u8; 4] = [0x0F, 0x00, 0x30, 0x01];

    #[test]
    fn catalog_lists_every_expansion() {
        let catalog = expansion_catalog();
        assert_eq!(catalog.len(), expansions::EXPANSIONS.len());
        let expcm = catalog.last().unwrap();
        assert_eq!(
            (expcm.name().as_str(), expcm.kind, expcm.slot_cost),
            ("ExPCM", ExpansionKind::ExPcm, 4)
        );
    }

    #[test]
    fn staging_uses_core_rules() {
        let current = [0x03, 0x05, 0x07, 0x08];
        assert_eq!(
            stage_expansion_slot(&current, 0, 0x07).unwrap(),
            [0x07, 0x05, 0x00, 0x08]
        );
        assert_eq!(stage_expansion_slot(&current, 1, 0x13).unwrap(), [0x13; 4]);
    }

    /// A load completing via a poll answer, the "done" DT1 lost.
    #[test]
    fn load_completes_via_poll_answer() {
        let mut dev = WasmDeviceState::new(0x10);
        let read = expansion_slots_address();
        assert_eq!(dev.expansion_slots(), None);
        assert!(dev.handle_expansion_dt1(&read, &[0x03, 0x05, 0x07, 0x08], 0.0));
        assert_eq!(dev.expansion_slots(), Some(vec![0x03, 0x05, 0x07, 0x08]));
        assert_eq!(dev.is_expansion_loaded(0x03), Some(true));

        let target = [0x01, 0x05, 0x07, 0x08];
        let plan = dev.plan_expansion_load(&target).unwrap();
        assert_eq!(plan.changed_slots(), [0]);
        dev.start_expansion_load(&target).unwrap();
        assert_eq!(dev.expansion_load_target(), Some(target.to_vec()));

        let send = dev.next_load_action(100.0).unwrap();
        assert_eq!(send.kind, LoadActionKind::Send);
        assert_eq!(
            send.bytes()[7..15],
            [0x0F, 0x00, 0x30, 0x00, 0x01, 0x05, 0x07, 0x08]
        );
        let wait = dev.next_load_action(110.0).unwrap();
        assert_eq!(wait.kind, LoadActionKind::Wait);
        assert!(wait.wait_ms > 0.0);

        assert!(dev.handle_expansion_dt1(&STARTED, &[0; 4], 170.0));
        let wait = dev.next_load_action(170.0).unwrap();
        assert_eq!((wait.kind, wait.wait_ms), (LoadActionKind::Wait, 500.0));
        let poll = dev.next_load_action(670.0).unwrap();
        assert_eq!(poll.kind, LoadActionKind::Poll);
        assert_eq!(poll.bytes(), dev.build_expansion_slots_request());

        assert!(dev.handle_expansion_dt1(&read, &target, 5_000.0));
        let done = dev.next_load_action(5_000.0).unwrap();
        assert_eq!(done.kind, LoadActionKind::Done);
        assert_eq!(done.outcome, Some(LoadOutcomeKind::Loaded));
        assert_eq!(done.slots(), Some(target.to_vec()));
        assert_eq!(dev.expansion_slots(), Some(target.to_vec()));
        assert!(dev.next_load_action(5_000.0).is_none());
        assert_eq!(dev.expansion_load_target(), None);
    }
}
