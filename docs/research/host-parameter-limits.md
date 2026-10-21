# Host Parameter Limits (research for #338)

Answers [#338](https://github.com/sigma/integral/issues/338) for the map
[#336](https://github.com/sigma/integral/issues/336): can a plugin expose about 1,000 host
parameters per instance (12,700+ for a PCM Drum Kit), can the list change at runtime, and
what do hosts do with idle plugins and sandboxing.

## TL;DR

- **Specs allow it.** VST3 parameter IDs span `[0, 0x7FFFFFFF]` with no count limit; CLAP
  states no limit. Neither spec lets the *set* of parameters change while the plugin runs.
- **nih-plug (pinned rev) fixes the list at construction.** It reads `param_map()` once, when
  the wrapper is built. It never sends VST3 `kParamTitlesChanged` or CLAP
  `CLAP_PARAM_RESCAN_INFO`/`ALL`, only "values changed". Parameter names are set at
  construction and have no setter. Renaming a parameter at runtime needs a nih-plug patch.
- **Built from data: yes.** `param_map()` returns owned `String` IDs and groups, and
  `#[nested(array)]` iterates with `.iter().enumerate()`, so a `Vec` sized at construction works,
  and arrays nest (`Key n/Partial m`). What a parameter *shows* can change at runtime:
  `with_value_to_string` takes an `Arc<dyn Fn>` that may read live state.
- **Hash collisions are a real risk at drum-kit scale.** nih-plug hashes each string ID to
  31 bits; it does not check hash collisions, only duplicate strings. About 0.03 % chance of a
  collision for 1,100 parameters, about **3.8 %** for 12,900. The IDs are deterministic, so a
  unit test over the generated ID set settles it.
- **Inert parameters:** neither format can hide them at runtime through nih-plug. VST3
  `kIsHidden` (static) also forces read-only and not automatable. CLAP `CLAP_PARAM_IS_HIDDEN`
  is the right runtime tool ("not used currently") but needs `RESCAN_INFO`, which nih-plug
  does not send.
- **Idle `process()`:** our plugin returns `ProcessStatus::Normal`, which nih-plug maps to CLAP
  `CLAP_PROCESS_CONTINUE_IF_NOT_QUIET` and VST3 tail `kNoTail`. That *invites* hosts to sleep
  the plugin on silent input, and our input is always silent (the sound comes from hardware).
  Return `KeepAlive` if the plugin must keep running. See the host table for who actually
  suspends.
- **Hosts:**
  - Bitwig runs plugins out of process by default (all in one sandbox, "Together") and can
    split them per plugin.
  - Reaper is in-process by default but has separate and dedicated process options.
  - Live and Cubase are in-process.
  - Live has no CLAP, and Live's plugin panel lists parameters only up to 64 before you have
    to pick them in Configure.
  - No host documents a hard parameter limit. Big instruments cap at 512 generic slots
    (Omnisphere, Kontakt, HALion).

## Sources (highest trust first)

| # | Source | What it gives |
|---|--------|---------------|
| N | nih-plug at the pinned rev [`28b149e`](https://github.com/robbert-vdh/nih-plug/tree/28b149ec4d62757d0b448809148a0c3ca6e09a95) (from `Cargo.lock`) | What the wrapper can and cannot do. File/line links below. |
| V1 | VST3 Developer Portal, [Parameters and Automation](https://steinbergmedia.github.io/vst3_dev_portal/pages/Technical+Documentation/Parameters+Automation/Index.html) | ID range, "no reconfiguration", `kIsHidden`, `kParamTitlesChanged`. |
| V2 | VST3 API reference, [`Steinberg::Vst` namespace](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/namespaceSteinberg_1_1Vst.html) | `RestartFlags` semantics. |
| C | CLAP [`include/clap/ext/params.h`](https://github.com/free-audio/clap/blob/main/include/clap/ext/params.h) | Rescan flags, `CLAP_PARAM_IS_HIDDEN`, ID stability, state ownership. |
| H | Host vendor manuals and staff forum posts | See [Hosts](#hosts); each row is tagged. |

## Specs

### VST3 (V1, V2)

- "Up to 2^31 parameters can be exported with ID range [0, 2.147.483.647]". The upper half,
  `[0x80000000, 0xFFFFFFFF]`, is reserved for the host.
- The plug-in "must not perform any reconfigurations that lead to a different set of
  automatable parameters". Adding or removing parameters is allowed only "in a future plug-in
  version", and "automation data can get lost when parameters are removed".
- `kParamTitlesChanged` (`1 << 4`): "Parameter titles (title, shortTitle and units), default
  values, stepCount or flags (ParameterFlags) have changed". The plug-in calls
  `restartComponent(kParamTitlesChanged)` on the UI thread and the host re-reads
  `getParameterInfo`. So VST3 *can* rename a fixed set of parameters at runtime.
- `kParamIDMappingChanged` (`1 << 11`) exists for remapping IDs; `kReloadComponent` makes the
  host unload and reload the plug-in.
- `kIsHidden`: "will NOT be displayed and cannot be changed from outside the plug-in". It
  requires `kIsReadOnly` and no `kCanAutomate`. It hides a parameter from automation entirely,
  not "currently unused".

### CLAP (C)

- No parameter count limit in the header.
- `clap_param_info.id`: "Stable parameter identifier, it must never change."
- `CLAP_PARAM_RESCAN_INFO`: name, module, `is_periodic` and `is_hidden` changed; "New info
  takes effect immediately". This is the runtime rename.
- `CLAP_PARAM_RESCAN_ALL`: "Invalidates everything the host knows about parameters. It can
  only be used while the plugin is deactivated." Otherwise call `clap_host->restart()` first.
- `CLAP_PARAM_IS_HIDDEN`: "should not be shown to the user, because it is currently not used".
  This is exactly the inert-parameter case.
- The plugin owns its state: hosts "should *not* try to save and restore parameter values for
  plugins that don't implement the state extension".

## nih-plug at `28b149e` (N)

### The list is fixed when the wrapper is built

- The VST3 wrapper calls `params.param_map()` once in its constructor and builds hash tables
  from it
  ([`vst3/inner.rs#L203-L276`](https://github.com/robbert-vdh/nih-plug/blob/28b149ec4d62757d0b448809148a0c3ca6e09a95/src/wrapper/vst3/inner.rs#L203-L276)).
  The CLAP wrapper does the same
  ([`clap/wrapper.rs#L450-L495`](https://github.com/robbert-vdh/nih-plug/blob/28b149ec4d62757d0b448809148a0c3ca6e09a95/src/wrapper/clap/wrapper.rs#L450-L495)).
- The only VST3 restart flags it sends are `kParamValuesChanged` (after a state load) and
  `kLatencyChanged`
  ([`vst3/inner.rs#L518-L536`](https://github.com/robbert-vdh/nih-plug/blob/28b149ec4d62757d0b448809148a0c3ca6e09a95/src/wrapper/vst3/inner.rs#L518-L536)).
  The only CLAP rescan is `CLAP_PARAM_RESCAN_VALUES`
  ([`clap/wrapper.rs#L415-L419`](https://github.com/robbert-vdh/nih-plug/blob/28b149ec4d62757d0b448809148a0c3ca6e09a95/src/wrapper/clap/wrapper.rs#L415-L419)).
  No public API reaches `kParamTitlesChanged`, `RESCAN_INFO`, `RESCAN_TEXT` or `RESCAN_ALL`.
- `FloatParam::new(name: impl Into<String>, ..)` stores the name; the builder has no
  `with_name` or setter
  ([`params/float.rs#L271`](https://github.com/robbert-vdh/nih-plug/blob/28b149ec4d62757d0b448809148a0c3ca6e09a95/src/params/float.rs#L271)).
  `getParameterInfo` copies `param_ptr.name()` into `title` and `short_title` on each call
  ([`vst3/wrapper.rs#L574-L575`](https://github.com/robbert-vdh/nih-plug/blob/28b149ec4d62757d0b448809148a0c3ca6e09a95/src/wrapper/vst3/wrapper.rs#L574-L575)),
  so a patched nih-plug that let the name change and sent the restart would be small.

**Consequence:** runtime renaming (one generic slot called "MFX 3: Rate" for one type and
"MFX 3: Feedback" for another) needs a nih-plug fork or upstream patch. Display *text* can
change without a patch, because `with_value_to_string(Arc<dyn Fn(f32) -> String>)`
([`params/float.rs#L377-L383`](https://github.com/robbert-vdh/nih-plug/blob/28b149ec4d62757d0b448809148a0c3ca6e09a95/src/params/float.rs#L377-L383))
may read live state. Hosts that cache value text would still not refresh it, since nih-plug
never sends `RESCAN_TEXT`.

### A parameter set built from data

- `Params::param_map(&self) -> Vec<(String, ParamPtr, String)>` uses owned strings "to be able
  to support custom reusable Params implementations"
  ([`params.rs#L282-L298`](https://github.com/robbert-vdh/nih-plug/blob/28b149ec4d62757d0b448809148a0c3ca6e09a95/src/params.rs#L282-L298)).
  IDs and group paths can therefore be computed from core field ids.
- `#[nested(array, group = "Foo")]` iterates `self.field.iter().enumerate()`
  ([`nih_plug_derive/src/params.rs#L358-L373`](https://github.com/robbert-vdh/nih-plug/blob/28b149ec4d62757d0b448809148a0c3ca6e09a95/nih_plug_derive/src/params.rs#L358-L373)),
  so a `Vec` sized at construction works as well as an array. It suffixes IDs `_{i+1}` and
  groups `Foo {i+1}`. A nested struct can itself hold a `#[nested(array)]`, giving
  `Key 12/Partial 3`. The gain example shows the pattern
  ([`plugins/examples/gain/src/lib.rs#L35-L36`](https://github.com/robbert-vdh/nih-plug/blob/28b149ec4d62757d0b448809148a0c3ca6e09a95/plugins/examples/gain/src/lib.rs#L35-L36)).
  Note: the attribute key is `group`; the trait doc says `group_name`, which the derive
  rejects.
- A hand-written `unsafe impl Params` is possible but `Param::as_ptr` is documented as an
  internal detail
  ([`params.rs#L189-L191`](https://github.com/robbert-vdh/nih-plug/blob/28b149ec4d62757d0b448809148a0c3ca6e09a95/src/params.rs#L189-L191)).
  Deriving over `Vec`s of typed parameter structs avoids that.
- The parameter *types* are still Rust types (`FloatParam`, `IntParam`, `EnumParam<E>`,
  `BoolParam`). `EnumParam<E>` needs a compile-time `Enum`; a data-driven list of choices
  would use `IntParam` plus `with_value_to_string`.

### IDs: 31-bit hashes, collisions unchecked

- Each string ID becomes a VST3/CLAP ID through a Rabin hash masked to 31 bits
  ([`wrapper/util.rs#L39-L51`](https://github.com/robbert-vdh/nih-plug/blob/28b149ec4d62757d0b448809148a0c3ca6e09a95/src/wrapper/util.rs#L39-L51)).
- In debug builds it asserts that string IDs are unique
  ([`vst3/inner.rs#L212-L223`](https://github.com/robbert-vdh/nih-plug/blob/28b149ec4d62757d0b448809148a0c3ca6e09a95/src/wrapper/vst3/inner.rs#L212-L223)),
  but not the hashes. Two IDs hashing alike silently share a host slot.
- Birthday odds for a random 31-bit hash: 0.03 % at 1,100 parameters, 3.8 % at 12,900. The
  hash is deterministic, so one unit test over the generated ID set rules it out per release.
- With `MIDI_INPUT >= MidiCCs`, nih-plug also reserves 130 × 16 IDs at the top of the range
  for MIDI CC parameters
  ([`vst3/util.rs#L14-L20`](https://github.com/robbert-vdh/nih-plug/blob/28b149ec4d62757d0b448809148a0c3ca6e09a95/src/wrapper/vst3/util.rs#L14-L20)).
  We use `MidiConfig::Basic` today.

### State size

- nih-plug saves every parameter, unnormalised, in a `BTreeMap<String, ParamValue>` serialised
  as JSON, zstd-compressed only with the `zstd` feature
  ([`wrapper/state.rs#L31-L45`](https://github.com/robbert-vdh/nih-plug/blob/28b149ec4d62757d0b448809148a0c3ca6e09a95/src/wrapper/state.rs#L31-L45),
  [`#L132-L160`](https://github.com/robbert-vdh/nih-plug/blob/28b149ec4d62757d0b448809148a0c3ca6e09a95/src/wrapper/state.rs#L132-L160)).
  We do not enable `zstd` (`crates/integral-vst/Cargo.toml`).
- Rough estimate (not measured): about 25–30 bytes per entry, so about 30 KB per Part
  instance and about 350 KB per PCM Drum Kit instance, before zstd.

### Idle processing

- `ProcessStatus::Normal` maps to `CLAP_PROCESS_CONTINUE_IF_NOT_QUIET` and to VST3
  `getTailSamples() = kNoTail`; `KeepAlive` maps to `CLAP_PROCESS_CONTINUE` and
  `kInfiniteTail`
  ([`clap/wrapper.rs#L2259-L2264`](https://github.com/robbert-vdh/nih-plug/blob/28b149ec4d62757d0b448809148a0c3ca6e09a95/src/wrapper/clap/wrapper.rs#L2259-L2264),
  [`vst3/wrapper.rs#L1673-L1680`](https://github.com/robbert-vdh/nih-plug/blob/28b149ec4d62757d0b448809148a0c3ca6e09a95/src/wrapper/vst3/wrapper.rs#L1673-L1680)).
- `crates/integral-vst/src/lib.rs` returns `ProcessStatus::Normal` from `process()`.

## Hosts

Confidence tags: **[primary]** vendor docs or spec, **[staff]** vendor staff on a forum,
**[user]** user or third-party report, **[unconf]** not verified. Vendor docs say little about
parameter counts; most host behaviour below is user-reported. The Cockos wiki and the Ableton
help center refused automated fetches.

### Many parameters

- **Live:** plug-ins with up to 64 parameters get every one as a slider. Larger ones open
  with an empty panel, and you pick parameters in Configure mode. No maximum stated
  [primary] ([manual](https://www.ableton.com/en/live-manual/12/using-plug-ins/)).
  - The "128" cap is the `-_PluginAutoPopulateThreshold` Options.txt flag, which auto-fills
    Configure up to 128 [user] ([forum](https://forum.ableton.com/viewtopic.php?p=1837238)).
  - Live 6 and 7 automated only the first 128 parameters; this was reportedly lifted in
    Live 8 [user] ([forum](https://forum.ableton.com/viewtopic.php?p=837958)).
  - VST3 preset loads take seconds when the plug-in notifies the host per parameter. The fix
    is one `restartComponent(kParamValuesChanged)` [user, JUCE devs]
    ([forum](https://forum.juce.com/t/ableton-live-vst3-parameter-changes-slow/50014)).
    nih-plug already sends that one flag after a state load.
- **Live has no CLAP support** as of 2026 [user]
  ([forum](https://forum.ableton.com/viewtopic.php?p=1802901)), so VST3 is the lowest common
  denominator.
- **Reaper:** "REAPER can deal with the number of parameters for a plug-in changing on the
  fly" (VST2 extension) [primary] ([SDK](https://www.reaper.fm/sdk/vst/vst_ext.php)).
  - A changelog entry reads "CLAP: optimize memory use for plug-ins with many parameters", and
    Reaper groups parameters by VST3 unit or CLAP module. Version unconfirmed [unconf]
    ([mirror](https://mac.softpedia.com/progChangelog/REAPER-Changelog-36382.html)).
- **Cubase:** the automation parameter list is "often huge" and needs scrolling [user]
  ([forum](https://vi-control.net/community/threads/cubase-touch-parameter-to-add-automation-lane.143730/)).
  Steinberg's own HALion exposes a fixed 512 automation slots [primary]
  ([manual](https://steinberg.help/halion/v7/en/halion/topics/automation/automation_and_midi_controllers_c.html)).
- **Bitwig:** no documented limit found. Remote Controls come in pages of 8.
- **Vendor practice:** large instruments cap host parameters at a fixed bank of generic,
  user-assigned slots.
  - Omnisphere: 512, and Spectrasonics notes that some hosts allow only 128 and others 1024
    [primary, search snippet only]
    ([manual](https://support.spectrasonics.net/manual/Omnisphere2/concepts/automation/page01.html)).
  - Kontakt: 512 [user] ([forum](https://community.native-instruments.com/discussion/comment/6454/)).
  - **No precedent found** for a plug-in exposing over 10,000 parameters.
- **Project size:** hosts store the plug-in's opaque state chunk. Per-parameter values are
  written only for plug-ins without chunk support [user, inferred from Ardour]
  ([source](https://git.ardour.org/ardour/ardour/src/branch/master/libs/ardour/vst_plugin.cc)).
  So the nih-plug JSON state above is the cost. No host load-time measurements were found.

### Runtime renames and inert parameters

- **Reaper:** reports conflict on whether it honours VST3 `kParamTitlesChanged` [user]
  ([KVR](https://www.kvraudio.com/forum/viewtopic.php?p=8832424),
  [JUCE](https://forum.juce.com/t/vst3-parameter-name-changes-dont-work/28313)).
- **Bitwig, Live, Cubase:** no vendor statement found on honouring title or info rescans,
  or CLAP `IS_HIDDEN` [unconf]. Ardour does re-read the parameter info
  ([commit](https://git.ardour.org/ardour/ardour/commit/649ad0f0520646bff7f89f3b486c5f7b4b8fc3d9)).
- **Live:** users report that it lists a plug-in's parameters only after you touch them in
  the plug-in UI [user] ([forum](https://forum.juce.com/t/hide-parameters-from-host-by-default/65338)).
  Inert parameters are harmless there but still crowd Configure.
- **Untested:** cross-host behaviour of runtime renames. The safe design must not depend on it.

### Idle `process()`

- **Specs:**
  - VST3 `silenceFlags` are optional for hosts, and `setProcessing(false)` may stop calls
    entirely [primary]
    ([`ivstaudioprocessor.h`](https://github.com/steinbergmedia/vst3_pluginterfaces/blob/master/vst/ivstaudioprocessor.h)).
  - CLAP `CLAP_PROCESS_SLEEP` stops processing "until the next event or variation in audio
    input" [primary] ([`process.h`](https://github.com/free-audio/clap/blob/main/include/clap/process.h)).
- **Cubase:** "Suspend VST3 plug-in processing when no audio signals are received" is off by
  default, and Steinberg recommends leaving it off. It uses the tail length [primary]
  ([manual](https://archive.steinberg.help/cubase_pro/v12/en/cubase_nuendo/topics/audio_effects/audio_effects_smart_plug_in_processing_c.html)).
- **Reaper:** since 6.71, auto-bypass on silence, per project and per FX, using the reported
  tail [secondary] ([KVR news](https://www.kvraudio.com/news/56611)).
- **Bitwig:** per-plug-in Suspend: Never / When silent / Trust plug-in (the default) [user
  quoting the manual] ([KVR](https://www.kvraudio.com/forum/viewtopic.php?p=8212920)).
  A VST3 stayed silent until its window was opened unless set to Never [user]
  ([forum](https://forum.audiomodern.com/t/loopmix-no-playback-until-plugin-visible-in-bitwig/1141)).
- **Live:** nothing found [unconf].
- **Stopped transport:** no host documents stopping `process()` on a stopped transport. The
  suspend rules above key on silence, not on transport.
- **Conclusion:** report `KeepAlive` (CLAP `CONTINUE`, VST3 `kInfiniteTail`). Even so, don't
  rely on `process()` for device I/O; the device service should own timing.

### Sandboxing (in-process singletons)

- **Bitwig:** five hosting modes, Within Bitwig / Together / By manufacturer / By plug-in /
  Individually, with per-plug-in overrides [primary]
  ([user guide](https://bitwig.com/userguide/latest/vst_plug-in_handling_and_options)). The
  default, Together, runs all plug-ins in one separate process [primary]
  ([video page](https://www.bitwig.com/en/19/plugin-hosting-video.html)). A static in our
  library is shared under Together, By manufacturer and By plug-in, but not under Individually.
- **Reaper:** in-process by default. Per plug-in it can run in a separate process (shared) or
  a dedicated process (one each) [user]
  ([blog](https://reaper.blog/2012/02/run-plugin-as-dedicated-process/)).
- **Live, Cubase:** in-process; no sandbox option found [unconf].
- **Two hosts at once:** two DAWs open together (or the standalone app next to one) are two
  processes in every case.
- **Conclusion:** an in-process singleton is shared in common setups but not guaranteed. The
  device owner must be a separate process, or at least an OS-level lock or port owner.

## Implications

For [#343](https://github.com/sigma/integral/issues/343) (expose every parameter):

- **Tone types:** use a union with inert parameters. nih-plug cannot rename or hide at
  runtime, VST3 forbids changing the parameter set, and host support for title rescans is
  unproven. One plugin per tone type is the other route that holds across hosts.
- **MFX, Chorus, Reverb:** generic slots keep fixed names ("MFX Param 7"). Only the value
  text can follow the type, via `with_value_to_string`. Per-type named parameters means the
  union over all types. Pick one; runtime renaming is off the table without forking nih-plug.
- **IDs:** derive string IDs from core field ids. Add a test that fails on any 31-bit
  `hash_param_id` collision, which is real at drum-kit scale. Also check the string IDs are
  stable across releases.
- **Drum kits:** 12,718 parameters in one instance is legal in both specs, but there is no
  precedent. It brings a 3.8 % collision risk to test away, about 350 KB of uncompressed state
  per instance (enable `zstd`), and a long Live Configure list. Per-key instances (about 143
  each) fit the observed host comfort zone (512 to 1,024).
- **Value mapping:** data-driven enums need `IntParam` with `with_value_to_string`;
  `EnumParam` needs a compile-time `Enum`.

For [#342](https://github.com/sigma/integral/issues/342) (plugin family):

- **Device owner:** an in-process singleton cannot be the device owner (Bitwig Individually,
  Reaper dedicated, two hosts). This confirms the separate device service.
- **Idle processing:** each instance should return `ProcessStatus::KeepAlive` and not rely on
  `process()` cadence for device I/O. Today `crates/integral-vst/src/lib.rs` returns `Normal`.
- **Format:** VST3 is the baseline because Live has no CLAP. CLAP-only features (`IS_HIDDEN`,
  `RESCAN_INFO`) can only be extras.

**Open question:** do we patch or fork nih-plug to send `kParamTitlesChanged` and
`CLAP_PARAM_RESCAN_INFO`/`RESCAN_TEXT`? The patch looks small, since names are read on each
`getParameterInfo` call. The value is unknown until host behaviour is tested.
