# integral-device-service

## Overview

The Device service
([ADR 0006](../../docs/adr/0006-device-service-owns-the-link.md)): one
process owns the link to the INTEGRA-7 and the one core `DeviceState`, with
its 40 ms throttle, queue and cache. It generalises the Bridge of
[ADR 0003](../../docs/adr/0003-wireless-via-virtual-midi-port.md) from the
Wireless link to every link. `integral-cli bridge` launches it. Native only;
the link and the note port are traits, so a Windows port adds
implementations; the socket layer is Unix-only, with the protocol itself
independent of it.

## Interface

- `Service::new(Box<dyn DeviceLink>, Config)`: the sans-IO core. `handle(Input,
  now_ms)` applies an event; `pump(now_ms)` is the only place it sends, so the
  wire order is decided in one spot. It also drives the Expansion load and
  returns how long to sleep, from `DeviceState::next_deadline` (throttle,
  tracked-read retries, load steps).
- `spawn(service, Option<Box<dyn NotePort>>) -> Running`: starts the link and
  the note port and runs the service on a thread. `Running::stop` ends it.
  `Handle::run` runs a closure against the service on its thread; a handle
  cannot forge link events, notes or a stop.
- Links (`DeviceLink`): `WirelessLink` (TCP session, RDDP discovery, optional
  shared port `INTEGRA-7 (Wireless)`; several devices answering is an error),
  `MidirLink` (an RTP-MIDI or USB port pair chosen by `PortMatch`, up only on
  an Identity Reply; probes only after a silent interval, so any device
  message resets liveness, which means other traffic on a shared RTP-MIDI
  session can mask a power cycle; down after `ProbeTiming::max_missed`
  unanswered probes, which wait out the SysEx spacing),
  and, behind the `test-util` feature, `FakeLink`. `matching_pairs` and `port_named` find
  ports by name.
- `NotePort`: `VirtualNotePort` (midir virtual destination
  `NOTE_PORT_NAME` = "Integral Device Service", Unix only).
- `SessionId`, `SessionIdSource`: a new ID each time the link comes up. A
  second `LinkUp` with no `LinkDown` between (the device ID changed and the
  state was replaced) means "re-read everything", like a first.
- Clients (Unix only): `SocketServer::bind(path, handle)` serves a running
  service on a Unix socket. The socket's directory must be private
  (`ensure_private_dir`, mode 0700); a lock file beside the socket makes two
  services unable to take one path, and lets a new one replace the socket a
  killed one left; only this user's connections are served.
  `default_socket_path()` is the path service and plugins agree on, in a
  private `Integral` directory under the per-user temporary one (macOS:
  `confstr`, not `$TMPDIR`). `Client::connect(path)` returns a cloneable
  `Client` and an `Events`; it never blocks, sets `SO_NOSIGPIPE`, reconnects
  on its own and says `Hello` with its subscriptions again. A real-time thread
  writes through its own `Sender` (wait-free `rtrb` ring, `Copy` arguments).
  `ClientEvent`: `Connected`, `Disconnected`, `Overflowed` (the bounded queue
  dropped events: re-read), `Incompatible` (final) and each `Message`.
