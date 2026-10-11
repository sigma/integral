//! WASM bindings for the Librarian: Packs, import plans and the export and
//! import machines.
//!
//! Types cross the boundary as their CLI name (`"sn-synth"`, ...). Plain
//! data crosses as JS objects typed by the TypeScript section below; the
//! Pack, the plan and the machines stay in Rust behind handles. The host
//! drives a machine as `integral_core::librarian::{export, import}`
//! describe: it sends what `nextAction` asks for and feeds DT1s back.

use integral_core::address::Address;
use integral_core::catalog::CatalogEntry;
use integral_core::device_spec::INTEGRA7;
use integral_core::expansion_slots::ExpansionSlots;
use integral_core::librarian::export::{self, ExportAction, ExportSource, PackExport};
use integral_core::librarian::import::{ImportAction, PackImport};
use integral_core::librarian::pack::{Pack, ToneRef};
use integral_core::librarian::plan::{self, ImportPlan, Origin};
use integral_core::svd::ChunkType;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/** A Librarian type (Studio Set or a tone type), by its CLI name. */
export interface LibrarianType {
  kind: string;
  label: string;
  slotCount: number;
  bankMsb: number;
  /** Names of an initialized (free) slot. */
  initNames: string[];
  /** The Studio Set, whose names come from the Studio Set catalog. */
  studioSet: boolean;
}
/** The tone a Studio Set Part selects: a tone of the Pack, or a bank. */
export type ToneRef =
  | { pack: { kind: string; index: number } }
  | { bank: { msb: number; lsb: number; pc: number } };
/** One entry of a Pack, for display. */
export interface PackEntryInfo {
  kind: string;
  /** Position among the Pack's entries of its type. */
  index: number;
  /** User slot it came from (0-indexed), if known. */
  slot?: number;
  name: string;
  category?: number;
  /** Each Part's tone; empty for a tone. */
  toneRefs: ToneRef[];
}
/** A User slot's name, from the catalog. */
export interface CatalogName { msb: number; lsb: number; pc: number; name: string }
/** Where an entry goes: a User slot (0-indexed) of its type, or not. */
export type Placement = { user: number } | "temporary" | "unplaced" | "excluded";
/** Something to tell the user before importing Studio Set `set`. */
export interface ImportWarning { set: number; part: number; message: string }
/** Something to export. */
export type ExportSource =
  | { source: "user"; kind: string; slot: number }
  | { source: "partTone"; kind: string; part: number }
  | { source: "studioSet" };
export interface ExportProgress {
  source: ExportSource;
  status: "pending" | "reading" | "read" | "failed";
}
export type ImportStatus =
  "pending" | "storing" | "stored" | "loaded" | "failed" | "excluded";
"#;

#[wasm_bindgen]
extern "C" {
    /// `LibrarianType[]`.
    #[wasm_bindgen(typescript_type = "LibrarianType[]")]
    pub type LibrarianTypes;
    /// `PackEntryInfo[]`.
    #[wasm_bindgen(typescript_type = "PackEntryInfo[]")]
    pub type PackEntries;
    /// `CatalogName[]`.
    #[wasm_bindgen(typescript_type = "CatalogName[]")]
    pub type CatalogNames;
    /// `Placement[]`.
    #[wasm_bindgen(typescript_type = "Placement[]")]
    pub type Placements;
    /// `ImportWarning[]`.
    #[wasm_bindgen(typescript_type = "ImportWarning[]")]
    pub type ImportWarnings;
    /// `ExportSource[]`.
    #[wasm_bindgen(typescript_type = "ExportSource[]")]
    pub type ExportSources;
    /// `ExportProgress[]`.
    #[wasm_bindgen(typescript_type = "ExportProgress[]")]
    pub type ExportProgressList;
    /// `ImportStatus[]`.
    #[wasm_bindgen(typescript_type = "ImportStatus[]")]
    pub type ImportStatuses;
}

