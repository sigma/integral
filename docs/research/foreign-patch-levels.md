# Foreign patch levels against INTEGRA-7 presets (research for #291 and #294)

Device data for the map
[Find out why imported Foreign patches play quieter than INTEGRA-7 presets](https://github.com/sigma/integral/issues/290):

- [Compare a converted SRX Patch's level parameters with the Integra's own SRX tone](https://github.com/sigma/integral/issues/291)
- [Measure the loudness of converted Patches against their Integra counterparts](https://github.com/sigma/integral/issues/294)

Taken on 2026-10-14 on the owner's INTEGRA-7 (device ID 10H, SRX-03/05/07/08 in Expansion slots
A–D), Studio Set "Test Set", Part 1 (receive channel 1). Every tone below was loaded into Part 1's
Temporary tone and read back from the device, so both sides of each diff share one byte layout.
Part 1 and the Temporary Studio Set were restored afterwards; a read-back matched the snapshot.

## TL;DR

- **Roland's INTEGRA-7 ports lower Tone Level by about ×0.85.** This holds for 24 of 26 pairs.
  The other two were re-voiced. Integra presets are *quieter* than the XV/JV originals they came
  from, by about 2.9 dB. Tone Level follows a square law (70 → 108 measured +7.6 dB,
  40·log10(108/70) = 7.5 dB).
- **All other gain fields match byte for byte** in all 26 pairs: Partial Level, Partial Output
  Level, Wave Gain, Structure/Booster, TVA Envelope Levels, TVA Level Velocity Sens/Curve, Bias
  and Partial switches. The MFX type and MFX levels match too, except in a few re-voiced ports.
- **Two converter defects make imports quieter (cause A):**
  1. **TFX Switch is written OFF** (Common 2 `00 33`). Every Integra preset carries ON. OFF
     bypasses the tone's MFX. The value comes from `PcmSynthCommon2::default()`
     (`crates/integral-core/src/librarian/foreign/pcm_tone.rs`, `fn common2`). For MFX-dependent
     Patches the effect is large: JV "Dist Gtr 1" is −11 dB with the MFX bypassed.
  2. **Partial Chorus/Reverb Send take the wrong XV pair.** Roland's ports put the XV
     "(MFX)" send pair (Tone `00 0D`/`00 0E`, usually 127/127) into Partial Chorus/Reverb Send
     `00 0F`/`00 10`. They zero the reserves or keep the old values there. The converter copies
     the "(non MFX)" pair byte for byte, often 0/0. This settles open doubt 2 of
     [`xv-patch-format.md`](xv-patch-format.md) in favour of its recommendation. With the Part
     sends at 75/85, a preset gains +4.0–4.5 dB from Chorus/Reverb. A conversion with sends 0/0
     gains nothing.
- **Cause B (Integra presets voiced hotter) is refuted.** With TFX ON and the same sends, a
  faithful conversion is about 3 dB *louder* than Roland's port.
- **The opt-in `--apply-effects` adds ≤ 0.4 dB to "United !"** on its own, because the Partial
  sends it feeds are 0–50. With the "(MFX)" pair in the Partial sends, it adds +5.0 dB.

## Method

- Pairs: a Foreign Patch and an Integra preset with the same name.
  - **SRX-03**: the fixture `srx03ap2-001` ("United !", from Roland's SRX-03 *additional* patch
    data) has **no counterpart**. None of the 128 Patches in `SRX03AP2.mid` is among the 128
    SRX-03 tones of the INTEGRA-7 (catalog query `0F 00 04 02`, MSB 93 LSB 2). The SRX-03
    factory Patches were found as XV-5080 Patches instead: the 9 Performance Part Patches of
    Roland's demo `srx03_demo_b/srx03_x2.mid` (corpus `archive-org-RolandXVYahooGroup/…/SRX
    expansion cards demos/srx03-midi.zip.unpacked`). 7 of them match an SRX-03 preset by name.
    They are demo-song copies, so Roland may have edited them for the song.
  - **XV-5080 factory**: 15 of the 134 name matches between `XV 5080 All.syx` User Patches
    (`30 nn`) and the INTEGRA-7 PCM Synth presets (`docs/sounds/pcm-synth-preset.json`).
  - **JV-2080 factory**: `Roland JV-2080 PrB/PrC Patch 001-128 (via User).syx` against PCM Synth
    presets 129–384 (MSB 87, LSB 65/66). These Patches have the same names and order.
- Each Patch was cut out of its file into a lone `.syx`, so that `integral pack import <file>
  --part 1` loads it into Temporary and never into User memory.
- Read-back: `raw-read` of `19 00 00 00` (80), `02 00` (145), `10 00` (41), `20/22/24/26 00`
  (154 each) and `30 00` (60). The sizes are 7-bit: `00 00 01 11` is 145 bytes, not 273.
- Audio: RØDECaster multitrack channels 2/3 at a fixed gain (#293), C4, velocity 100, held 2 s,
  5 s take, integrated LUFS (`ebur128`) and astats RMS. Repeat takes agreed within 0.2 LUFS.
  One distorted conversion (Xtremities with its MFX bypassed) varied by 1 dB.
- **Test Set has Motional Surround ON** (`18 00 08 00` = 1). That disables the Studio Set
  Reverb: a reverb send of 127 left the release tail at −80 dBFS. MS ON also lowers the overall
  level by about 6 dB. The dry battery ran with MS ON and Part sends 0. The wet battery ran with
  MS OFF and the Test Set's own Part sends (Chorus 75, Reverb 85). MS was restored to ON.

## Tone Level and sends (26 pairs)

`TFX` and `MFX` columns are Integra/converted. Sends are Partial Chorus,Reverb for partials 1/2/3/4.

| Patch | Foreign Tone Level | Integra Tone Level | ratio | TFX | MFX type | Integra sends | converted sends |
|---|---|---|---|---|---|---|---|
| Bigger Brass (SRX-03) | 80 | 67 | 0.838 | 1/0 | 1/1 | 127,127/127,127/127,127/127,127 | 127,127/127,127/127,127/127,127 |
| 42nd Bass (SRX-03) | 127 | 108 | 0.850 | 1/0 | 0/0 | 127,127/127,127/127,127/127,127 | 0,0/0,0/0,0/0,0 |
| Fingr Tap'n (SRX-03) | 90 | 76 | 0.844 | 1/0 | 30/30 | 127,127/127,127/127,127/127,127 | 0,0/0,0/0,0/0,42 |
| Velo OB Str (SRX-03) | 110 | 93 | 0.845 | 1/0 | 36/36 | 127,127/127,127/127,127/127,127 | 0,51/0,51/0,51/0,51 |
| Bright Nyle (SRX-03) | 120 | 101 | 0.842 | 1/0 | 0/0 | 127,127/127,127/127,127/127,127 | 0,0/0,0/0,0/0,0 |
| Mod Sfz (SRX-03) | 127 | 108 | 0.850 | 1/0 | 0/0 | 127,46/127,46/127,46/127,46 | 127,46/127,46/127,46/127,46 |
| Night Blade (SRX-03) | 75 | 63 | 0.840 | 1/0 | 0/0 | 127,127/127,127/127,127/127,127 | 0,120/0,120/0,120/0,120 |
| Xtremities (XV) | 90 | 76 | 0.844 | 1/0 | 29/29 | 0,0/0,0/0,0/0,0 | 0,0/0,0/0,0/0,0 |
| Ambi Voices (XV) | 100 | 84 | 0.840 | 1/0 | 0/36 | 0,110/0,90/0,127/0,110 | 0,0/0,0/0,0/0,0 |
| Pianomonics (XV) | 116 | 98 | 0.845 | 1/0 | 1/0 | 0,0/0,91/0,37/127,127 | 0,0/0,91/0,37/127,127 |
| 5ths in 4ths (XV) | 127 | 93 | 0.732 | 1/0 | 44/44 | 46,6/46,6/46,6/46,6 | 46,6/46,6/46,6/46,6 |
| Boogie Organ (XV) | 80 | 67 | 0.838 | 1/0 | 30/30 | 0,0/0,0/0,0/0,0 | 0,0/0,0/0,0/0,0 |
| Soaring Hrns (XV) | 80 | 67 | 0.838 | 1/0 | 34/34 | 127,127/127,127/127,127/127,127 | 0,0/0,0/0,0/0,0 |
| COSM Searing (XV) | 127 | 108 | 0.850 | 1/0 | 30/30 | 0,0/0,0/0,0/36,127 | 0,0/0,0/0,0/36,127 |
| Suite Combo (XV) | 127 | 108 | 0.850 | 1/0 | 26/26 | 0,40/0,40/0,0/127,127 | 0,40/0,40/0,0/0,40 |
| Harmonica XV (XV) | 85 | 72 | 0.847 | 1/0 | 0/36 | 0,60/127,127/127,127/127,127 | 5,40/0,0/0,0/0,0 |
| Byzantine (XV) | 127 | 108 | 0.850 | 1/0 | 0/0 | 127,127/127,127/127,127/127,127 | 0,60/0,60/0,60/0,60 |
| Halographix (XV) | 127 | 108 | 0.850 | 1/0 | 43/43 | 127,127/127,127/127,127/127,127 | 0,100/0,100/0,100/0,100 |
| 3rdTeenChord (XV) | 95 | 97 | 1.021 | 1/0 | 9/9 | 16,42/16,42/16,42/16,42 | 16,42/16,42/16,42/16,42 |
| Atmospherics (XV) | 127 | 108 | 0.850 | 1/0 | 6/6 | 127,127/127,127/45,127/45,127 | 0,0/0,0/45,127/45,127 |
| ClassicJPpad (XV) | 79 | 66 | 0.835 | 1/0 | 0/0 | 127,127/127,127/127,127/127,127 | 0,60/0,60/0,0/0,0 |
| Wood Symphny (XV) | 70 | 59 | 0.843 | 1/0 | 1/1 | 127,127/127,127/0,127/0,127 | 33,62/33,62/0,127/0,127 |
| Dist Gtr 1 (JV PrB 1) | 70 | 108 | 1.543 | 1/0 | 29/29 | 127,127/127,127/127,127/127,127 | 127,127/127,127/127,127/127,127 |
| Super JX Bs (JV PrB 41) | 127 | 108 | 0.850 | 1/0 | 32/32 | 127,127/107,44/127,127/78,47 | 107,44/107,44/107,0/78,47 |
| Brass Sect (JV PrC 4) | 107 | 90 | 0.841 | 1/0 | 34/34 | 127,127/127,127/127,127/127,127 | 0,97/0,97/0,72/0,72 |
| Deep Strings (JV PrC 61) | 85 | 72 | 0.847 | 1/0 | 27/27 | 127,127/127,127/127,127/127,127 | 0,0/0,0/0,0/0,0 |

Where the Integra has 127,127 and the conversion has something else, the Integra tone's reserves
`00 0D`/`00 0E` hold 0,0 (or the old "(non MFX)" value). The conversion holds 127,127 there. The
pair was swapped, not re-levelled.

Integra Tone Level is about `XV × 0.85`, rounded down at times (80 → 67, 79 → 66, 100 → 84).
The outliers are 5ths in 4ths (127 → 93), 3rdTeenChord (95 → 97) and JV Dist Gtr 1 (70 → 108).
JV Patch Levels follow the same ×0.85 when Roland did not re-level them.

### Other differences in Roland's ports (not gain)

- MFX Control Assign 1/2: the Integra has 1–3 where the conversion has 0. Seen in 8 pairs, including all 4 JV ones.
- JV only: TVF Cutoff Velocity Curve. The Integra has 0 (FIXED) on every partial; the
  conversion has 1. Legato Retrigger is 0 on the Integra and 1 in the conversion (Dist Gtr 1, Deep Strings,
  Brass Sect). Matrix Control Source is CTRL1 (98) on the Integra where the conversion has CC01
  (MC 1 in Dist Gtr 1, Brass Sect and Super JX Bs; MC 4 in Dist Gtr 1).
- Wave Group ID 1 on INT partials where the conversion writes 0 (Pianomonics, Soaring Hrns,
  Harmonica XV, Halographix, ClassicJPpad).
- Unused partials: the Integra zeroes their Wave Number. The conversion keeps the source Wave.
- MFX re-voiced: Ambi Voices and Harmonica XV became THRU on the Integra (MFX 36 in the
  source). Pianomonics went from 0 to 1. Fingr Tap'n, Bright Nyle and Night Blade have edited
  MFX parameters. JV Dist Gtr 1's DISTORTION went from Drive 127/Level 95 to Drive 32/Level
  105, and Xtremities' Level went from 80 to 110.
- Common 2 Phrase Number is set on the Integra and 0 in the conversion. Tone Category differs
  in a few pairs.

## Loudness, dry (MS ON, Part sends 0)

Integrated LUFS. Δ is converted − Integra, so a positive Δ means the conversion is louder.
"TFX ON" is the conversion with only `19 00 30 33` set to 1. "Expected" is 40·log10 of the Tone
Level ratio.

| Pair | Integra | converted | Δ | converted, TFX ON | Δ | expected |
|---|---|---|---|---|---|---|
| Bigger Brass (SRX-03) | −34.1 | −32.2 | +1.9 | −30.9 | +3.2 | +3.1 |
| Velo OB Str (SRX-03) | −41.8 | −39.0 | +2.8 | −38.9 | +2.9 | +2.9 |
| Bright Nyle (SRX-03) | −43.3 | −39.9 | +3.4 | −40.4 | +2.9 | +3.0 |
| Xtremities (XV) | −27.2 | −20.1 | +7.1 | −27.8 | −0.6 | +3.0 |
| Boogie Organ (XV) | −32.4 | −30.1 | +2.3 | −29.2 | +3.2 | +3.1 |
| Dist Gtr 1 (JV) | −26.3 | −45.0 | −18.7 | −34.6 | −8.3 | −7.5 |
| Brass Sect (JV) | −29.2 | −26.4 | +2.8 | −26.2 | +3.0 | +3.0 |
| Deep Strings (JV) | −38.8 | −39.1 | −0.3 | −34.4 | +4.4 | +2.9 |

RMS (channel 2, dB) followed LUFS within 0.5 dB. Xtremities' remaining −0.6 comes from its
DISTORTION Level, which Roland raised from 80 to 110.

Dist Gtr 1, step by step on the converted tone (LUFS): imported −45.0. TVF velocity curve 0:
−44.9. Tone Level 108: −37.3. Drive 32 and Level 105: −37.7. MFX Control Assign, Matrix
Sources and Partial 4's Wave: −37.5. **TFX ON: −26.3**, the same as the Integra preset (−26.3).

## Loudness, wet (MS OFF, Part sends Chorus 75 / Reverb 85, Test Set Chorus/Reverb)

| Pair | Integra dry | Integra wet | converted wet | TFX ON, dry | TFX ON, wet | wet gain, Integra | wet gain, TFX ON |
|---|---|---|---|---|---|---|---|
| Bigger Brass | −30.9 | −26.4 | −23.9 | −27.9 | −23.4 | +4.5 | +4.5 (same sends) |
| Dist Gtr 1 | −22.6 | −20.9 | −33.9 | −30.8 | −29.4 | +1.7 | +1.4 (same sends) |
| Brass Sect | −25.8 | −21.4 | −22.4 | −22.7 | −22.0 | +4.4 | +0.7 (sends 0,97/0,72) |
| Deep Strings | −32.8 | −28.8 | −35.0 | −29.1 | −30.0 | +4.0 | −0.9 (sends 0,0) |

As imported, against the preset in this realistic setting: Bigger Brass +2.5, Brass Sect −1.0,
Deep Strings −6.2, Dist Gtr 1 −13.0 dB.

## "United !" and `--apply-effects` (MS OFF)

| Take | LUFS |
|---|---|
| Part sends 0 (dry) | −33.2 |
| Part sends 75/85, Test Set Chorus/Reverb | −33.4 |
| Part sends 75/85, `--apply-effects` (Chorus CHORUS 127, Reverb SRV HALL 127) | −33.0 |
| as above, TFX ON | −32.1 |
| as above, Partial sends 127/127 (the "(MFX)" pair) | −27.0 |
| Part sends 0, TFX ON, Partial sends 127/127 | −32.0 |

Its converted Partial sends are 0,30/0,30/50,80/0,80 and its MFX sends are 0/20, so the Patch's
own Chorus and Reverb barely reach the Studio Set. With the "(MFX)" pair the wet signal is +5.0
dB.