- The web app (`WebSocketServer::bind(port, origins, handle, log)`;
  `integral-cli bridge --ws-port N --ws-origin ORIGIN...`): a localhost
  WebSocket endpoint where a browser is a raw SysEx client, with `tungstenite`
  (blocking, thread per client like the Unix socket, so the service thread
  never waits on it). One binary frame is one whole SysEx message each way:
  the page's frames go through the paced queue as `Request::Raw`, and every
  SysEx the device sends comes back as a frame, as on a shared MIDI port. Text
  frames are ignored, pings answered, and a frame the service refuses is not
  answered. Listens on 127.0.0.1 only, default port
  `integral_core::device_service::WEBSOCKET_PORT` (4707); the web app always
  uses the default, so `--ws-port` is for tests and port clashes. Browsers
  carry no notes through it (those go to the note port: #480).
  - Link: when the service's link goes down every page is closed with code
    1013 and reason "INTEGRA-7 link down", and a page that connects while it
    is down is accepted and closed the same way at once (a browser sees a
    refused handshake only as a generic failure, but reads the close code),
    so a page's reconnect covers link state. A client the service drops
    (a full queue, say) has its socket closed.
  - Origins: the handshake needs exactly one `Origin` that, parsed as
    `scheme://host[:port]` and lowercased, is on the allow-list (also parsed
    at `bind`: a bad entry, `null` or one with a path is `InvalidInput`);
    else `403`, logged with the origin through `log`. The defaults,
    `integral_core::device_service::default_origins()`, are the `just dev-web`
    server (pinned to port 5181 by `web/vite.config.ts`, and a web test checks
    the two agree), `vite preview` (4181), each as `localhost` and `127.0.0.1`,
    and the GitHub Pages app, `https://sigma.github.io`, which as an origin
    has no path and so admits every Pages site of that user. `--ws-origin`
    given replaces them. This keeps another page in the user's browser from
    driving the device; a local program can send any header, and another
    user of the machine can connect to the loopback port (accepted).
- The protocol (`protocol.rs` has the full description): length-prefixed JSON
  frames, versioned by a `Hello`. A parameter is named by plugin and the
  host-parameter ID of core's `ParamIndex` (ADR 0008), with a `Binding` (Part,
  key) for IDs that are Binding-relative; values are the raw `i32`. `Request`:
  `Subscribe`/`Unsubscribe`, `ReceiveRaw`, `Write`/`Read`, `Raw` (one SysEx
  message), `ChangeTone`, `SwitchStudioSet`, `LoadMixer`, `Push`, `Baseline`. `Message`: `Welcome`,
  `LinkUp`, `LinkDown`, `Changed` (with the writer's `ClientId`;
  `ClientId::DEVICE` for the device, only when the value differs from the last
  known and never as a read's answer), `Bulk` (a tone or Studio Set replaced,
  or more than 16 parameters changed, or an effect slot whose type the session
  does not know, scoped to that effect: one event, not N),
  `Value` (a read's answer, to the reader only), `Pushed`, `Baseline` (the
  tone and its bytes as address runs, or no tone), `Shared` (how many other
  clients are bound to the same target; only to clients that said what they
  are bound to, when the number changes), `Sysex` and `Rejected`.
  Unknown messages decode as `Unknown` and are skipped; a frame that does not
  decode costs only itself.
- Client work enters as `Input::Connect`/`Client`/`Malformed`/`Disconnect` and
  is checked on arrival. It waits in `Service::queues` (`queues.rs`), a queue
  per client; the client served least recently goes next, one item moving into
  the device's queue whenever that is empty, so the client queues are the real
  ones, and each item is announced with its writer as it moves. A client's own
  items keep their order; between clients a write replaces the earlier one it
  covers (the device queue's own coalescing rule,
  `integral_core::device::coalesce`), so a fader dragged faster than the
  device takes it sends the latest value. A write replaces in place in its own
  client's queue, and a covered write of another client is dropped and the new
  one goes to the end of its own queue (it never moves into another client's
  queue, so it can be later than the write it replaced would have been). A
  `ChangeTone` replaces the one queued just before it for the same Part. Any
  write goes through `DeviceState::write_dt1`: no echo suppression, and a tone
  or Studio Set select is a tone or Studio Set change.