fn to_js<T: Serialize + ?Sized, J: JsCast>(value: &T) -> Result<J, JsError> {
    Ok(serde_wasm_bindgen::to_value(value)
        .map_err(|e| JsError::new(&e.to_string()))?
        .unchecked_into())
}

fn from_js<T: for<'de> Deserialize<'de>>(value: JsValue) -> Result<T, JsError> {
    serde_wasm_bindgen::from_value(value).map_err(|e| JsError::new(&e.to_string()))
}

fn js_err(e: impl ToString) -> JsError {
    JsError::new(&e.to_string())
}

fn address(addr: &[u8]) -> Option<Address> {
    let [a, b, c, d] = *addr else { return None };
    Some(Address::new(a, b, c, d))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TypeInfo {
    kind: ChunkType,
    label: &'static str,
    slot_count: u16,
    bank_msb: u8,
    init_names: &'static [&'static str],
    studio_set: bool,
}

/// Every Librarian type with its User memory, Studio Set first.
#[wasm_bindgen(js_name = librarianTypes)]
pub fn librarian_types() -> Result<LibrarianTypes, JsError> {
    let types: Vec<TypeInfo> = INTEGRA7
        .user_memory
        .iter()
        .filter_map(|m| {
            let kind = ChunkType::from_cli_name(m.cli_name)?;
            Some(TypeInfo {
                kind,
                label: kind.label(),
                slot_count: m.slot_count,
                bank_msb: m.bank_msb,
                init_names: m.init_names,
                studio_set: m.tone_type.is_none(),
            })
        })
        .collect();
    to_js(&types)
}

// ---------------------------------------------------------------------------
// Pack
// ---------------------------------------------------------------------------

/// Studio Sets and tones moved together.
#[wasm_bindgen(js_name = Pack)]
#[derive(Clone)]
pub struct WasmPack {
    inner: Pack,
    origin: Origin,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EntryInfo {
    kind: ChunkType,
    index: usize,
    slot: Option<u16>,
    name: String,
    category: Option<u8>,
    tone_refs: Vec<ToneRef>,
}

#[wasm_bindgen(js_class = Pack)]
impl WasmPack {
    /// Read a sound-export or backup SVD file.
    #[wasm_bindgen(js_name = fromSvd)]
    pub fn from_svd(bytes: &[u8]) -> Result<WasmPack, JsError> {
        let inner = Pack::from_svd(bytes).map_err(js_err)?;
        // `from_svd` gives entries a slot only when reading a backup.
        let backup = inner.entries.iter().any(|e| e.slot().is_some());
        let origin = if backup {
            Origin::Backup
        } else {
            Origin::Sound
        };
        Ok(Self { inner, origin })
    }

    /// The sound-export SVD file.
    #[wasm_bindgen(js_name = toSvd)]
    pub fn to_svd(&self) -> Result<Vec<u8>, JsError> {
        self.inner.to_svd().map_err(js_err)
    }

    /// Whether it was read from a full backup.
    #[wasm_bindgen(getter, js_name = isBackup)]
    pub fn is_backup(&self) -> bool {
        self.origin == Origin::Backup
    }

    /// The entries, in Pack order.
    pub fn entries(&self) -> Result<PackEntries, JsError> {
        let entries: Vec<EntryInfo> = self
            .inner
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| EntryInfo {
                kind: e.kind(),
                index: self.inner.entries[..i]
                    .iter()
                    .filter(|o| o.kind() == e.kind())
                    .count(),
                slot: e.slot(),
                name: e.name(),
                category: e.category(),
                tone_refs: e.tone_refs(),
            })
            .collect();
        to_js(&entries)
    }
}

// ---------------------------------------------------------------------------
// Import plan
// ---------------------------------------------------------------------------

/// Where each entry of a Pack goes.
#[wasm_bindgen(js_name = ImportPlan)]
pub struct WasmImportPlan {
    inner: ImportPlan,
}

#[derive(Deserialize)]
struct CatalogName {
    msb: u8,
    lsb: u8,
    pc: u8,
    name: String,
}

