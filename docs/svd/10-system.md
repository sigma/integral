# System — SVD Mapping (SYSa)

Chunk type: `SYSa` | Entry size: **108 bytes** | Count: **1** | Model: `MI69` |
SysEx block: System Common (`02 00 00 00`, size `2F`)

`SYSa` appears **only in BACKUP files**, as the seventh directory entry. SOUND
files never carry it. See [SOUND vs BACKUP](11-sound-vs-backup.md).

**Confidence:**

- **Bits 0–220 (System Common): verified on real data, offline.** All 18
  backups decode to the documented factory defaults.
- **Bits 221–863: undocumented.** Only one interpretation is offered, and it is
  marked *inferred*.

## Entry Layout

| Entry bits | Bytes   | Content |
|------------|---------|---------|
| 0–220      | 0–27    | System Common `00 00`–`00 2E` (221 bits) |
| 221–863    | 27–107  | Undocumented system settings (see below) |

The last set bit observed is bit 847, the LSB of byte 105.

## System Common (bits 0–220)

| SysEx | Parameter | Bits | Section bit | Entry byte.bit | Range | Encoding | INIT |
|---|---|---|---|---|---|---|---|
| `00 00` | Master Tune | 16 | 0 | 0.0 | 24–2024 | 4 nibbles, MSB first | 1024 |
| `00 04` | Master Key Shift | 6 | 16 | 2.0 | 40–88 | bias | 64 |
| `00 05` | Master Level | 7 | 22 | 2.6 | 0–127 | raw | 127 |
| `00 06` | Scale Tune Switch | 1 | 29 | 3.5 | 0–1 | raw | 1 |
| `00 07` | (reserve 07) | 1 | 30 | 3.6 | – | raw (reserve) | 0 |
| `00 08` | (reserve 08) | 1 | 31 | 3.7 | – | raw (reserve) | 1 |
| `00 09` | (reserve 09) | 5 | 32 | 4.0 | – | raw (reserve) | 0 |
| `00 0A` | (reserve 0A) | 5 | 37 | 4.5 | – | raw (reserve) | 1 |
| `00 0B` | (reserve 0B) | 5 | 42 | 5.2 | – | raw (reserve) | 2 |
| `00 0C` | (reserve 0C) | 5 | 47 | 5.7 | – | raw (reserve) | 3 |
| `00 0D` | (reserve 0D) | 5 | 52 | 6.4 | – | raw (reserve) | 4 |
| `00 0E` | (reserve 0E) | 5 | 57 | 7.1 | – | raw (reserve) | 5 |
| `00 0F` | (reserve 0F) | 5 | 62 | 7.6 | – | raw (reserve) | 6 |
| `00 10` | (reserve 10) | 5 | 67 | 8.3 | – | raw (reserve) | 7 |
| `00 11` | Studio Set Control Channel | 5 | 72 | 9.0 | 0–16 | raw | 15 |
| `00 12` | (reserve 12) | 7 | 77 | 9.5 | – | raw (reserve) | 1 |
| `00 13` | (reserve 13) | 7 | 84 | 10.4 | – | raw (reserve) | 0 |
| `00 14` | (reserve 14) | 7 | 91 | 11.3 | – | raw (reserve) | 64 |
| `00 15` | (reserve 15) | 7 | 98 | 12.2 | – | raw (reserve) | 64 |
| `00 16` | (reserve 16) | 7 | 105 | 13.1 | – | raw (reserve) | 64 |
| `00 17` | (reserve 17) | 7 | 112 | 14.0 | – | raw (reserve) | 64 |
| `00 18` | (reserve 18) | 7 | 119 | 14.7 | – | raw (reserve) | 64 |
| `00 19` | (reserve 19) | 7 | 126 | 15.6 | – | raw (reserve) | 64 |
| `00 1A` | (reserve 1A) | 7 | 133 | 16.5 | – | raw (reserve) | 64 |
| `00 1B` | (reserve 1B) | 7 | 140 | 17.4 | – | raw (reserve) | 64 |
| `00 1C` | (reserve 1C) | 7 | 147 | 18.3 | – | raw (reserve) | 64 |
| `00 1D` | (reserve 1D) | 7 | 154 | 19.2 | – | raw (reserve) | 64 |
| `00 1E` | (reserve 1E) | 7 | 161 | 20.1 | – | raw (reserve) | 64 |
| `00 1F` | (reserve 1F) | 7 | 168 | 21.0 | – | raw (reserve) | 64 |
| `00 20` | System Control 1 Source | 7 | 175 | 21.7 | 0–97 | raw | 1 |
| `00 21` | System Control 2 Source | 7 | 182 | 22.6 | 0–97 | raw | 97 |
| `00 22` | System Control 3 Source | 7 | 189 | 23.5 | 0–97 | raw | 2 |
| `00 23` | System Control 4 Source | 7 | 196 | 24.4 | 0–97 | raw | 4 |
| `00 24` | Control Source | 1 | 203 | 25.3 | 0–1 | raw | 1 |
| `00 25` | System Clock Source | 1 | 204 | 25.4 | 0–1 | raw | 0 |
| `00 26` | System Tempo | 8 | 205 | 25.5 | 20–250 | 2 nibbles, MSB first | 120 |
| `00 28` | Tempo Assign Source | 1 | 213 | 26.5 | 0–1 | raw | 0 |
| `00 29` | Receive Program Change | 1 | 214 | 26.6 | 0–1 | raw | 1 |
| `00 2A` | Receive Bank Select | 1 | 215 | 26.7 | 0–1 | raw | 1 |
| `00 2B` | 5.1CH Center Speaker Switch | 1 | 216 | 27.0 | 0–1 | raw | 0 |
| `00 2C` | 5.1CH Sub Woofer Switch | 1 | 217 | 27.1 | 0–1 | raw | 0 |
| `00 2D` | 2CH Output Mode | 1 | 218 | 27.2 | 0–1 | raw | 0 |
| `00 2E` | (reserve 2E) | 2 | 219 | 27.3 | – | raw (reserve) | 0 |