- Cache rules (`Service::known` is the one session cache):
  - A typed `Write` is dropped, unannounced, when `known` holds the bytes it
    would write. A field is unknown until the session reads it (a typed read, a
    DT1 from the device, a baseline read) or writes it, so a default the
    service never read is never compared against. A raw DT1 is never dropped.
  - `Push` (recall, ADR 0009) holds `Write`, `Raw`, `ChangeTone` and
    `SwitchStudioSet`. Its items are never replaced by a later write, and
    nothing queued after them is moved before them; `Pushed` is sent when the
    last is handed to the device's queue. Its writes are never dropped as
    unchanged, except those to the tone of a Part the push selected earlier,
    which wait for the read-back and are compared with it.
  - Session boundary: a link Down, or a second Up (a new device ID), ends the
    session. `known` is emptied, and so are the clients' queued work and the
    `DeviceState` (a fresh one replaces it: its queue, its mixer cache, its
    throttle). Queued work is dropped, not replayed or re-validated: it may be
    for another device, or one that was power-cycled and no longer holds what
    the writes assumed. Every outstanding request (queued, read, baseline,
    push) is refused `LinkDown`. What a caller put on `device_mut()` before the
    first Up is kept for that Up.
  - Tone baseline: after a Preset or Expansion tone select (typed, raw DT1, or
    a Program Change on the note port) the service reads the tone's documented
    blocks (`device_spec::tone_read_blocks`: a PCM Synth tone is 8 RQ1s) one at
    a time with core's `BlockReader`, which places short or split replies by
    address. Each RQ1 is an item in the selecting client's queue, in front of
    its later items, so the reads share the device with everyone and the client
    waits for the tone. `Service::baseline(part)` (`Request::Baseline` over the
    protocol) is what arrived. A tone read whole is kept by tone as offsets
    from the Part (Expansion tones for the session only), so the next select of
    it reads nothing. A User tone, and a bank the device does not read back
    (GM2, ExPCM), have none. A drum kit's per-key blocks are not read. A new
    select, a Studio Set select, a note-port change or a session end drops a
    baseline; a reply to a request of a reading that is gone is recognised and
    thrown away; a block unanswered 2 s after its request left is read as
    missing.
  - The note port: a Program Change is still forwarded. It reaches the Parts
    whose Receive Channel is its channel and whose Receive Program Change is
    on, and the Studio Set if the channel is the System Studio Set Control
    Channel (`rx.rs`). The service reads those settings (17 RQ1s, once per
    session) when it first needs them and the Program Change waits, up to
    5 s, after which every Part not ruled out is taken as reached. A Part
    reached forgets its tone from `known`, loses its baseline, is announced as
    `Bulk` `Tone` from the device, and has its dump read again
    (`DeviceState::reread_part`); a Studio Set reached is announced as
    `Bulk` `StudioSet`. When the dump arrives, a Preset or Expansion tone gets
    its baseline read. Bank Select alone changes nothing (it applies at the
    Program Change).
  - A late reply to a mixer read that a tone change dropped is recognised by
    core (`DeviceState::take_stale_reply`) and not applied or cached.
- `Listener` / `Event`: what the service tells an observer (link up with the
  session ID, link down, every message from the device with whether it changed
  the cache). It runs on the service thread and must not block. The client
  protocol is data-shaped `Input`s (see above); a `Listener` still sees the link
events and the device's messages.

## Invariants

- Notes go to the link at once, between SysEx messages and never inside one;
  SysEx keeps core's spacing. From the note port everything passes except
  SysEx, which is dropped, never echoed. Notes are dropped while the link is
  down; SysEx queued before the first Up stays queued for it, and what is
  queued when a session ends is dropped with it.
- Each `link.send` is one whole MIDI message. The service thread never sleeps: a link that just probed reports `sysex_gap`, and the service sends notes and wakes when it ends. Links serialise their own
  writes (the output is behind a lock; a probe goes between messages).
- The SysEx device ID is the one in the device's Identity Reply, unless
  `Config::device_id` fixes it. A link that cannot tell makes the service
  queue an Identity Request first (paced) and hold other SysEx for up to a
  second; a different ID replaces the state and starts a new session.
- A link Down during an Expansion load is deferred until the load ends.
- No environment or working-directory reads: everything is a parameter.
- Device constants come from `integral-core` (`INTEGRA7`, `wireless`,
  `sysex`).

## Verification

- Run: `cargo test -p integral-device-service` (fake link and the sans-IO
  probe, no hardware).
- Run: `just bridge --link rtp --port "Network Integra"` (needs the device).