#[derive(Serialize)]
struct WarningInfo {
    set: usize,
    part: u8,
    message: String,
}

/// Plan importing `pack`. `slot_names` are the catalog names of the User
/// banks of its types (a slot missing from them counts as taken);
/// `loaded` the Expansion slot values A–D, or `undefined` when unknown.
#[wasm_bindgen(js_name = planImport)]
pub fn plan_import(
    pack: &WasmPack,
    slot_names: CatalogNames,
    loaded: Option<Vec<u8>>,
) -> Result<WasmImportPlan, JsError> {
    let names: Vec<CatalogName> = from_js(slot_names.into())?;
    let names: Vec<CatalogEntry> = names
        .into_iter()
        .map(|n| CatalogEntry {
            bank_msb: n.msb,
            bank_lsb: n.lsb,
            pc: n.pc,
            name: n.name,
        })
        .collect();
    let loaded = loaded
        .map(|v| ExpansionSlots::from_values(&v))
        .transpose()
        .map_err(js_err)?;
    Ok(WasmImportPlan {
        inner: plan::plan_import(&pack.inner, pack.origin, &names, loaded.as_ref()),
    })
}

#[wasm_bindgen(js_class = ImportPlan)]
impl WasmImportPlan {
    /// Where each entry goes, indexed like the Pack's entries.
    pub fn placements(&self) -> Result<Placements, JsError> {
        to_js(self.inner.placements())
    }

    /// Entries with no destination yet; the import can't start until none.
    pub fn unplaced(&self) -> Vec<usize> {
        self.inner.unplaced().collect()
    }

    /// Entries whose destination is a User slot that isn't free.
    pub fn overwrites(&self) -> Vec<usize> {
        self.inner.overwrites().collect()
    }

    /// Warnings about the Studio Sets that are not excluded.
    pub fn warnings(&self) -> Result<ImportWarnings, JsError> {
        let warnings: Vec<WarningInfo> = self
            .inner
            .warnings()
            .map(|w| {
                let (plan::Warning::BrokenReference { part, .. }
                | plan::Warning::ExpansionNotLoaded { part, .. }) = *w;
                WarningInfo {
                    set: w.set(),
                    part,
                    message: w.to_string(),
                }
            })
            .collect();
        to_js(&warnings)
    }

    /// Send `entry` to User slot `slot` (0-indexed) of its type.
    pub fn assign(&mut self, entry: usize, slot: u16) -> Result<(), JsError> {
        self.inner.assign(entry, slot).map_err(js_err)
    }

    /// Clear `entry`'s destination, or take it back after `exclude`.
    pub fn unassign(&mut self, entry: usize) -> Result<(), JsError> {
        self.inner.unassign(entry).map_err(js_err)
    }

    /// Leave `entry` out of the import.
    pub fn exclude(&mut self, entry: usize) -> Result<(), JsError> {
        self.inner.exclude(entry).map_err(js_err)
    }
}

// ---------------------------------------------------------------------------
// Machines
// ---------------------------------------------------------------------------

/// What the host should do next for a running export or import.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibrarianActionKind {
    /// Send `bytes` (an RQ1) and feed its DT1 replies to `onRead`.
    Read,
    /// Send `bytes`; nothing comes back.
    Send,
    /// Feed DT1s to `onRead`, or call `onTimeout` once the reply timeout
    /// elapses.
    Wait,
    /// Feed DT1s to `onWriteReply` until it returns true, or call
    /// `onTimeout` after `timeoutMs`.
    AwaitWrite,
    /// Over; `error` says why if it stopped early.
    Done,
}

/// One step of a running export or import.
#[wasm_bindgen]
pub struct LibrarianAction {
    /// What to do.
    #[wasm_bindgen(readonly)]
    pub kind: LibrarianActionKind,
    /// For `AwaitWrite`: how long to wait for "write finished" (ms).
    #[wasm_bindgen(readonly, js_name = timeoutMs)]
    pub timeout_ms: f64,
    bytes: Vec<u8>,
    error: Option<String>,
}