"bias" means `sysex = svd + 64 − 2^(bits−1)`. The INIT column is the value in
all 18 backups.

Decoded, these are the documented factory defaults: Master Tune 1024 = 440 Hz,
Key Shift 0, Level 127, Studio Set Control Channel 16, Control 1–4 = CC01, AFT,
CC02, CC04, Control Source = STUDIO SET, Clock = MIDI, Tempo 120, Receive PC and
Bank Select ON.

The `(reserve 09)`–`(reserve 10)` cells hold 0, 1, …, 7. That is a non-trivial
table and needs a device check.

## Undocumented Area (bits 221–863)

- **Inferred:** bits 225–280 hold 4 × (7-bit note, 7-bit velocity) = (48, 127),
  (60, 127), (72, 127), (84, 127), that is C3/C4/C5/C6 at velocity 127. These are
  most likely the Preview 1–4 Note Number and Velocity settings (Owner's Manual
  p. 48, "Preview Settings").
- Bits 221–224 are `0110`. They may be the Preview Mode (SINGLE, CHORD, PHRASE)
  or other flags.
- Bytes 36–59 are mostly constant. Bytes 66–107 look like 21 big-endian 16-bit
  words: `0081 0003 001F 0003 001F`, then 15 × `0001`, then `0000`.
- Only two places **vary across the 18 backups**: bits 308, 311, 312, 319 and 320
  (bytes 38–40), and bits 376–399 (bytes 47–49). There are 7 distinct SYSa
  images. Bytes 47–49 take the values `A3 40 00` (D-50, Sounds of Summer 2014,
  Super Super Saw), `C1 C0 00` (Children of Bodom), `A2 03 89` (Euro Attack
  Synth, Synth Legends, Synth Legends Pure) and `00 00 00` (the rest). They don't
  track the pack dates.

A writer must copy bits 221–863 through unchanged until they are understood.

## Evidence

- All 18 backups are identical over bits 0–220, and every decoded value is the
  documented factory default.
- Master Key Shift needs the 6-bit bias (stored 32 = SysEx 64) to land on its
  default.

## Open Questions / Device Check

| Unknown | Device check that settles it |
|---------|------------------------------|
| Bits 221–863 (preview settings, bytes 38–40 and 47–49, the 16-bit word table) | Change one SYSTEM-menu item at a time (Preview Mode/notes/velocities, USB driver, LCD contrast, …), run a USB BACKUP after each change, and diff the SYSa images. |
| `(reserve 09)`–`(reserve 10)` = 0…7 and `(reserve 12)` = 1 | RQ1 `02 00 00 00` size `2F`, and check whether the reply bytes equal the decoded values (0…7, 1, 0, 64 × 12). |
