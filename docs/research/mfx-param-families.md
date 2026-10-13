# FX parameters by Parameter family (research for [#158](https://github.com/sigma/integral/issues/158))

Answers [#158](https://github.com/sigma/integral/issues/158), part of the map
[#156](https://github.com/sigma/integral/issues/156): how do the parameters of every MFX type,
the Chorus and Reverb types and the drum-kit Comp+EQ sort into the **Parameter families**
(`CONTEXT.md` → *Editing*: Pitch / Source, Filter / Tone, Amp / Level, Modulation,
Time / Space, Common / Global)? The ambiguous cases were decided in [#159](https://github.com/sigma/integral/issues/159): every recommendation stands except
non-delay Feedback (cluster 8), which moved from M to F.

## TL;DR

- **771 parameters** classified: 725 MFX (67 types), 22 Chorus (3 types), 10 Reverb
  (2 layouts covering 6 types), 14 per Comp+EQ unit (×6 identical units).
- **509 (66 %) sort cleanly** with about 70 name rules plus a short list of per-type exceptions.
  **262 (34 %) are ambiguous.** They fall into 21 clusters. Five clusters cover 154 of the 262:
  dry/wet **Balance** (41), the 16-step **Step** patterns (34), amp/speaker/mic **voicing
  selectors** (29), the **Drive** cluster (28) and **Pan** (22).
- With the [#159](https://github.com/sigma/integral/issues/159) decisions applied to every ambiguous case: Filter / Tone 265,
  Amp / Level 176, Modulation 165, Time / Space 115, Common / Global 40, Pitch / Source 10.
- **Pitch / Source is almost empty for effects.** Only the Pitch Shifters' Coarse/Fine are clear
  cases, plus Ring Mod Frequency and the lo-fi controls.
- **Bias does not exist** on the INTEGRA-7. No MFX, Chorus or Reverb parameter has that name in the
  repo tables or the Parameter Guide. **Switch/Note/msec slots** are not in the repo's MFX tables
  at all (see [#30](https://github.com/sigma/integral/issues/30)). They
  take the family of the value they select.

## Sources (highest trust first)

| # | Source | Used for |
|---|--------|----------|
| R1 | `crates/integral-core/src/mfx_params.rs` (`mfx_type_def(0..=67)`) | Every MFX type's parameter list: names, ranges, units. Dumped programmatically. |
| R2 | `crates/integral-core/src/fx_params.rs` (`chorus_params`, `reverb_params`) and [`docs/midi/11-fx-parameters.md`](../midi/11-fx-parameters.md) | Chorus and Reverb per-type parameters (layout established in [fx-param-layout.md](fx-param-layout.md)). |
| R3 | [`docs/midi/07-pcm-drum-kit.md`](../midi/07-pcm-drum-kit.md) § *Common Comp/EQ* and `params::comp` / `params::comp_eq` | The 14 parameters of a Comp+EQ unit. SN Drum uses the same block. |
| P1 | Roland *INTEGRA-7 Parameter Guide* e01, pp. 73–99 ([PDF](https://static.roland.com/assets/media/pdf/INTEGRA-7_Parameter_Guide_e01_W.pdf)) | What each parameter *does*: the "Explanation" column. This is the evidence for the ambiguous cases. |
| P2 | [`docs/midi/06-pcm-synth-tone.md`](../midi/06-pcm-synth-tone.md), [`08-supernatural-synth-tone.md`](../midi/08-supernatural-synth-tone.md) | Where the tone editors put shared concepts (e.g. SN-S `AMP Pan`). Cross-editor consistency. |

## Method

1. **Dump.** A throwaway integration test iterated `mfx_type_def(0..=67)` and wrote
   `(type, index, name, min, max, unit)` for all 724 MFX parameters. The test has since been
   deleted. Chorus/Reverb come from the `fx_params.rs` tables (the UI surface: a `synced`
   Rate/Delay counts as one control). Comp+EQ comes from R3.
2. **Classify.** A throwaway script did this in three steps:
   - It removed a *stage prefix* (`Chorus `, `Flanger `, `Delay `, `OD `, `Amp `, `Enhancer `,
     `Touch Wah `, `Auto Wah `, `TWah `, `AutoWah `, `Pre Amp `, `Overdrive `, `Distortion `, `Pitch1 `, `Pitch2 `, `Delay 1 `–`Delay 4 `,
     `Mid1 `, `Mid2 `, `Boost `, `Woofer `, `Tweeter `, …). For example,
     `Flanger Rate` became `Rate` and `Delay HF Damp` became `HF Damp`.
   - It looked up the remaining base name in an exact-name rule table.
   - It applied per-type overrides wherever the same name means different things in different
     types. Example: `Feedback` is comb resonance in a Flanger but echo repeats in a Delay.
3. **Evidence.** Each ambiguous case was checked against the P1 "Explanation" text.
4. **Count.** "Firm" means exactly one family fits. "Ambiguous" means two or more defensible
   families. Ambiguous cases also carry the decided family so totals can be shown either way.

Caveats:

- The counts are over the **parameters the Parameter Guide shows** (R1 flattens the
  Hz/Note and msec/Note three-slot groups into one entry). They are not over the 32 wire slots.
  If [#30](https://github.com/sigma/integral/issues/30) expands those groups, the extra
  switch/note slots follow the rule in [Switch / Note / msec slots](#switch--note--msec-slots).
- Hidden Reverb slots (the "hidden level" and the GM2 "hidden pre delay") are excluded.
- Each Comp+EQ unit is counted once. A UI shows six identical units.
- The appendix and counts were re-matched to the MFX tables regenerated in
  [7738602](https://github.com/sigma/integral/commit/7738602546ac), which match the device slot
  layout: renamed stage prefixes (`Pre Amp `, `TWah `, `AutoWah `, `Overdrive `, `Distortion `),
  `Brake` moved from Rotary 1 to Rotary 2, Humanizer gained and Rotary 1 lost its 2-band EQ,
  EP AmpSim -> Tremolo has `Speed`/`Duty` instead of `Rate`/`Mod Wave`, EP AmpSim -> Delay gained
  `Delay Acceleration`, and the wahs' `Filter Type` is now `Mode` (LPF/BPF, still F).

Family codes used below: **P** Pitch / Source, **F** Filter / Tone, **A** Amp / Level,
**M** Modulation, **T** Time / Space, **C** Common / Global. A `?` marks an ambiguous case.

## Counts

Each cell reads "firm (with the [#159](https://github.com/sigma/integral/issues/159) decisions applied)".

| Scope | Params | Pitch / Source | Filter / Tone | Amp / Level | Modulation | Time / Space | Common / Global | of which ambiguous |
|---|--:|--:|--:|--:|--:|--:|--:|--:|
| MFX (67 types) | 725 | 6 (10) | 146 (249) | 110 (166) | 106 (160) | 70 (102) | 34 (38) | 253 |
| Chorus (3 types) | 22 | 0 (0) | 3 (6) | 4 (5) | 5 (5) | 6 (6) | 0 (0) | 4 |
| Reverb (2 layouts) | 10 | 0 (0) | 1 (3) | 0 (0) | 0 (0) | 6 (7) | 0 (0) | 3 |
| Comp+EQ (one unit) | 14 | 0 (0) | 7 (7) | 3 (5) | 0 (0) | 0 (0) | 2 (2) | 2 |
| **Total** | 771 | 6 (10) | 157 (265) | 117 (176) | 111 (165) | 82 (115) | 36 (40) | 262 |

Filter / Tone dominates because nearly every MFX type ends in a 2-band EQ (`Low Gain`/`High Gain`:
74 parameters).

## Rules (MFX)

Applied to the base name after the stage prefix is removed. The number in brackets is how many
times the rule fires across the 67 types. Exceptions are listed in the next section.

| Family | Firm rules | Ambiguous rules (decision shown) |
|---|---|---|
| **P** | Coarse (3), Fine (3) | Sample Rate, Bit Down |
| **F** | High Gain (37), Low Gain (37), Bass (10), Treble (10), Filter Type (5), Middle (5), Tone (4), Cutoff Freq (3), Freq (2), Gain (3, Boost/Mid EQ), Peak (3), Q (2), Band1–8, Bright, Presence, Filter Slope, High Freq, Low Freq, Pre Filter Type, Post Filter Type, Post Filter Cutoff, Mode (2, wah LPF/BPF), Vowel1/2, Width (Boost), Filter, Filter Gain | Drive (17, incl. OD Drive), HF Damp (15), Speaker Type (11), Manual (8), Resonance (6), Mic Setting (2), Filter Resonance, Feedback in a Flanger stage (7), Cross Feedback (2) |
| **A** | Level (87, incl. every `X Level`), Master (5), Threshold (3), Direct Level (2), Mic Level (2), Post Gain (2), Left/Right/Center Level, Volume, Ratio | Balance (41), Mix (2) |
| **M** | Rate (35), Depth (32), Slow Speed (6), Fast Speed (6), Phase (5, LFO L/R phase in °), Trans Up/Down (8), Mod Wave (2), Acceleration (2, Rotary), Step Rate (2), Depth Deviation, Shuffle, Duty | Polarity (5), Speed (3, Rotary SLOW/FAST), Brake (2), Input Sync Sw/Threshold (4), Step 01–16 (32) |
| **T** | Pre Delay (20), Feedback in a delay stage (14), Time / Delay Time / Delay N Time (21), Delay Left/Right/Center (7), Feedback Mode (2), Spread (2), Center Feedback, Pre Delay Deviation, Delay | Pan (21, incl. every `X Pan`), Pan Deviation, Phase Left/Right |
| **C** | Switch (17), Sw (9), Speaker Sw (5), Drive Switch/Sw (3) | — |

## Per-type exceptions (MFX)

These are names whose family depends on the type, or that the rules above don't cover.

| Param | Types | Family | Candidates | Note |
|---|---|---|---|---|
| Acceleration | Time Ctrl Delay, EP AmpSim -> Delay | T | T/M | speed of delay-time glide |
| Amp Type | Overdrive, Distortion, OD/DS -> TouchWah, OD/DS -> AutoWah | F | F/C | amp voicing |
| Boost Frequency | Low Boost | F | — | boost centre, not Ring Mod's carrier |
| Attack | Gate, Compressor | A | A/T | dynamics envelope time |
| Attack | Slicer | M | M/A | per-step envelope of the slicer pattern |
| Attack | Step Filter | M | M/F | per-step envelope of the filter pattern |
| Color | Phaser 2 | M | M/F | modulation character |
| Drive Type | OD/DS -> TouchWah, OD/DS -> AutoWah | F | F/C | OD/DS voicing |
| Enhancer Mix | Enhancer -> Chorus, Enhancer -> Flanger, Enhancer -> Delay | A | A/F | level of overtones |
| Enhancer Sens | Enhancer -> Chorus, Enhancer -> Flanger, Enhancer -> Delay | F | F/A | amount of overtones generated |
| Feedback | Flanger, Step Flanger | F | F/M/T | flanger feedback = comb resonance |
| Feedback | Pitch Shifter | T | T/P | delay-line feedback |
| Frequency | Ring Modulator | P | P/M | carrier oscillator |
| Hold | Gate | A | A/T | dynamics envelope time |
| LoFi Type | LOFI Compress | P | P/F/C | lo-fi algorithm |
| Manual | Humanizer | F | F/M | vowel morph position |
| Mix | Enhancer | A | A/F | level of overtones |
| Mix | Phaser 1, Step Phaser, Multi Stage Phaser, Infinite Phaser | A | A/C | level of phase-shifted sound |
| Mode | Gate | C | C/A | GATE/DUCK |
| Mode | Infinite Phaser | M | M/C | PG p. 77: higher values give a deeper phaser effect |
| Mode | Phaser 1, Step Phaser, Multi Stage Phaser | C | C/F | number of phaser stages |
| Mode | Slicer | M | M/C | LEGATO/SLASH: shape of the slice pattern |
| OD Gain | EP AmpSim -> Tremolo, EP AmpSim -> Chorus, EP AmpSim -> Flanger, EP AmpSim -> Phaser, EP AmpSim -> Delay, Rotary 3 | F | F/A/P | drive amount |
| Pitch1 Feedback | 2Voice Pitch Shifter | T | T/P | delay-line feedback |
| Pitch2 Feedback | 2Voice Pitch Shifter | T | T/P | delay-line feedback |
| Pre Amp Gain | Guitar Amp Simulator, GuitarAmpSim -> Chorus, GuitarAmpSim -> Flanger, GuitarAmpSim -> Phaser, GuitarAmpSim -> Delay | F | F/A/P | preamp gain LOW/MID/HIGH |
| Pre Amp Type | Guitar Amp Simulator, GuitarAmpSim -> Chorus, GuitarAmpSim -> Flanger, GuitarAmpSim -> Phaser, GuitarAmpSim -> Delay | F | F/C | amp/cabinet voicing |
| Release | Gate, Limiter | A | A/T | dynamics envelope time |
| Sens | Auto Wah | M | M/F | envelope follower depth |
| Sens | Enhancer | F | F/A | amount of overtones generated |
| Sens | Ring Modulator | M | M/A | "amount of frequency modulation applied" (PG p. 78) |
| Separation | Rotary 1 | T | T/M | no PG description; read as stereo spread |
| Speed | Infinite Phaser, Phaser 3, EP AmpSim -> Tremolo | M | — | LFO speed, not the Rotary SLOW/FAST switch |
| Threshold | Reverse Delay | T | T/A | level that triggers reverse playback |
| TWah Sens | OD/DS -> TouchWah | M | M/F | envelope follower depth |
| Tremolo Separation | Tremolo Chorus | T | T/M | "spread of the tremolo effect" (PG p. 82) |
| Type | EP AmpSim -> Tremolo, EP AmpSim -> Chorus, EP AmpSim -> Flanger, EP AmpSim -> Phaser, EP AmpSim -> Delay | F | F/C | amp voicing |

## Chorus, Reverb, Comp+EQ (full)

| Scope | Param (UI name) | Family | Candidates |
|---|---|---|---|
| Chorus: Chorus | Filter, Cutoff | F | — |
| | PreDly | T | — |
| | Rate, Depth, Phase | M | — |
| | Fdbk | F | F/M/T |
| Chorus: Delay | Dly L, Dly R, Dly C, C Fdbk | T | — |
| | HFDamp | F | F/T |
| | Lv L, Lv R, Lv C | A | — |
| Chorus: GM2 Cho | PreLPF | F | — |
| | Level | A | — |
| | Fdbk | F | F/M/T |
| | Delay | T | — |
| | Rate, Depth | M | — |
| | →Rev (Send Level to Reverb) | A | A/C |
| Reverb: Room 1/2, Hall 1/2, Plate | PreDly, Time, Densty, Diffus, Spread | T | — |
| | LFDamp, HFDamp | F | F/T |
| | Tone | F | — |
| Reverb: GM2 Rev | Char (Character 0–5) | T | T/C |
| | Time | T | — |
| Comp+EQ unit (×6) | Comp Switch, EQ Switch | C | — |
| | Threshold, Ratio, Output Gain | A | — |
| | Attack, Release | A | A/T |
| | Low/Mid/High Freq, Low/Mid/High Gain, Mid Q | F | — |

The parameters around these blocks are outside the counts but follow the same rules:

- **C:** Chorus/Reverb Type, Switch, Output Select and Output Assign; Comp+EQ Output Assign;
  MFX Type and MFX Switch.
- **A:** Chorus/Reverb Level, and the MFX Chorus/Reverb Send Levels. A send feeds a space effect,
  so T is a weaker alternative.
- **M:** MFX Control 1–4 Source/Sens. These are modulation routing; C is the alternative.

## Ambiguous

Twenty-one clusters, roughly by size. The number in brackets is how many parameter instances the
cluster covers across all scopes. Each cluster lists the competing families, the evidence, and a
recommendation. [#159](https://github.com/sigma/integral/issues/159) decided each one as recommended, except cluster 8.

1. **Balance: dry/wet (41).** A vs C. P1: "Volume balance between the direct sound (D) and the
   effect sound (W)" (e.g. p. 78, p. 85, pp. 95–96). It sets the output mix, so it is a level
   (A). It is also the effect's "amount" master, the same kind of control as on/off, which
   argues C. **Decided: A.** It is a volume control, and A keeps it next to `Level`, which every type
   ends with.
2. **Step 01–16 and step Attack (34).** M vs F (Step Filter: "Cutoff frequency at each step",
   p. 74) vs A (Slicer: "Level at each step", p. 78). The steps *are* the modulation source, a
   16-step sequencer that `Rate` clocks. **Decided: M** for both. Their `Attack` follows the steps.
3. **Voicing selectors (29):** Amp Type / Pre Amp Type (9), Speaker Type (11), EP-amp Type (5),
   Mic Setting (2), Drive Type (2). F vs C. They pick an amp/cabinet/mic model, so each value
   changes the timbre (F). In UI terms they are enum selectors, like a type switch (C).
   **Decided: F**, because what they change is the tone. Keep C for selectors that change *structure*
   (phaser stage count, Gate/Duck).
4. **Drive cluster (28):** Drive and OD Drive (17), OD Gain (6), Amp Gain (5). F vs A vs P.
   P1 descriptions:
   - Drive: "Degree of distortion" (p. 75). Humanizer's Drive "Also changes the volume".
   - OD Gain: "Overdrive input level. Higher values will increase the distortion" (p. 79).
   - Amp Gain: "Amount of pre-amp distortion" (pp. 83, 90–91).
   A level knob that produces harmonics could fit A (it is a gain), F (it changes timbre) or
   P (it acts as a harmonic *source*). **Decided: F.** Its audible effect is a change of tone, and
   amp-sim tone controls (Bass/Middle/Treble/Presence) are already F.
5. **Pan (22):** every `Pan`, `X Pan`, Pan Deviation. T vs A. Stereo placement is "space" (T).
   The SN-S tone editor, however, puts pan in the amp section: `AMP Pan`, between the AMP
   envelope and the LFO (P2, 08 § Partial). **Decided: T**, everywhere: the tone-editor Pan takes T too, inside its AMP section.
6. **Manual / Resonance (16):** phaser and wah centre and feedback. F vs M. P1:
   - Phaser Manual "Adjusts the basic frequency from which the sound will be modulated"
     (pp. 76–77).
   - Wah Manual "Center frequency at which the wah effect is applied" (p. 89).
   - Phaser Resonance "Amount of feedback" (pp. 76–77, 91, 94).
   They set the filter that the LFO sweeps. **Decided: F.** Rate/Depth/Polarity already carry M,
   and Step Filter's `Filter Resonance` is F by the same reasoning.
7. **HF/LF Damp (18).** F vs T. P1: "Frequency at which the high-frequency portion of the delay
   sound will be cut (BYPASS: no cut)" (pp. 85–86, 92). For Reverb, damping shapes the *decay*.
   **Decided: F.** It is a low-pass in the feedback loop, so a cutoff.
8. **Non-delay Feedback (16).** Flanger/Step Flanger and `Flanger Feedback` in combinations
   (9), Chorus `Fdbk` (2, Chorus and GM2 Chorus: "amount of the chorus sound that is fed back",
   p. 98), Pitch Shifter feedback (3), phaser Cross Feedback (2). Delay feedback is firm T. With
   a ~1–10 ms line, a flanger's feedback acts as comb *resonance*, not echoes.
   **Decided:**
   - Flanger, Chorus and Cross Feedback (P1: "proportion of the phaser sound that is fed
     back", p. 76): F. Feedback adds comb resonance, the same mechanism as phaser
     `Resonance` (cluster 6); it does not generate movement. The research had recommended M.
   - Pitch Shifter: T. It is real delay-line repetition, and P is the weaker alternative.
   "Every Feedback is T" was rejected: consistent but sonically misleading for flangers.
9. **Mix (10):** Phaser Mix (6: "Level/Volume of the phase-shifted sound", pp. 77, 91),
   Enhancer Mix (4: "Level of the overtones generated by the enhancer", pp. 75, 95). A vs C
   (as for Balance), and F for the Enhancer. **Decided: A.**
10. **Sens (7):** Enhancer Sens (4: "Sensitivity of the enhancer", the amount of enhancement)
    is F vs A, **decided F**. Auto Wah and Touch Wah Sens ("sensitivity with which the filter is
    controlled", pp. 75, 89) are M vs F, **decided M**, because the envelope follower is a
    modulation source. Ring Mod Sens ("amount of frequency modulation applied", p. 78) is M vs
    A, **decided M**.
11. **Mode / Color selectors (7).**
    - Phaser 1 / Step / Multi Stage `Mode` (number of stages): C vs F, **decided C**.
    - Infinite Phaser `Mode` ("Higher values will produce a deeper phaser effect", p. 77): M vs
      C, **decided M**.
    - Slicer `Mode` (LEGATO/SLASH, the shape of each slice): M vs C, **decided M**.
    - Gate `Mode` (GATE/DUCK): C vs A, **decided C**.
    - Phaser 2 `Color` ("Modulation character", p. 76): M vs F, **decided M**.
12. **Dynamics times (7):** Compressor Attack, Gate Attack/Hold/Release, Limiter Release, Comp+EQ
    Attack/Release. A vs T. They are times, but they are the envelope of a *level* processor,
    just as a tone's amp envelope is Amp. **Decided: A.**
13. **Polarity (5).** M vs C. Wah/Phaser/Ring Mod polarity sets the sweep direction (UP/DOWN,
    p. 78). Phaser 1 polarity (INVERSE/SYNCHRO) is the L/R LFO phase (p. 76). **Decided: M.**
14. **Rotary Speed / Brake (5).** M vs C. Speed is the SLOW/FAST switch ("Rotational speed",
    p. 79). Brake stops or resumes the rotation. They are performance switches, but what they
    switch is the modulation rate. **Decided: M.**
15. **Input Sync Sw / Threshold (4)** (Humanizer, Slicer). M vs C. They restart the LFO or step
    sequence on note input (p. 78). **Decided: M.**
16. **Lo-fi (3):** Bit Crasher Sample Rate / Bit Down, LOFI Compress LoFi Type. P vs F
    (vs C for the type). They degrade the signal itself rather than filter it. **Decided: P**, which
    gives Pitch / Source a few effect members. F is the safe alternative.
17. **Phase Left / Right (2)** (Delay, "NORMAL, INVERSE: Phase of the delay sound", p. 85). T vs
    C. **Decided: T.** It is stereo image. Note that `Phase` in degrees, the LFO L/R offset, is firm
    M.
18. **Separation (2):** Rotary 1 Separation (no P1 explanation), Tremolo Chorus Tremolo
    Separation ("Spread of the tremolo effect", p. 82). T vs M. **Decided: T**, the same as
    `Spread`.
19. **Delay-trigger oddities (3).** Reverse Delay `Threshold` (the level that triggers reverse
    playback): T vs A, **decided T**. Time Ctrl Delay `Acceleration` ("Speed at which the current
    delay time changes", p. 94) and EP AmpSim -> Delay `Delay Acceleration`: T vs M, **decided T**.
20. **Ring Mod Frequency (1).** P vs M. It is the carrier oscillator's frequency, a source, but
    it modulates the input. **Decided: P.**
21. **Chorus/Reverb leftovers (2).** GM2 Reverb `Char` ("Type of reverb", p. 98): T vs C,
    **decided T**, because it selects the room. GM2 Chorus `→Rev` send: A vs C, **decided A**.

Count check: 41 + 34 + 29 + 28 + 22 + 16 + 18 + 16 + 10 + 7 + 7 + 7 + 5 + 5 + 4 + 3 + 2 + 2 +
3 + 1 + 2 = 262.

### Switch / Note / msec slots

These do not appear as separate parameters anywhere the UI reads today:

- Chorus/Reverb use `FxParamDef::synced`, so one control resolves the switch/value/note
  triple.
- The MFX tables (R1) list one entry per Parameter Guide row, so `Rate` is a single 0–127
  entry. Whether the wire uses three slots is
  [#30](https://github.com/sigma/integral/issues/30).

If these slots become visible, the choices are:

- C, as a mode switch.
- The family of the value the slot selects: Hz/Note → M (Rate), msec/Note → T (Delay Time).

**Decided: inherit.** The switch and the note value are views of one control, and colouring them
differently would split one knob into two families.

*Stage* on/off switches in combination types (`Amp Sw`, `OD Switch`, `Chorus Switch`, … 34
instances) are counted as firm C above, and [#159](https://github.com/sigma/integral/issues/159) kept them C: they have no sound dimension.

### Not ambiguous, noted for completeness

- **Bias** does not occur on the INTEGRA-7. There is no such parameter in R1/R2 or in P1
  pp. 73–99.
- **Dry/Wet** is the INTEGRA-7's `Balance` (item 1). `Direct Level` (Speaker Sim, Guitar Amp
  Sim) is a plain level: firm A.
- **Threshold** in the dynamics types (Compressor, Limiter, Gate) is firm A. Only Reverse
  Delay's is ambiguous (item 19).

## Appendix: per-type result (MFX)

| # | Type | Parameters (family; `?` = ambiguous, decision shown) |
|--:|---|---|
| 0 | Thru | — |
| 1 | Equalizer | Low Freq F, Low Gain F, Mid1 Freq F, Mid1 Gain F, Mid1 Q F, Mid2 Freq F, Mid2 Gain F, Mid2 Q F, High Freq F, High Gain F, Level A |
| 2 | Spectrum | Band1 (250Hz) F, Band2 (500Hz) F, Band3 (1000Hz) F, Band4 (1250Hz) F, Band5 (2000Hz) F, Band6 (3150Hz) F, Band7 (4000Hz) F, Band8 (8000Hz) F, Q F, Level A |
| 3 | Low Boost | Boost Frequency F, Boost Gain F, Boost Width F, Low Gain F, High Gain F, Level A |
| 4 | Step Filter | Step 01 M?, Step 02 M?, Step 03 M?, Step 04 M?, Step 05 M?, Step 06 M?, Step 07 M?, Step 08 M?, Step 09 M?, Step 10 M?, Step 11 M?, Step 12 M?, Step 13 M?, Step 14 M?, Step 15 M?, Step 16 M?, Rate M, Attack M?, Filter Type F, Filter Slope F, Filter Resonance F?, Filter Gain F, Level A |
| 5 | Enhancer | Sens F?, Mix A?, Low Gain F, High Gain F, Level A |
| 6 | Auto Wah | Filter Type F, Manual F?, Peak F, Sens M?, Polarity M?, Rate M, Depth M, Phase M, Low Gain F, High Gain F, Level A |
| 7 | Humanizer | Drive Sw C, Drive F?, Vowel1 F, Vowel2 F, Rate M, Depth M, Input Sync Sw M?, Input Sync Threshold M?, Manual F?, Low Gain F, High Gain F, Pan T?, Level A |
| 8 | Speaker Simulator | Speaker Type F?, Mic Setting F?, Mic Level A, Direct Level A, Level A |
| 9 | Phaser 1 | Mode C?, Manual F?, Rate M, Depth M, Polarity M?, Resonance F?, Cross Feedback F?, Mix A?, Low Gain F, High Gain F, Level A |
| 10 | Phaser 2 | Rate M, Color M?, Low Gain F, High Gain F, Level A |
| 11 | Phaser 3 | Speed M, Low Gain F, High Gain F, Level A |
| 12 | Step Phaser | Mode C?, Manual F?, Rate M, Depth M, Polarity M?, Resonance F?, Cross Feedback F?, Step Rate M, Mix A?, Low Gain F, High Gain F, Level A |
| 13 | Multi Stage Phaser | Mode C?, Manual F?, Rate M, Depth M, Resonance F?, Mix A?, Pan T?, Low Gain F, High Gain F, Level A |
| 14 | Infinite Phaser | Mode M?, Speed M, Resonance F?, Mix A?, Pan T?, Low Gain F, High Gain F, Level A |
| 15 | Ring Modulator | Frequency P?, Sens M?, Polarity M?, Low Gain F, High Gain F, Balance A?, Level A |
| 16 | Tremolo | Mod Wave M, Rate M, Depth M, Low Gain F, High Gain F, Level A |
| 17 | Auto Pan | Mod Wave M, Rate M, Depth M, Low Gain F, High Gain F, Level A |
| 18 | Slicer | Step 01 M?, Step 02 M?, Step 03 M?, Step 04 M?, Step 05 M?, Step 06 M?, Step 07 M?, Step 08 M?, Step 09 M?, Step 10 M?, Step 11 M?, Step 12 M?, Step 13 M?, Step 14 M?, Step 15 M?, Step 16 M?, Rate M, Attack M?, Input Sync Sw M?, Input Sync Threshold M?, Mode M?, Shuffle M, Level A |
| 19 | Rotary 1 | Speed M?, Woofer Slow Speed M, Woofer Fast Speed M, Woofer Acceleration M, Woofer Level A, Tweeter Slow Speed M, Tweeter Fast Speed M, Tweeter Acceleration M, Tweeter Level A, Separation T?, Level A |
| 20 | Rotary 2 | Speed M?, Brake M?, Woofer Slow Speed M, Woofer Fast Speed M, Woofer Trans Up M, Woofer Trans Down M, Woofer Level A, Tweeter Slow Speed M, Tweeter Fast Speed M, Tweeter Trans Up M, Tweeter Trans Down M, Tweeter Level A, Spread T, Low Gain F, High Gain F, Level A |
| 21 | Rotary 3 | Speed M?, Brake M?, Woofer Slow Speed M, Woofer Fast Speed M, Woofer Trans Up M, Woofer Trans Down M, Woofer Level A, Tweeter Slow Speed M, Tweeter Fast Speed M, Tweeter Trans Up M, Tweeter Trans Down M, Tweeter Level A, Spread T, Low Gain F, High Gain F, Level A, OD Switch C, OD Gain F?, OD Drive F?, OD Level A |
| 22 | Chorus | Filter Type F, Cutoff Freq F, Pre Delay T, Rate M, Depth M, Phase M, Low Gain F, High Gain F, Balance A?, Level A |
| 23 | Flanger | Filter Type F, Cutoff Freq F, Pre Delay T, Rate M, Depth M, Phase M, Feedback F?, Low Gain F, High Gain F, Balance A?, Level A |
| 24 | Step Flanger | Filter Type F, Cutoff Freq F, Pre Delay T, Rate M, Depth M, Phase M, Feedback F?, Step Rate M, Low Gain F, High Gain F, Balance A?, Level A |
| 25 | Hexa-Chorus | Pre Delay T, Rate M, Depth M, Pre Delay Deviation T, Depth Deviation M, Pan Deviation T?, Balance A?, Level A |
| 26 | Tremolo Chorus | Pre Delay T, Chorus Rate M, Chorus Depth M, Tremolo Rate M, Tremolo Separation T?, Tremolo Phase M, Balance A?, Level A |
| 27 | Space-D | Pre Delay T, Rate M, Depth M, Phase M, Low Gain F, High Gain F, Balance A?, Level A |
| 28 | Overdrive | Drive F?, Tone F, Amp Sw C, Amp Type F?, Low Gain F, High Gain F, Pan T?, Level A |
| 29 | Distortion | Drive F?, Tone F, Amp Sw C, Amp Type F?, Low Gain F, High Gain F, Pan T?, Level A |
| 30 | Guitar Amp Simulator | Pre Amp Sw C, Pre Amp Type F?, Pre Amp Volume A, Pre Amp Master A, Pre Amp Gain F?, Pre Amp Bass F, Pre Amp Middle F, Pre Amp Treble F, Pre Amp Presence F, Pre Amp Bright F, Speaker Sw C, Speaker Type F?, Mic Setting F?, Mic Level A, Direct Level A, Pan T?, Level A |
| 31 | Compressor | Attack A?, Threshold A, Post Gain A, Low Gain F, High Gain F, Level A |
| 32 | Limiter | Release A?, Threshold A, Ratio A, Post Gain A, Low Gain F, High Gain F, Level A |
| 33 | Gate | Threshold A, Mode C?, Attack A?, Hold A?, Release A?, Balance A?, Level A |
| 34 | Delay | Delay Left T, Delay Right T, Phase Left T?, Phase Right T?, Feedback Mode T, Feedback T, HF Damp F?, Low Gain F, High Gain F, Balance A?, Level A |
| 35 | Modulation Delay | Delay Left T, Delay Right T, Feedback Mode T, Feedback T, HF Damp F?, Rate M, Depth M, Phase M, Low Gain F, High Gain F, Balance A?, Level A |
| 36 | 3Tap Pan Delay | Delay Left T, Delay Right T, Delay Center T, Center Feedback T, HF Damp F?, Left Level A, Right Level A, Center Level A, Low Gain F, High Gain F, Balance A?, Level A |
| 37 | 4Tap Pan Delay | Delay 1 Time T, Delay 2 Time T, Delay 3 Time T, Delay 4 Time T, Delay 1 Feedback T, HF Damp F?, Delay 1 Level A, Delay 2 Level A, Delay 3 Level A, Delay 4 Level A, Low Gain F, High Gain F, Balance A?, Level A |
| 38 | Multi Tap Delay | Delay 1 Time T, Delay 2 Time T, Delay 3 Time T, Delay 4 Time T, Delay 1 Feedback T, HF Damp F?, Delay 1 Pan T?, Delay 2 Pan T?, Delay 3 Pan T?, Delay 4 Pan T?, Delay 1 Level A, Delay 2 Level A, Delay 3 Level A, Delay 4 Level A, Low Gain F, High Gain F, Balance A?, Level A |
| 39 | Reverse Delay | Threshold T?, Rev Delay Time T, Rev Delay Feedback T, Rev Delay HF Damp F?, Rev Delay Pan T?, Rev Delay Level A, Delay 1 Time T, Delay 2 Time T, Delay 3 Time T, Delay 3 Feedback T, Delay HF Damp F?, Delay 1 Pan T?, Delay 2 Pan T?, Delay 1 Level A, Delay 2 Level A, Low Gain F, High Gain F, Balance A?, Level A |
| 40 | Time Ctrl Delay | Delay Time T, Acceleration T?, Feedback T, HF Damp F?, Low Gain F, High Gain F, Balance A?, Level A |
| 41 | LOFI Compress | Pre Filter Type F, LoFi Type P?, Post Filter Type F, Post Filter Cutoff F, Low Gain F, High Gain F, Balance A?, Level A |
| 42 | Bit Crasher | Sample Rate P?, Bit Down P?, Filter F, Low Gain F, High Gain F, Level A |
| 43 | Pitch Shifter | Coarse P, Fine P, Delay Time T, Feedback T?, Low Gain F, High Gain F, Balance A?, Level A |
| 44 | 2Voice Pitch Shifter | Pitch1 Coarse P, Pitch1 Fine P, Pitch1 Delay T, Pitch1 Feedback T?, Pitch1 Pan T?, Pitch1 Level A, Pitch2 Coarse P, Pitch2 Fine P, Pitch2 Delay T, Pitch2 Feedback T?, Pitch2 Pan T?, Pitch2 Level A, Low Gain F, High Gain F, Balance A?, Level A |
| 45 | Overdrive -> Chorus | Overdrive Drive F?, Overdrive Pan T?, Chorus Pre Delay T, Chorus Rate M, Chorus Depth M, Chorus Balance A?, Level A |
| 46 | Overdrive -> Flanger | Overdrive Drive F?, Overdrive Pan T?, Flanger Pre Delay T, Flanger Rate M, Flanger Depth M, Flanger Feedback F?, Flanger Balance A?, Level A |
| 47 | Overdrive -> Delay | Overdrive Drive F?, Overdrive Pan T?, Delay Time T, Delay Feedback T, Delay HF Damp F?, Delay Balance A?, Level A |
| 48 | Distortion -> Chorus | Distortion Drive F?, Distortion Pan T?, Chorus Pre Delay T, Chorus Rate M, Chorus Depth M, Chorus Balance A?, Level A |
| 49 | Distortion -> Flanger | Distortion Drive F?, Distortion Pan T?, Flanger Pre Delay T, Flanger Rate M, Flanger Depth M, Flanger Feedback F?, Flanger Balance A?, Level A |
| 50 | Distortion -> Delay | Distortion Drive F?, Distortion Pan T?, Delay Time T, Delay Feedback T, Delay HF Damp F?, Delay Balance A?, Level A |
| 51 | OD/DS -> TouchWah | Drive Switch C, Drive Type F?, Drive F?, Tone F, Amp Switch C, Amp Type F?, TWah Switch C, TWah Mode F, TWah Polarity M?, TWah Sens M?, TWah Manual F?, TWah Peak F, TWah Balance A?, Low Gain F, High Gain F, Level A |
| 52 | OD/DS -> AutoWah | Drive Switch C, Drive Type F?, Drive F?, Tone F, Amp Switch C, Amp Type F?, AutoWah Switch C, AutoWah Mode F, AutoWah Manual F?, AutoWah Peak F, AutoWah Rate M, AutoWah Depth M, AutoWah Balance A?, Low Gain F, High Gain F, Level A |
| 53 | GuitarAmpSim -> Chorus | Pre Amp Sw C, Pre Amp Type F?, Pre Amp Volume A, Pre Amp Master A, Pre Amp Gain F?, Pre Amp Bass F, Pre Amp Middle F, Pre Amp Treble F, Speaker Sw C, Speaker Type F?, Chorus Switch C, Chorus Pre Delay T, Chorus Rate M, Chorus Depth M, Chorus Balance A?, Level A |
| 54 | GuitarAmpSim -> Flanger | Pre Amp Sw C, Pre Amp Type F?, Pre Amp Volume A, Pre Amp Master A, Pre Amp Gain F?, Pre Amp Bass F, Pre Amp Middle F, Pre Amp Treble F, Speaker Sw C, Speaker Type F?, Flanger Switch C, Flanger Pre Delay T, Flanger Rate M, Flanger Depth M, Flanger Feedback F?, Flanger Balance A?, Level A |
| 55 | GuitarAmpSim -> Phaser | Pre Amp Sw C, Pre Amp Type F?, Pre Amp Volume A, Pre Amp Master A, Pre Amp Gain F?, Pre Amp Bass F, Pre Amp Middle F, Pre Amp Treble F, Speaker Sw C, Speaker Type F?, Phaser Switch C, Phaser Rate M, Phaser Manual F?, Phaser Depth M, Phaser Resonance F?, Phaser Mix A?, Level A |
| 56 | GuitarAmpSim -> Delay | Pre Amp Sw C, Pre Amp Type F?, Pre Amp Volume A, Pre Amp Master A, Pre Amp Gain F?, Pre Amp Bass F, Pre Amp Middle F, Pre Amp Treble F, Speaker Sw C, Speaker Type F?, Delay Switch C, Delay Time T, Delay Feedback T, Delay HF Damp F?, Delay Balance A?, Level A |
| 57 | EP AmpSim -> Tremolo | Type F?, Bass F, Treble F, Tremolo Switch C, Tremolo Speed M, Tremolo Depth M, Tremolo Duty M, Speaker Type F?, OD Switch C, OD Gain F?, OD Drive F?, Level A |
| 58 | EP AmpSim -> Chorus | Type F?, Bass F, Treble F, Chorus Switch C, Chorus Pre Delay T, Chorus Rate M, Chorus Depth M, Chorus Balance A?, Speaker Type F?, OD Switch C, OD Gain F?, OD Drive F?, Level A |
| 59 | EP AmpSim -> Flanger | Type F?, Bass F, Treble F, Flanger Switch C, Flanger Pre Delay T, Flanger Rate M, Flanger Depth M, Flanger Feedback F?, Flanger Balance A?, Speaker Type F?, OD Switch C, OD Gain F?, OD Drive F?, Level A |
| 60 | EP AmpSim -> Phaser | Type F?, Bass F, Treble F, Phaser Switch C, Phaser Rate M, Phaser Manual F?, Phaser Depth M, Phaser Resonance F?, Phaser Mix A?, Speaker Type F?, OD Switch C, OD Gain F?, OD Drive F?, Level A |
| 61 | EP AmpSim -> Delay | Type F?, Bass F, Treble F, Delay Switch C, Delay Time T, Delay Acceleration T?, Delay Feedback T, Delay HF Damp F?, Delay Balance A?, Speaker Type F?, OD Switch C, OD Gain F?, OD Drive F?, Level A |
| 62 | Enhancer -> Chorus | Enhancer Sens F?, Enhancer Mix A?, Chorus Pre Delay T, Chorus Rate M, Chorus Depth M, Chorus Balance A?, Level A |
| 63 | Enhancer -> Flanger | Enhancer Sens F?, Enhancer Mix A?, Flanger Pre Delay T, Flanger Rate M, Flanger Depth M, Flanger Feedback F?, Flanger Balance A?, Level A |
| 64 | Enhancer -> Delay | Enhancer Sens F?, Enhancer Mix A?, Delay Time T, Delay Feedback T, Delay HF Damp F?, Delay Balance A?, Level A |
| 65 | Chorus -> Delay | Chorus Pre Delay T, Chorus Rate M, Chorus Depth M, Chorus Balance A?, Delay Time T, Delay Feedback T, Delay HF Damp F?, Delay Balance A?, Level A |
| 66 | Flanger -> Delay | Flanger Pre Delay T, Flanger Rate M, Flanger Depth M, Flanger Feedback F?, Flanger Balance A?, Delay Time T, Delay Feedback T, Delay HF Damp F?, Delay Balance A?, Level A |
| 67 | Chorus -> Flanger | Chorus Pre Delay T, Chorus Rate M, Chorus Depth M, Chorus Balance A?, Flanger Pre Delay T, Flanger Rate M, Flanger Depth M, Flanger Feedback F?, Flanger Balance A?, Level A |
