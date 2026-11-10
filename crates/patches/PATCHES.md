# Patches to vendored crates

Third-party code that Integral patches lives here, apart from the project's own
crates. Each vendored crate is first committed unmodified, so that the commit
that patches it shows the whole patch. It is wired in by the root
`Cargo.toml`'s `[patch]` table; this directory is excluded from the workspace
because nih-plug brings a workspace of its own.

## nih-plug (`nih-plug/`, upstream rev 28b149e)

Vendored: the `nih_plug` crate and its `nih_plug_derive`, with the license.

**Patch: `InitContext::gui_context()`, `InitContext::async_executor()`,
`GuiContext::try_set_state()` and `AsyncExecutor::execute_gui() -> bool`**
(`src/context/{init,gui}.rs`, `src/wrapper/vst3/{context,inner}.rs`,
`src/wrapper/{clap,standalone}/wrapper.rs`, `src/event_loop{,/macos}.rs`),
and a crate-level
`allow(unexpected_cfgs, unused_imports)` in `src/lib.rs` for three upstream lint warnings.

- *Why.* A plugin can tell the host about a value it changed itself (a
  gesture, or a bulk update with one `kParamValuesChanged`) only through a
  `GuiContext`, and upstream creates one only for `Editor::spawn`, when the
  host attaches the editor view. Every parameter setter is `pub(crate)`.
  So a plugin with its editor closed cannot show a change that came from
  elsewhere (another client, the device) to the host, and the host records no
  automation for it. Integral's plugins follow the device, whether or not an
  editor is open.
- *What.* `InitContext::gui_context()` returns an `Arc<dyn GuiContext>` the plugin
  may keep, in `initialize()`. The default is `None`; only the VST3 wrapper
  returns one. The wrapper's `WrapperInner` keeps a weak reference to itself
  (`Arc::new_cyclic`) to build it from `&self`.
- *Lifetime.* A kept context must not keep the wrapper alive: the plugin lives
  inside the wrapper, so a strong reference would be a cycle. `WrapperGuiContext`
  now refers to the wrapper weakly, for the editor's context too; once the
  wrapper is gone its calls do nothing (`get_state` returns an empty state).
- *Threads.* Unchanged from upstream: the context's calls are for the host's
  main thread. `InitContext::async_executor()` gives the way there: an
  `AsyncExecutor` the plugin keeps and uses from any thread (its worker), whose
  closures refer to the wrapper weakly (the one upstream hands to
  `Plugin::editor()` holds it strongly). Without it a task could reach the main
  thread only through `ProcessContext::execute_gui`, i.e. only while the host
  calls `process()`.
- *Teardown.* The executor must not be the thread that releases the last
  reference to the wrapper (a worker doing so would run the plugin's teardown,
  which joins the worker). So `async_executor()`'s closure never upgrades the
  wrapper: it holds only the event loop's channel sender and a retained
  run-loop source (`EventLoop::off_thread_gui_poster()`), queues the task and
  wakes the main thread. After the wrapper is gone posting fails (and says so,
  below).
  macOS only; other backends return `None`, so `async_executor()` is `None`
  there. It has no background route (`execute_background` drops its tasks).
- *A full queue says so.* The event loop's queue holds 4096 tasks and drops
  the rest. Upstream's `AsyncExecutor::execute_gui` returns nothing, so its
  caller cannot tell; the long-lived executor threw away the poster's
  result. `execute_gui` now returns whether the task was queued (`false`: the
  queue is full, or the wrapper is gone), and every executor that builds one
  (VST3, CLAP, standalone) passes its wrapper's result on. Integral's worker
  posts a refused task again. Callers that ignore the result build as
  before.
- *Bounded state restore.* `GuiContext::set_state` while the plugin is
  processing retries every second until the audio thread takes the state, for
  ever if the host stopped calling `process()` without clearing the flag, or if
  the caller is the audio thread. `try_set_state(state, timeout)` gives the
  state back if it was not taken in time. The default calls `set_state`.
- *Upstream.* Not yet proposed. A candidate for an upstream PR, which would
  also need the CLAP and standalone wrappers (they return `None` here).

**Patch: `Plugin::KEEP_ALIVE`** (`src/plugin.rs`, `src/wrapper/vst3/{inner,wrapper}.rs`)

- *Why.* The VST3 wrapper tells the host a plugin keeps itself alive only
  through `getTailSamples()`, which reads the status the last `process()`
  call returned. That status starts as `Normal`, and `setProcessing()` resets
  it to `Normal`. So a host that asks for the tail after activation, before
  the first `process()` call, gets "no tail" and may suspend a plugin whose
  input is silent. Integral's plugins reach the device from `process()`, so a
  suspended plugin stops sending the host's changes
  ([#502](https://github.com/sigma/integral/issues/502)).
- *What.* A plugin sets `const KEEP_ALIVE: bool = true` (default `false`).
  The VST3 wrapper then starts and resets the status to `KeepAlive` instead of
  `Normal` (`WrapperInner::initial_process_status()`), so `getTailSamples()`
  answers `kInfiniteTail` from activation on. A `process()` result still
  replaces it, as upstream.
- *CLAP.* The CLAP wrapper has the same gap (`start_processing()` resets
  to `Normal`, `ext_tail_get()` reads it) and ignores `KEEP_ALIVE`. Integral
  ships no CLAP build (no `nih_export_clap!`), so it is left as upstream.
- *Upstream.* Not yet proposed; an upstream PR would cover CLAP too.
