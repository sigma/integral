# Wireless link reached through a virtual MIDI port

The web UI reaches the INTEGRA-7's Wireless link through a native **Bridge** (`integral-cli bridge`) that holds the TCP session and exposes it as a virtual MIDI port named `INTEGRA-7 (Wireless)`. The web UI keeps using Web MIDI and changes nothing. The protocol itself (frame and RDDP codecs, session state machine) lives sans-IO in `integral-core::wireless`; the Bridge owns the sockets and the port. Decided in [How does the browser reach the INTEGRA-7 over wireless?](https://github.com/sigma/integral/issues/303) and [Write the INTEGRA-7 wireless transport spec](https://github.com/sigma/integral/issues/304); the protocol is in [`docs/specs/wireless-transport.md`](../specs/wireless-transport.md).

## Consequences

- A plain web page cannot open TCP or UDP, so some native process is unavoidable. A WebSocket helper was rejected: it needs a second web transport and localhost-origin security, for cross-platform reach we don't need yet. Leaving wireless native-only was rejected: the web UI is the primary surface.
- Any MIDI app (DAW, the VST in a host, every other `integral-cli` command) gets wireless for free, through the same port pattern as a MIDI interface.
- Windows is excluded: it has no built-in virtual MIDI ports, and `midir` virtual ports are unix-only. Tracked in [Reach the INTEGRA-7 over wireless from the web UI on Windows](https://github.com/sigma/integral/issues/305).
- The page's only link-state signal is the port appearing and disappearing (Web MIDI `statechange`). It cannot tell "no Bridge" from "device in use by another editor".
- Core stays socket-free, so it still builds for `wasm32-unknown-unknown`, and the protocol is tested without a device.
