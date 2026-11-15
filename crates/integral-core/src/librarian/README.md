# librarian

## Overview

Reads, packs and stores the device's User memory (Studio Sets and tones). A
`Pack` keeps each entry as SysEx blocks in the Temporary layout and uses SVD
only as a file encoding. Every component that talks to the device is a pure
state machine. The host sends its RQ1, DT1, Init and write requests through
`DeviceState`'s throttled queue, then feeds back the replies and timeouts.

## Interface

- [`foreign/`](./foreign/README.md): `Pack::read` for SVD, XV SVD, SysEx and
  SMF files. It converts JV/XP and XV patches and reports what was lost.
- `pack.rs`
  - `Pack` has `entries` ordered by Pack position within a type, and methods
    `from_svd`, `to_svd` (writes all six chunks) and `of_kind`.
  - `PackEntry` has `kind`, `slot` (backup files only), `blocks`, `name`,
    `category`, `tone_refs`, `wave_groups` and `set_tone_ref`.
  - `ToneRef` is a Pack position or a `Bank`. Its `from_bank_select` method
    decodes `LSB*128+PC`. Its `bank_select` method returns `None` if no User
    bank fits.
  - `PackError` is the error type.
- `export.rs`: `PackExport` reads `ExportSource`s into a `Pack`: User
  slots, a Part's Temporary tone (`PartTone`, unsaved edits included) or
  the Temporary Studio Set (`StudioSet`).
  - Methods: `next_action`, `on_read`, `on_timeout`, `cancel`, `progress`
    and `auto_included`.
  - `ExportAction` is `Read(Vec<u8>) | Wait | Done(Result<Pack, ExportError>)`.
  - `EntryStatus` holds the result for each entry.
- `import.rs`: `PackImport` stores a `Pack` into User slots, and `reinit`
  stores INIT data.
  - `ImportAction` is
    `Send | Read | Wait | AwaitWrite { timeout_ms } | Done(ImportReport)`.
  - Methods: `next_action`, `on_read`, `on_write_reply`, `on_timeout`,
    `progress` and `cancel`.
  - `ImportReport` holds the statuses (import's own `EntryStatus`) and an
    optional `ImportError`.
  - `WRITE_TIMEOUT_MS` is 5000, the wait for each write request.
    `COMMIT_TRIES` (6) bounds how many write requests a batch sends.
- `plan.rs`
  - `plan_import` and `plan_read` propose a `Placement` for each entry:
    `User(u16) | Temporary | Unplaced | Excluded`.
  - The plan is an `ImportPlan`. Edit it with `assign`, `unassign`,
    `exclude` and `include`. Read it with `placements`, `overwrites`,
    `unplaced`, `warnings`, `origin`, `effects`, `source`, `report` and
    `file_report`.
  - `Origin`, `Warning` and `PlanError` are the supporting types.
- `reader.rs`: `BlockReader` fetches whole blocks with one RQ1 each.
  - Methods: `next_request`, `on_dt1`, `on_timeout`, `is_waiting`,
    `is_done` and `result`.
  - Each `BlockRead` holds a `BlockData`: `Complete`, `Gap { data, missing }`
    or `Timeout`.

## Invariants

- No I/O in this module: the host performs all I/O.
- Import messages go through the queue raw, so DT1s to the same address are
  not merged.
- A direct DT1 to a User slot only changes RAM, and one write request
  commits every pending direct change in User memory. Import therefore
  DT1s each entry straight to its User slot and sends one write request.
- Import order:
  1. Take a snapshot of the Temporary areas the commit borrows: the
     borrowed Part if the batch has a tone, else every Part and the
     Temporary Studio Set.
  2. DT1 the tones, then the Studio Sets (relinked to the tone slots), to
     their User slots.
  3. Commit: store the last tone (else the last Studio Set) through
     Temporary with a write request. An unanswered request is sent again
     after `WRITE_TIMEOUT_MS`, up to `COMMIT_TRIES` times; then every entry
     fails with `NotCommitted` (written in RAM, not committed). No entry is
     `Stored` before "write finished".
  4. Restore the snapshot.

  If the snapshot read fails, import aborts before sending anything.
- `reinit` stores slot by slot through Temporary, because the device's
  Init works on Temporary.
- Cancel or failure stops between slots, and the snapshot is still restored.
  A write that was already requested is still awaited. Cancelling a batch
  finishes the entry being DT1'd and commits the entries written so far.
- A plan whose only entry goes to Temporary loads that entry and stops: it
  takes no snapshot, sends no write, and restores nothing.
- Slot placement:
  - Planning never chooses a slot that is not free. A slot is free when it
    has an INIT name, and a slot missing from the names counts as taken.
  - Only `assign` can overwrite a slot. The overwritten slots are listed in
    `overwrites()`.
  - Import is blocked while any entry is `Unplaced`.
- A Studio Set Part that selects a User tone holds that tone's 0-based Pack
  position. In a backup, the position equals the slot.
- Export pulls each User tone that a Part references into the Pack once,
  ordered by User slot. Export never loads anything into a Temporary area.
- The reader never splits a nibblized parameter, because it requests whole
  blocks. Bytes past the end of a block are dropped.

## Side Effects

- None in-process. When a host runs `PackImport`, it overwrites User memory
  and briefly changes the Temporary areas, so the host should refresh mixer
  state afterwards. `PackExport` and `BlockReader` only read.

## Verification

- Run: `cargo test -p integral-core --features smf`
