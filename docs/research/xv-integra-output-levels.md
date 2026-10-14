# XV-series vs INTEGRA-7 output levels for the same Patch (research for #292)

Research for [#292](https://github.com/sigma/integral/issues/292), part of the Foreign patch
level map [#290](https://github.com/sigma/integral/issues/290). The question: is there documented
or reported evidence that the INTEGRA-7 plays a given raw Tone Level, Partial Level or Wave Gain
quieter than the XV-5080/5050/3080 or the JV-1080/2080? If so, the converter needs a mapping
correction (cause A in #290). If not, the gap comes from how the Integra presets are voiced
(cause B).

Our converter copies XV levels byte for byte and JV levels raw. Roland's own JV → XV conversions
did not rescale levels either ([`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)).

## TL;DR

- **No source shows a per-value gain difference.** Roland publishes no rated output level (dBu)
  or headroom figure for the JV-1080, XV-3080, XV-5050, XV-5080 or INTEGRA-7 [M1–M5, W1].
  The level parameters have the same ranges and the same wording on the XV and the Integra.
  Wave Gain is −6/0/+6/+12 dB on both, and "an increase of 6 dB doubles the gain" [M2, M6].
- **Output headroom is a global stage, not a per-Patch one.** The XV-5080 shipped with outputs
  that users found quiet. Roland left headroom for 128 voices, and a later OS added a ±12 dB
  Output Gain [F1, M2]. The XV-5050 and the INTEGRA-7 have the same System Output Gain
  (Integra: −12…+12 dB, front panel only) [M3, M1]. This setting scales presets and imported
  tones equally, so it cannot explain a gap between them on one unit.
- **A/B reports find tone and effect differences, not level offsets.** Sound On Sound compared
  the same preset on an XV-5080 and an INTEGRA-7: with the effects off, they "sounded similar"
  [F2]. A 16-Patch owner A/B found the differences mostly in effects and some voicing, with no
  level measurement [F3].
- **Roland did edit levels when it ported Patches.** SRX-08 "Next Stab" has Patch Level 20 on
  XV hardware but plays balanced on the Integra [F4]. So Integra preset ports are not
  byte-identical to the XV originals. That supports cause B, and only #291's device diff can
  measure it.
- **Typical Roland XV values (corpus, 4 sets):** Patch Level median 110–120, Tone Level median
  127, Wave Gain mostly +6 or +12 dB.

Verdict: no evidence for cause A. Cause B (Roland revoiced or edited the presets, and parts of
the signal chain are lost in conversion) is the open lead. [#291](https://github.com/sigma/integral/issues/291)
and [#294](https://github.com/sigma/integral/issues/294) settle it on the device.

## Sources

Graded by trust: **P** = Roland primary document, **D** = our own analysis of Roland factory
data, **R** = reputable press, **F** = forum report.

| Tag | Grade | Source | What it gives |
|-----|-------|--------|---------------|
| M1 | P | [INTEGRA-7 Owner's Manual](https://static.roland.com/assets/media/pdf/INTEGRA-7_OM.pdf) (local copy `~/Downloads/IntegraManuals/INTEGRA-7_eng04_W.pdf`), p. 48 (SYSTEM > SOUND: Master Level 0–127, Output Gain −12…+12 dB), troubleshooting ("Could the Output Gain be raised excessively?"), Specifications | Output Gain exists. Specifications list the jacks but no output level, impedance or dBu figure. |
| M2 | P | [XV-5080 Owner's Manual](https://static.roland.com/assets/media/pdf/XV-5080_OM.pdf), p. 119 (Output Gain −12, −6, 0, +6, +12 dB, "when … there are relatively few voices being sounded, boosting the output gain can let you attain the most suitable output level"), p. 172 (Wave Gain), Specifications | The same gain stage, in 6 dB steps. No dBu spec. |
| M3 | P | [XV-5050 Owner's Manual](https://static.roland.com/assets/media/pdf/XV-5050_OM.pdf), p. 107 (Output Gain, same text as M2), Patch Level "Specifies the volume of the Patch" | As M2. |
| M4 | P | [XV-3080 Owner's Manual](https://static.roland.com/assets/media/pdf/XV-3080_OM.pdf), p. 196 (Specifications) | No Output Gain parameter and no dBu spec. |
| M5 | P | [JV-1080 Owner's Manual](https://static.roland.com/assets/media/pdf/JV-1080_OM.pdf) (scanned; specification pages OCR'd) | Specifications list MIX OUT and OUTPUT 1/2 jacks with no output level. |
| M6 | P | [INTEGRA-7 Parameter Guide](https://static.roland.com/assets/media/pdf/INTEGRA-7_Parameter_Guide_e01_W.pdf), p. 50 (PCM Synth Wave Gain −6, 0, +6, +12 dB; Tone Level 0–127) and [`docs/midi/06-pcm-synth-tone.md`](../midi/06-pcm-synth-tone.md) | Same ranges and wording as M2. |
| W1 | P | [roland.com INTEGRA-7 specs page](https://www.roland.com/global/products/integra-7/specs/) | The jacks only. No output level. |
| D1 | D | Roland XV and JV factory Patches in `~/Downloads/integral-corpus/` (see its `MANIFEST.md`) and `~/Downloads/JV-1080_SP/`, scanned for this note | Value distributions (below). |
| D2 | D | [`crates/integral-core/tests/fixtures/init/pcm-synth-common.bin`](../../crates/integral-core/tests/fixtures/init/pcm-synth-common.bin), [`pcm-synth-partial.bin`](../../crates/integral-core/tests/fixtures/init/pcm-synth-partial.bin) | Integra INIT tone: Tone Level 127, Partial Level 127, Wave Gain 0 dB. |
| F1 | R | Sweetwater inSync, ["Output levels with Roland's XV-5080"](https://www.sweetwater.com/insync/output-levels-roland-xv-5080/), 2000-09-11 | Early XV-5080 outputs were "lower in level than the old 760" because "with 128 voices, Roland needed a good deal of headroom". A new OS would add a boost of "up to 12db". Retailer tech tip, not Roland. |
| F2 | R | Gordon Reid, ["Roland Integra 7"](https://www.soundonsound.com/reviews/roland-integra-7), *Sound On Sound*, Feb 2013 | Side-by-side XV-5080 and INTEGRA-7, both at factory settings, preset "128VoicePno": "the effects differed, so I switched them off. The two now sounded similar." He also compared real SRX-04/06/07/11 boards with the Integra versions: "most of the sounds were at least similar". He reports no level difference. |
| F3 | F | SoundworldA.D. et al., ["XV-5080 vs. I-7: More differences than you might think!"](https://forums.rolandclan.com/viewtopic.php?t=45792), Roland Clan Forums, 2012-10-25 | 16-Patch A/B (XV-5080 vs INTEGRA-7). Some Integra versions use different effects (added delay). Some sound "thin and 'tinny'", others "came alive". "even if the 5080 or SRX patches sound a little weaker comparatively". These are listening impressions, not level measurements. |
| F4 | F | Thread ["Roland SRX-08 Platinum Trax: question for owners of this board"](https://www.kvraudio.com/forum/viewtopic.php?p=8901050), KVR Audio, 2023–2024 | SRX-08 C338 "Next Stab": "level defaults at 20" on XV-2020 + SRX-08. cryophonik (2024-05-10): "perfectly balanced with the other sounds on the Integra 7, so they obviously corrected it when porting it over." |

Not found: any Roland support page, SRX board document or forum post that gives a dB offset
between the XV and Integra engines for the same raw value. No Roland SRX document covers output
level. The SRX documents in the corpus (wave cross-references) cover only waveforms.

## Findings in detail

### No published analog output level for any of the five units

None of the JV-1080, XV-3080, XV-5050, XV-5080 or INTEGRA-7 owner's manuals lists a nominal
output level, maximum output level or output impedance [M1–M5]. Neither does Roland's online spec
page [W1]. So there is no spec-sheet basis for an "the Integra is N dB quieter" claim, in either
direction.

### Same parameters, same meaning

The XV Patch layout is the INTEGRA-7 PCM Synth tone layout
([`xv-patch-format.md`](xv-patch-format.md)). Tone Level, Partial Level and Partial Output Level
are 0–127 on both. Wave Gain is a 2-bit value for −6/0/+6/+12 dB, and both manuals describe it
the same way [M2 p. 172, M6 p. 50]. No Roland document says the Integra engine maps these values
to a different gain.

### Output Gain: a global trim on both families

F1 is the only source that talks about level headroom. It describes the XV-5080 as quieter than
the S-760 sampler, by design, and says Roland added an output boost. The XV-5080/5050 manuals
then document that Output Gain (−12…+12 dB in 6 dB steps) [M2, M3]. The INTEGRA-7 has the same
parameter, −12…+12 dB, under SYSTEM > SOUND [M1 p. 48]. It has no SysEx address
([`system-settings-sysex.md`](system-settings-sysex.md), `OUTPUT_GAIN` has no address in
Roland's editor). Output Gain sits after the whole mix, so it moves a preset and an imported tone
together. It matters for an XV-vs-Integra comparison across two units. It cannot explain a gap
between two tones on one Integra.

### A/B reports

- F2 is the only careful side-by-side, by a reviewer who owned both units. With effects off, the
  first XV preset matched closely. He blames the differences on the effects and says nothing
  about level.
- F3 compares by ear. The reported differences are effect choices (delay added on the Integra)
  and timbre. "Weaker" is a listening impression, not a measured level.
- F4 is the one concrete level fact. An SRX-08 Patch that is quiet on XV hardware (Patch Level
  20) is balanced on the Integra, so Roland changed at least that Patch's levels when it ported.
  It is a single Patch, reported second-hand. Treat it as proof that ports *can* differ, not as
  a general rule.

### Typical Roland XV/JV level values (D1)

Distinct non-INIT Patches, decoded from checksummed DT1s. "Active" tones have their Tone Switch
(PMT Partial Switch) ON.

| Set | Patches | Patch Level median (p10–p90) | Active tones | Tone Level median (p10) | Tone Output Level median | Wave Gain −6 / 0 / +6 / +12 dB |
|-----|--------:|-----------------------------|-------------:|------------------------|-------------------------|-------------------------------|
| Roland SRX-03/04 XV Patch data (2001 FTP) | 684 | 110 (81–127) | 1,931 | 127 (80) | 127 | 2 % / 22 % / 44 % / 32 % |
| Roland 2002 SR-JV80 → XV sets (bwalk) | 3,121 | 120 (90–127) | 8,178 | 127 (70) | 127 | 0.5 % / 9 % / 52 % / 39 % |
| XV-5050 factory banks | 839 | 115 (70–127) | 2,297 | 127 (69) | 127 | 1 % / 28 % / 39 % / 32 % |
| Other XV Patches in the corpus | 2,354 | 115 (80–127) | 6,432 | 127 (60) | 127 | — |
| JV (Model `6A`), all sets | 41,580 | 117 (90–127) | 72,423 | 127 (62) | 127 (Mix/EFX Send) | 0.2 % / 8 % / 84 % / 8 % |

Excluded: 117 bwalk/"other" tones whose Wave Gain byte is out of range (64, 70), probably Rhythm
data under the same address prefix. Also excluded: XV-5050 tones with no Wave Gain byte. The
Integra INIT tone is Tone Level 127, Partial Level 127, Wave Gain 0 dB [D2].

So Roland's XV and JV factory voicing already runs hot. Most active tones are at full Tone Level
with +6 or +12 dB Wave Gain. Patch Level is usually 110–120, not 127. The repo has no INTEGRA-7
preset values to compare with ([#290](https://github.com/sigma/integral/issues/290) starting
facts). The Integra presets' Tone Level and Wave Gain are what
[#291](https://github.com/sigma/integral/issues/291) reads on the device.

### Leads for the quieter-import gap

These are not established. They are what the evidence leaves open:

1. **Voicing edits in the ports** (F4). If the Integra's own SRX or XV preset has higher Tone
   Level or Wave Gain than Roland's XV file of the same Patch, that is cause B.
   [#291](https://github.com/sigma/integral/issues/291) diffs this directly.
2. **Effects that don't survive conversion.** An XV Patch has its own Chorus and Reverb, and an
   Integra PCM Synth tone has none ([`xv-patch-format.md`](xv-patch-format.md)). Integra presets
   play with effects voiced for them, so an import loses the wet signal and sounds quieter and
   drier. F2 and F3 both name effects as the main audible difference.
3. **Output Gain** matters only when an XV unit and an Integra are compared side by side by ear.

## Method

- Owner's manuals: Roland's PDFs from `static.roland.com` (JV-1080, XV-3080, XV-5050, XV-5080),
  plus the cached INTEGRA-7 OM and Parameter Guide in `~/Downloads/IntegraManuals/`. Text came
  from `pdftotext -layout`. The JV-1080 manual is a scan, so its last 13 pages were OCR'd with
  tesseract. All were searched for `dBu`, `dBm`, `Nominal`, `Impedance`, `Output Gain`,
  `Output Level`, `Wave Gain` and `Patch Level`.
- Corpus scan (D1): a throwaway Python script, not kept. It walked every `.syx` and `.mid` file
  (SMF sysex events with continuations) and kept checksummed DT1s with Model `00 10` (XV) or
  `6A` (JV). It grouped them by Patch address and deduplicated identical Patches. It read XV
  Common `0E`, PMT switches `05`/`0E`/`17`/`20`, and Tone `00`, `0C`, `34`. For JV it read Common
  `2E` and Tone `00`, `05`, `65`, `7E` ([`jv-patch-format.md`](jv-patch-format.md)).
- Web: searches for owner A/B reports and Roland support pages. Sources F2–F4 were read in full.
  Page 2 of F3 could not be fetched (rate-limited).
- Nothing was checked on the device. Loading a preset onto a Part changes the Temporary Tone, so
  it needs the user's OK ([#290](https://github.com/sigma/integral/issues/290)).
