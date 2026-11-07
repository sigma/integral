# integral-device-service

## Overview

The Device service
([ADR 0006](../../docs/adr/0006-device-service-owns-the-link.md)): one
process owns the link to the INTEGRA-7 and the one core `DeviceState`, with
its 40 ms throttle, queue and cache. It generalises the Bridge of
[ADR 0003](../../docs/adr/0003-wireless-via-virtual-midi-port.md) from the
Wireless link to every link. `integral-cli bridge` launches it. Native only;
the link, the note port and (next slice) the socket are traits, so a Windows
port adds implementations.

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
- `Listener` / `Event`: what the service tells an observer (link up with the
  session ID, link down, every message from the device with whether it changed
  the cache). It runs on the service thread and must not block. The client
  protocol (#451) adds data-shaped `Input`s and implements `Listener`.

## Invariants

- Notes go to the link at once, between SysEx messages and never inside one;
  SysEx keeps core's spacing. From the note port everything passes except
  SysEx, which is dropped, never echoed. Notes are dropped while the link is
  down; SysEx stays queued.
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