impl LibrarianAction {
    fn new(kind: LibrarianActionKind) -> Self {
        Self {
            kind,
            timeout_ms: 0.0,
            bytes: Vec::new(),
            error: None,
        }
    }

    fn with_bytes(kind: LibrarianActionKind, bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            ..Self::new(kind)
        }
    }

    fn done(error: Option<String>) -> Self {
        Self {
            error,
            ..Self::new(LibrarianActionKind::Done)
        }
    }
}

#[wasm_bindgen]
impl LibrarianAction {
    /// For `Read` and `Send`: the SysEx to send; empty otherwise.
    #[wasm_bindgen(getter)]
    pub fn bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    /// For `Done`: why it stopped early, if it did.
    #[wasm_bindgen(getter)]
    pub fn error(&self) -> Option<String> {
        self.error.clone()
    }
}

/// Reads Studio Sets and tones from the device into a Pack.
#[wasm_bindgen(js_name = PackExport)]
pub struct WasmPackExport {
    inner: PackExport,
    pack: Option<Pack>,
}

#[derive(Serialize)]
struct ProgressInfo {
    source: ExportSource,
    status: export::EntryStatus,
}

#[wasm_bindgen(js_class = PackExport)]
impl WasmPackExport {
    /// Prepare exporting `sources`; duplicates are read once.
    #[wasm_bindgen(constructor)]
    pub fn new(device_id: u8, sources: ExportSources) -> Result<WasmPackExport, JsError> {
        let sources: Vec<ExportSource> = from_js(sources.into())?;
        Ok(Self {
            inner: PackExport::new(device_id, sources).map_err(js_err)?,
            pack: None,
        })
    }

    /// The next step.
    #[wasm_bindgen(js_name = nextAction)]
    pub fn next_action(&mut self) -> LibrarianAction {
        match self.inner.next_action() {
            ExportAction::Read(bytes) => {
                LibrarianAction::with_bytes(LibrarianActionKind::Read, bytes)
            }
            ExportAction::Wait => LibrarianAction::new(LibrarianActionKind::Wait),
            ExportAction::Done(Ok(pack)) => {
                self.pack = Some(pack);
                LibrarianAction::done(None)
            }
            ExportAction::Done(Err(e)) => LibrarianAction::done(Some(e.to_string())),
        }
    }

    /// The exported Pack, once `nextAction` returned `Done` with no error.
    pub fn pack(&self) -> Option<WasmPack> {
        self.pack.clone().map(|inner| WasmPack {
            inner,
            origin: Origin::Sound,
        })
    }

    /// Feed an incoming DT1. Returns true if it belonged to the read in
    /// flight.
    #[wasm_bindgen(js_name = onRead)]
    pub fn on_read(&mut self, addr: &[u8], data: &[u8]) -> bool {
        address(addr).is_some_and(|a| self.inner.on_read(&a, data))
    }

    /// The reply timeout elapsed: stop waiting for the current block.
    #[wasm_bindgen(js_name = onTimeout)]
    pub fn on_timeout(&mut self) {
        self.inner.on_timeout();
    }

    /// Every source with its status, in Pack order.
    pub fn progress(&self) -> Result<ExportProgressList, JsError> {
        let progress: Vec<ProgressInfo> = self
            .inner
            .progress()
            .into_iter()
            .map(|(source, status)| ProgressInfo { source, status })
            .collect();
        to_js(&progress)
    }

    /// The User tones pulled in by Studio Set references, in Pack order.
    #[wasm_bindgen(js_name = autoIncluded)]
    pub fn auto_included(&self) -> Result<ExportSources, JsError> {
        to_js(self.inner.auto_included())
    }

    /// Stop the export; late replies are ignored.
    pub fn cancel(&mut self) {
        self.inner.cancel();
    }
}

/// Stores a Pack into User memory as an import plan says.
#[wasm_bindgen(js_name = PackImport)]
pub struct WasmPackImport {
    inner: PackImport,
}

