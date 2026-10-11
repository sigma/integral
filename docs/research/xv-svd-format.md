# XV-series `.svd` Files vs the INTEGRA-7 SVD Container (research for #203)

Answers [#203](https://github.com/sigma/integral/issues/203) (parent map
[#192](https://github.com/sigma/integral/issues/192)). Do the `.svd` files that XV-series
synths save use the INTEGRA-7 SVD container ([`docs/svd/01-container.md`](../svd/01-container.md),
[`crates/integral-core/src/svd.rs`](../../crates/integral-core/src/svd.rs)), how are XV
Patches encoded inside, and is `.svd` import worth building?

## TL;DR

- **Corpus: 32 `.svd` files, 19 XV and 13 JV/XP-era.** The XV ones have signature `SVD1`
  and chunk model `XV88` (some chunks `NEXY`). The JV/XP ones have signature **`SVD0`** and
  model `XP50`/`XP80`. Both of the `~/Downloads/JV-1080_SP/` files are `SVD0`.
- **The XV container is the Integra container.** It has the same 16-byte header (u16 header
  length + signature + 10 zero bytes), the same 16-byte directory entries
  (code, model, offset, size), and the same 16-byte zone header (count, entry size,
  `0x10`, `0`). The only differences are the model ID (`XV88`/`NEXY`, not `MI69`), the chunk
  codes, and that an empty chunk records entry size 0.
- **XV Patches (`PATa`) use the Integra bit-packing:** each SysEx byte is stored MSB first,
  using only its mask bits, and each section is byte-aligned. The Common, MFX, TMT and
  Tone bit widths are exactly the
  Integra RFPa ones (`PCMS_*` and `MFX_PARAMS` in
  [`svd_specs.rs`](../../crates/integral-core/src/svd_specs.rs)). Chorus and Reverb sections
  are new. Decoding with this layout reproduces the SysEx twin **byte for byte** for
  899 + 251 + 12 entries across all three entry sizes.
- **`SvdFile::parse` accepts none of them.** It returns `InvalidChunkType` for all 19 XV files
  (the first code is `PRFa` or `MLTa`) and `InvalidMagic` for all 13 `SVD0` files.
- **`SVD0` (JV/XP) is not readable with this scheme.** The header and directory are the same,
  but there are no zone headers, and the Patch data is not plain bit-packing (no Patch name
  decodes at any bit offset). Not decoded.
- **Recommendation: reuse the SVD reader with a new chunk spec, but as a late, optional
  slice.** The container code needs only a model/chunk-code parameter, and the entry decoder
  needs one new `SvdToneSpec` built mostly from existing `PCMS_*` tables. The output is the
  9 XV SysEx blocks, which then go through the XV SysEx path unchanged. The payoff is
  **542 distinct non-INIT XV Patches that exist only as `.svd`** in this corpus, against
  ≈6.2k distinct XV Patches already available as SysEx ([#197](https://github.com/sigma/integral/issues/197)).
  Skip `SVD0`.
- **Confidence:** high for the container match, the section layout and the widths (they were
  checked against hundreds of SysEx twins). Medium for what the `NEXY` chunks and the
  18-byte trailer are. Low for the `SVD0` payload (not decoded).

## Sources (highest trust first)

| # | Source | What it gives |
|---|--------|---------------|
| P1 | The 32 `.svd` files under `~/Downloads/integral-corpus/` and `~/Downloads/JV-1080_SP/` (read-only, see the corpus `MANIFEST.md`) | Raw bytes: headers, directories, zone headers, entries. |
| P2 | XV SysEx Patch dumps in the same corpus (979 `.syx`/`.mid` files scanned, Model ID `00 10`, complete 9-block Patches only) | Ground truth for each `.svd` entry that has a SysEx twin. |
| P3 | Roland *XV-5080 Owner's Manual* ([PDF](https://static.roland.com/assets/media/pdf/XV-5080_OM.pdf)), PDF pp. 188, 190–191, 194 (`.SVD` files), p. 297 (Patch Split Key block) | What an XV-5080 `.SVD` holds, and the Split Key parameters. |
| P4 | Roland *XV-2020 Owner's Manual* ([PDF](https://static.roland.com/assets/media/pdf/XV-2020_OM.pdf)), PDF p. 12, p. 135 | The XV-2020 has no drive: XV Editor writes its `.SVD` ("Save SVD File…"). The Performance list (p. 135) matches the `MLTa` names. |
| P5 | [`crates/integral-core/src/svd.rs`](../../crates/integral-core/src/svd.rs) (`SvdFile::parse`), [`svd_specs.rs`](../../crates/integral-core/src/svd_specs.rs) (`PCMS_COMMON_PARAMS`, `MFX_PARAMS`, `PCMS_PMT_PARAMS`, `PCMS_PARTIAL_PARAMS`) | The Integra reader and the bit widths reused for XV. |
| P6 | [`docs/svd/01-container.md`](../svd/01-container.md), [`02-encoding.md`](../svd/02-encoding.md), [`06-pcm-synth.md`](../svd/06-pcm-synth.md) | The Integra container and encoding. |
| P7 | [`docs/research/xv-patch-format.md`](xv-patch-format.md) | XV Patch blocks (9 blocks, 1056-byte dump) and their Integra counterparts. |

No public write-up of the XV `.svd` format was found (a web search for the chunk codes
found nothing). Everything below comes from P1–P2, checked against P3–P7. The throwaway
scripts (Python byte inspection, and a scratch `SvdFile::parse` test) were not kept.

## 1. Corpus inventory

| Kind | Files | Signature | Chunk models | Directory |
|---|---|---|---|---|
| XV, 12 chunks | 15 (`Roland Librarian-XV 5080 patches`, `JV-2080 Pr{A..E}/User … (via User XV)` ×6, Murray's `Beauty_1/2`, `WEST SIDE 05`, `Finian's Rainbow`, `JV17_128/256` ×2 copies each) | `SVD1` | `NEXY` + `XV88` | `PRFa MLTa PATa CTGa RHYa SEQa SYSa VIPa VIFa` (`NEXY`), then `PRFa SYSa FVRa` (`XV88`) |
| XV, 6 chunks | 3 (`Newshow`, `FNTM_001`, `FNTM_002`) | `SVD1` | `XV88` | `PRFa PATa RHYa SYSa FVRa CTGa` |
| XV-2020 | 1 (`2020user`) | `SVD1` | `NEXY` + `XV88` | `MLTa`(`NEXY`) `PATa RHYa`(`XV88`) `SYSb`(`NEXY`) |
| JV/XP | 13 (planet-groove ×9, `JV-1080_SP/` ×2, `2080EBnk`, `2080User`, `XP-80 User Patch 001-128`; the last three are in the XV Yahoo-group archive) | `SVD0` | `XP50`, `XP80` | `PRFa PATa RHYa` + 1–3 `SYSa`/`SYSb` |

The four `JV17_*.svd` files are two byte-identical pairs.

The XV-5080 manual lists what an `.SVD` holds: "Patch, Performance, Rhythm Set, System,
and Favorites List settings" (P3, PDF p. 190). That matches `PATa`, `PRFa`, `RHYa`, `SYSa`
and `FVRa` (64 × 3-byte entries). `CTGa` holds one byte per Patch (128 × 1). By the code
mnemonic it is probably a per-Patch category or group (inferred). On the XV-2020 file, `MLTa`
holds the 64 Performances: its names are the XV-2020 preset Performance list (P4, p. 135).

### Chunk sizes seen

| Chunk | Model | Entries × size | Seen in |
|---|---|---|---|
| `PATa` | `XV88` | 128 × **938** | 6-chunk files |
| `PATa` | `XV88` | 128 × **956** | 12-chunk files |
| `PATa` | `XV88` | 128 × **574** | XV-2020 |
| `PRFa` | `XV88` | 64 × 926 or 64 × 1064 | XV files |
| `RHYa` | `XV88` | 4 × 10808 or 4 × 10824 | XV files |
| `SYSa` | `XV88` | 1 × 394 | 6-chunk files |
| `FVRa` / `CTGa` | `XV88` | 64 × 3 / 128 × 1 | 6-chunk files |
| `MLTa` / `SYSb` | `NEXY` | 64 × 954 / 1 × 42 | XV-2020 |

In 12-chunk files every `NEXY` chunk is empty (count 0, entry size 0). So are the trailing
`SYSa`/`FVRa` (`XV88`).

## 2. Container: same as the Integra's

Checked on all 19 `SVD1` files (P1):

| Field | Integra (P6) | XV |
|---|---|---|
| Bytes 0–1 | u16 BE header length = directory end − 2 | Same: `00 4E` (4 chunks), `00 6E` (6), `00 CE` (12) |
| Bytes 2–5 | `SVD1` | `SVD1` |
| Bytes 6–15 | zero | zero |
| Directory entry | code(4) model(4) offset(u32 BE) size(u32 BE) | Same |
| Model ID | `MI69` | `XV88`, `NEXY` (both can appear in one file) |
| Zone header | count, entry size, `0x10`, `0` | Same, with `0x10`, `0` in every chunk |
| Chunk size | 16 + count × entry size | Same in every chunk |
| Empty chunk | entry size still set | **entry size 0** |
| End of file | end of last chunk | Same: no trailer |

`SvdFile::parse` (P5) rejects every XV file at the first directory entry with
`InvalidChunkType` (`PRFa` in 18 files, `MLTa` in `2020user`). It checks the chunk code
before the model, so it never reaches `InvalidModel`. Apart from the chunk-code and model
checks, the parse logic already fits these files: it tolerates an empty chunk with entry
size 0, because `count = 0` skips the entry-size check.

### `SVD0` (JV/XP era)

The 13 `SVD0` files share the header-length field and the directory layout, but:

- the signature is `SVD0`, so `SvdFile::parse` returns `InvalidMagic`;
- **there are no zone headers**: each chunk's data starts directly at its offset, and
  `PATa` is 50,560 bytes = 128 × 395, with no count/size prefix;
- the Patch data is not plain MSB-first bit-packing: across all 128 entries of
  `80TO1080.SVD`, no bit offset in the 395-byte entry decodes as a 12-character 7-bit name for
  more than 15 entries. The payload is scrambled or compressed in some way not identified here.

The JV/XP Patches are already well covered as SysEx (≈44k distinct, [#197](https://github.com/sigma/integral/issues/197)),
so this was not pursued.

## 3. XV Patch entry (`PATa`) encoding

The encoding is the one in [`02-encoding.md`](../svd/02-encoding.md): each SysEx byte is
stored using only the bits of its mask, MSB first. Nibblized (`#`) values take 4 bits per
nibble, and each section is byte-aligned. Signed fields under 7 bits use the same
re-centering bias (Octave Shift, 3 bits, decodes 61–67 correctly).

### Section layout (all three entry sizes share bytes 0–573)

| Bytes | Section | XV SysEx block (P7) | Bits | Bytes | Widths from |
|---|---|---|---|---|---|
| 0–55 | Common | `00 00 00` (79 B) | 445 | 56 | `PCMS_COMMON_PARAMS`, first 79 params (the Integra's 80th, `00 4F`, does not exist on XV) |
| 56–133 | Common MFX | `00 02 00` (145 B) | 618 | 78 | `MFX_PARAMS`, unchanged |
| 134–159 | Common Chorus | `00 04 00` (52 B) | 207 | 26 | **new**: Type 4, Level 7, Output Assign 2, Output Select 2, Parameter 1–12 × 16 (nibblized) |
| 160–201 | Common Reverb | `00 06 00` (83 B) | 333 | 42 | **new**: Type 4, Level 7, Output Assign 2, Parameter 1–20 × 16 (nibblized) |
| 202–233 | TMT | `00 10 00` (41 B) | 242 | 32 | `PCMS_PMT_PARAMS`, unchanged (same extra pad byte as on the Integra) |
| 234 + 85·(n−1) | Tone 1–4 | `00 20 00`…`00 26 00` (137 B) | 674 | 85 | `PCMS_PARTIAL_PARAMS`, first 137 SysEx bytes (the Integra's LFO step sequencer `01 09`–`01 19` does not exist on XV) |
| 574 | end of the XV-2020 entry | | | | |

The Chorus and Reverb Type fields take 4 bits, although their ranges are 0–2 and 0–4 (the
mask in P3 is wider than the range). This was found by brute force over 4-tuples of widths:
only `(4,7,2,2)`/`(4,7,2)` reproduce all 300 sampled entries. `(4,7,1,3)` also fits the
Chorus because Output Select never exceeds 1 in the sample. The P3 ranges (Output Assign 0–3,
Output Select 0–2) fix it at `(4,7,2,2)`.

### Tail beyond byte 574

| Entry size | Bytes 574–937 | Bytes 938–955 |
|---|---|---|
| 574 (XV-2020) | — | — |
| 938 (6-chunk files) | **Split Key area**, always present: all zero in 346 entries, filled in the 28 MULTI-PARTIAL entries (Common `00 0D` = 1) and in 10 4-TONE entries (stale data) | — |
| 956 (12-chunk files) | **Variable**: in 4-TONE entries an 18-byte block sits at 574 and the rest is zero. In the 7 MULTI-PARTIAL entries the 364-byte Split Key area comes first and the 18-byte block follows at 938. | (18-byte block or zero) |

The Split Key area is 364 bytes for the 88 keys (21–108) of the XV-5080 Patch Split Key
block (P3, PDF p. 297: Partial Number, Assign Type, Mute Group, five sends, Output Assign).
Its per-key packing was not decoded: MULTI-PARTIAL Patches are rejected anyway
([#193](https://github.com/sigma/integral/issues/193)). In 1,913 of 1,920 entries the 18-byte
block is the constant `02 00 FE 00 00 00 00 20 20 20 20 00 00 32 32 80 80 00`. It is
undecoded, and is probably extra Patch parameters from a later firmware or editor
(inferred). A reader can ignore it.

### Verification against SysEx twins (P2)

Every `.svd` entry was paired with each corpus SysEx Patch of the same name, and each
section was decoded with the layout above:

| Entry size | Entries with a same-name twin | ≥1 byte-exact twin | Closest twin off by 1–3 bytes | Closest twin off by more (mostly a different Patch with the same name) |
|---|---|---|---|---|
| 956 | 1,049 | **899** | 22 | 128 |
| 938 | 264 | **251** | 7 | 6 |
| 574 | 84 | **12** | 9 | 63 |

The near misses differ in single user-editable fields (Reverb Level, Octave Shift, Category,
Level). They are edits of a same-named Patch, not layout errors: the same fields decode
exactly in hundreds of other entries. The XV-2020 file is the user bank of a different
device, so most of its twins are factory Patches of the same name. Its 12 exact matches and
the shared section boundaries confirm the layout.

## 4. Is `.svd` import worth it?

Distinct non-INIT XV Patches in the 19 `SVD1` files: **1,491**. Of these, **542 have no
byte-exact SysEx twin** anywhere in the corpus: the SR-JV17 "converted to SRX" bank (256),
the five Murray's "In the Field" files (159), the XV-2020 user bank (116) and 11 from
`FNTM_001`. Everything else already exists as SysEx.

| Option | Cost | Gain |
|---|---|---|
| **Reuse the SVD reader with a new chunk spec** (recommended, late slice) | Make the container's model ID and chunk-code set parameters of `SvdFile::parse` (or add a sibling that shares the directory and zone code). Add one `SvdToneSpec` for XV `PATa`: existing `PCMS_COMMON_PARAMS[..79]`, `MFX_PARAMS`, two small new Chorus/Reverb tables, `PCMS_PMT_PARAMS`, `PCMS_PARTIAL_PARAMS[..137 bytes]`, sections 56/78/26/42/32/85×4. Decode to the 9 XV SysEx blocks and hand them to the XV SysEx converter. | +542 Patches (≈9 % over the ≈6.2k already available as SysEx) |
| Own reader | Duplicates the container code for no reason: the container is identical. | Same |
| Not worth it | None | Those 542 Patches stay unreachable. Users with XV-5080 or Fantom card dumps would have to convert them to SysEx elsewhere. |

The cheap path exists because the XV Patch decodes to **SysEx bytes**, not to a new model.
Under [#198](https://github.com/sigma/integral/issues/198) the XV reader converts SysEx
blocks straight into PCM Synth tone blocks, so an XV `.svd` reader is just another source of
those blocks. Only XV `PATa` needs a spec. `PRFa`, `RHYa` and the rest are outside "Patches
only".

### Constraint for `Pack::read`

[#198](https://github.com/sigma/integral/issues/198) has one `Pack::read` recognising SVD,
SysEx or SMF. XV and Integra `.svd` files share the `SVD1` signature, so **sniffing by magic
is not enough**. Detection must read the directory and route on the model ID (`MI69` →
Integra, `XV88`/`NEXY` → XV, signature `SVD0` → unsupported). The Integra parser itself
rejects foreign files already (`InvalidChunkType`/`InvalidModel`), so a misroute fails
loudly, not silently.

## Open questions

- What the 18-byte block in 956-byte entries holds, and which device writes the 12-chunk
  `NEXY` layout. Its `SEQa`/`VIPa`/`VIFa` chunks suggest a sequencer-equipped host, perhaps
  the original Fantom, which shares Model ID `00 10` ([#197](https://github.com/sigma/integral/issues/197)).
  This is inferred, with no primary source.
- The per-key packing of the 364-byte Split Key area (only needed if MULTI-PARTIAL Patches
  ever become importable).
- The `SVD0` payload encoding.
