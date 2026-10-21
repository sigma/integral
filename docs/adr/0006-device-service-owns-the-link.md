# One out-of-process Device service owns the device link

A single **Device service** process owns the link to the INTEGRA-7 and the one `DeviceState`. The link can be a Wireless link, an RTP-MIDI port or a USB port. Plugin instances are thin clients of the service. It is the Bridge from [ADR 0003](0003-wireless-via-virtual-midi-port.md), generalised from the Wireless link to every link and given the device state. It lives in its own crate, and `integral-cli bridge` becomes a launcher for it. Decided in [Who owns the device link?](https://github.com/sigma/integral/issues/341).

## Consequences

- **Out of process, never an in-plugin singleton.** Hosts that sandbox or bridge plugins put instances in separate processes, so no in-plugin singleton can be the single owner ([#338](https://github.com/sigma/integral/issues/338)). Elektron's Overbridge is the one precedent for a service that owns the link, and it is out of process ([#337](https://github.com/sigma/integral/issues/337)).
- **Host SysEx routing is not used.** Ableton Live drops plugin SysEx ([#328](https://github.com/sigma/integral/issues/328)), and [#335](https://github.com/sigma/integral/issues/335) stays moot.
- **One throttle, one cache.** The service runs core's `DeviceState`, including its pacing, queue and cache. No client paces its own SysEx to the device.
- **Typed protocol over a local socket** (a Unix domain socket, or a named pipe on Windows):
  - **Requests:** writes and reads named by core field enum plus Scope plus value, not raw bytes.
  - **Subscriptions:** per-Scope change notifications.
  - **Raw SysEx:** an escape hatch for Librarian bulk operations.
  - **No gestures:** automation begin and end stay between a plugin and its host.
- **Notes stay host-scheduled.** DAW MIDI tracks send notes to the virtual port the service exposes, and the service merges them onto the link. **Notes jump the SysEx queue:** they go out immediately, between SysEx messages and never inside one, while SysEx keeps core's spacing. This keeps a note from waiting behind throttled SysEx.
- **Ownership is a convention, not a lock.** CoreMIDI shares the hardware port ([#339](https://github.com/sigma/integral/issues/339)). The web UI and CLI keep their own access, writes are last-writer-wins, and the service's cache can go stale when they write. Several hosts may connect to one service. Two instances bound to the same Part both work, with a warning in their UIs.
- **Roland's "INTEGRA-7 CTRL" port is not relied on.** It would split SysEx from notes with no merge, but it exists only over USB, maybe only in VENDER mode, and never over a Wireless link or RTP-MIDI. It stays a possible later optimisation.
- **Windows support follows Windows MIDI Services:** sharing the port needs Windows 11 24H2 or later, and the service's own virtual port needs 25H2 or later ([#339](https://github.com/sigma/integral/issues/339)).