#[wasm_bindgen(js_class = PackImport)]
impl WasmPackImport {
    /// Prepare importing `pack` as `plan` says, borrowing Part `part`
    /// (0-indexed) to store tones.
    #[wasm_bindgen(constructor)]
    pub fn new(
        device_id: u8,
        pack: &WasmPack,
        plan: &WasmImportPlan,
        part: u8,
    ) -> Result<WasmPackImport, JsError> {
        Ok(Self {
            inner: PackImport::new(device_id, &pack.inner, &plan.inner, part).map_err(js_err)?,
        })
    }

    /// The next step. On `Done`, refresh mixer state: the Temporary areas
    /// changed.
    #[wasm_bindgen(js_name = nextAction)]
    pub fn next_action(&mut self) -> LibrarianAction {
        match self.inner.next_action() {
            ImportAction::Send(bytes) => {
                LibrarianAction::with_bytes(LibrarianActionKind::Send, bytes)
            }
            ImportAction::Read(bytes) => {
                LibrarianAction::with_bytes(LibrarianActionKind::Read, bytes)
            }
            ImportAction::Wait => LibrarianAction::new(LibrarianActionKind::Wait),
            ImportAction::AwaitWrite { timeout_ms } => LibrarianAction {
                timeout_ms,
                ..LibrarianAction::new(LibrarianActionKind::AwaitWrite)
            },
            ImportAction::Done(report) => {
                LibrarianAction::done(report.error.map(|e| e.to_string()))
            }
        }
    }

    /// Feed an incoming DT1. Returns true if it belonged to the snapshot
    /// read in flight.
    #[wasm_bindgen(js_name = onRead)]
    pub fn on_read(&mut self, addr: &[u8], data: &[u8]) -> bool {
        address(addr).is_some_and(|a| self.inner.on_read(&a, data))
    }

    /// Feed an incoming DT1 while awaiting a write. Returns true if it was
    /// the awaited "write finished".
    #[wasm_bindgen(js_name = onWriteReply)]
    pub fn on_write_reply(&mut self, addr: &[u8], data: &[u8]) -> bool {
        address(addr).is_some_and(|a| self.inner.on_write_reply(&a, data))
    }

    /// A wait ran out: stop waiting for the snapshot block, or give up on
    /// the awaited write.
    #[wasm_bindgen(js_name = onTimeout)]
    pub fn on_timeout(&mut self) {
        self.inner.on_timeout();
    }

    /// Each entry's status, indexed like the Pack's entries.
    pub fn progress(&self) -> Result<ImportStatuses, JsError> {
        to_js(self.inner.progress())
    }

    /// Stop between slots; the snapshot is still restored.
    pub fn cancel(&mut self) {
        self.inner.cancel();
    }
}

#[cfg(test)]
mod tests {
    use integral_core::librarian::plan::Placement;
    use serde_json::{from_str, json, to_value};

    use super::*;

    /// The JSON shapes the TypeScript section declares.
    #[test]
    fn shapes_match_the_typescript_types() {
        let source: ExportSource =
            from_str(r#"{"source":"user","kind":"sn-synth","slot":3}"#).unwrap();
        assert_eq!(
            source,
            ExportSource::User {
                kind: ChunkType::SnSynthTone,
                slot: 3
            }
        );
        assert_eq!(
            to_value(ExportSource::PartTone {
                kind: ChunkType::PcmDrumKit,
                part: 1
            })
            .unwrap(),
            json!({"source": "partTone", "kind": "pcm-drum", "part": 1})
        );
        assert_eq!(
            to_value([Placement::User(3), Placement::Temporary]).unwrap(),
            json!([{"user": 3}, "temporary"])
        );
        let tone = ToneRef::Pack {
            kind: ChunkType::SnSynthTone,
            index: 0,
        };
        assert_eq!(
            to_value(tone).unwrap(),
            json!({"pack": {"kind": "sn-synth", "index": 0}})
        );
        assert_eq!(
            to_value(export::EntryStatus::Reading).unwrap(),
            json!("reading")
        );
    }
}
