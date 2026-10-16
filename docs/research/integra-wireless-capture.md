# INTEGRA-7 Wireless Editor Link: Packet Capture

Research for [#168](https://github.com/sigma/integral/issues/168), part of
map [#166](https://github.com/sigma/integral/issues/166).

**Answer:** the link is plain TCP carrying USB-MIDI class packets. Discovery is
one UDP broadcast. There is no encryption and no authentication. The payload is
the same DT1/RQ1 SysEx the Network MIDI path carries.

Only infrastructure mode was captured. Ad-hoc mode was ruled out of scope for
the map.

## Setup

- INTEGRA-7 system 1.21 with the WNA1100-RL adapter, joined to the home router
  by WPS. It got `192.168.1.32`. Its WirelessID is `41288`. USB-MIDI Thru was OFF.
- iPad (iPadOS 26.6.2) on the same Wi-Fi at `192.168.1.132`, running the
  INTEGRA-7 Editor with MIDI device `INTEGRA-7_41288`.
- Capture on a Mac through `rvictl` (`rvi0`), started before the editor was
  launched. Actions, in order: launch and connect, preview, Read, Part 1
  (SN-S) partial 3 off, quit.
- [`integra-wireless-capture.pcap`](integra-wireless-capture.pcap) is trimmed
  to the INTEGRA-7 flows and the iPad's discovery broadcasts (1274 packets,
  19:37:01–19:39:14).

## Discovery: UDP broadcast, TCP callback

The iPad first opens a TCP listener on an ephemeral port, here `53479`
(`0xd0e7`). It then broadcasts 8 bytes to UDP `192.168.1.255:9314` from an
ephemeral port:

```
52 44 44 50 76 31  d0 e7
 R  D  D  P  v  1  └ iPad TCP listen port, u16 BE
```

The INTEGRA-7 answers by **connecting to that port**. Each broadcast gets one
connection, from source ports `23849`, `23850` and `23851` in turn. It sends
84 bytes and closes:

```
52 44 44 50 76 31                    "RDDPv1"
5d 27 6f 8c c3 13                    6-byte ID (not the adapter MAC bc:30:7d:be:71:64)
83 04 00 00                          byte 2 counts 00, 01 across replies
0f d8 05 fc  "Roland\0"              manufacturer
08 0f d8 06 04 0f a0 82 84  "41288\0"   WirelessID
71 d8  "1.21\0"                      system version
d8 05 c8  "INTEGRA-7\0"              model name
01 84 00 00 00 00 00 00 00 00 0f a0 82 84 0f d8 06 0a 0f d8 06 04
```

The strings are plain ASCII. The bytes between them are not decoded. They look
like pointers or tags left over from a native struct, so treat them as opaque.
mDNS plays no part: the only mDNS the iPad sends is its own `_apple-midi._udp`
advertisement.

## Session: TCP 23847

The iPad connects to `192.168.1.32:23847`. Both directions use one frame shape:

```
[type u8][length u24 BE][body: length bytes]
```

| Type | Length | Body | Sender |
|------|--------|------|--------|
| `01` | 4 | device uptime in ms, u32 BE | INTEGRA-7, once, first frame after connect |
| `00` | 0 | (none): heartbeat | both, about 1 s apart, each side answering the other |
| `02` | 4 + 4n | timestamp u32 BE, then n USB-MIDI event packets | both |

The handshake is just this exchange:

```
INTEGRA-7 -> iPad   01 00 00 04  00 13 c8 9e     greeting, uptime 1296542 ms
iPad -> INTEGRA-7   00 00 00 00                  heartbeat
```

`01 00 00 04` plus a "time quad" is what the 2018 port scan reported as an RTP
header. It isn't RTP.

### Type 02: USB-MIDI event packets

Each 4-byte packet is a USB-MIDI 1.0 class event packet: cable number in the
high nibble (always 0 here), Code Index Number (CIN) in the low nibble, then 3
MIDI bytes. SysEx uses CIN 4 (start or continue, 3 bytes), then 5, 6 or 7 (end
with 1, 2 or 3 bytes).

The two sides pack frames differently:

- **iPad -> INTEGRA-7**: one frame per SysEx message, timestamp always `0`.
  The largest frame body seen was 28 bytes (6 packets).
- **INTEGRA-7 -> iPad**: one frame per USB-MIDI packet (body always 8 bytes),
  stamped with its uptime in ms. Every SysEx reply costs 12 wire bytes per 3
  MIDI bytes.

RQ1 for the Part 1 tone (Studio Set Part `18 00 20 06`, 3 bytes):

```
02 00 00 1c  00 00 00 00
04 f0 41 10  04 00 00 64  04 11 18 00  04 20 06 00  04 00 00 03  06 3f f7 00
=> F0 41 10 00 00 64 11 18 00 20 06 00 00 00 03 3F F7
```

The DT1 reply, one packet per frame (only the packets shown):

```
00 00 00 07 | 04 f0 41 10 | 04 00 00 64 | 04 12 18 00 | 04 20 06 5f | 04 40 26 7d | 05 f7 00 00
=> F0 41 10 00 00 64 12 18 00 20 06 5F 40 26 7D F7
```

The partial 3 OFF edit went out as a single DT1:
`F0 41 10 00 00 64 12 19 01 00 1D 00 49 F7`.

Totals for the session: the iPad sent 118 SysEx messages (115 RQ1, 3 DT1), all
well formed. The INTEGRA-7 sent 2412 type-02 frames. The editor waits for each
reply before sending the next RQ1, about 50 ms apart.

### Not understood

- **CIN 0 prefix.** Nearly every INTEGRA-7 reply starts with a packet
  `00 00 00 07` (CIN 0, reserved in USB-MIDI). The first one in the session is
  `08 08 08 08`. A reader that drops CIN 0 packets loses nothing seen here.
- **Non-SysEx packets** from the INTEGRA-7: CIN C (1), D (6) and F (8). These
  are probably MIDI output from the preview.
- **Multi-client behaviour and reconnects:** covered by the probe below. The
  USB-MIDI Thru interaction is not worth testing: Thru could at most echo
  SysEx out of MIDI OUT, which does not touch the wireless session.

## Probe from a non-Roland client

Research for [#302](https://github.com/sigma/integral/issues/302). A throwaway
Python client (since discarded) talked to the same INTEGRA-7 (system 1.21,
`192.168.1.32`) over the same Wi-Fi. It sent RQ1, the Identity Request, and one
DT1 test that wrote a block and then restored it.

**Answer:** the INTEGRA-7 accepts a client it has never seen. It does no check
of any kind. The [feasibility verdict](https://github.com/sigma/integral/issues/169)
stands.

- **Session:** connect to 23847, read the `01` greeting, then send heartbeats.
  The first RQ1 got its DT1 in 53 ms. The reply matched the editor capture
  byte for byte: `F0 41 10 00 00 64 12 18 00 20 06 5F 40 26 7D F7`.
- **Device ID:** the Identity Request `F0 7E 7F 06 01 F7` works over the link.
  The reply is `F0 7E 10 06 02 41 64 02 00 00 00 00 00 00 F7`, so the device
  ID stays discoverable.
- **Liveness timeout: about 3 s.** The INTEGRA-7 sends a heartbeat every
  1.0 s. A client that sends nothing is closed (FIN) 3.0–3.9 s after connect,
  right after the third unanswered heartbeat. Any inbound frame counts as
  liveness: RQ1 traffic every 0.5 s with no heartbeat kept the link up. A client
  can run its own heartbeat clock instead of echoing. A heartbeat every 1.0,
  2.0 or 2.9 s kept the link up for 10 s each.
- **Reconnect:** works at once after a FIN close or a RST close. Five
  connections in a row, about 0.5 s apart, each got a fresh greeting and a
  DT1 reply. Reconnect plus the first reply took 50 ms. No session state
  carries over, and there is no cool-down.
- **Second client:** the TCP connect succeeds, but the second client gets no
  greeting, no heartbeats and no replies. It is not closed either: it waits.
  Once the first client leaves, the second gets its greeting at once and is
  served. So the device serves one session at a time and queues the rest.
  A shipped client that connects but gets no greeting within a few seconds
  should report "in use by another editor".
- **Throttle: still needed.** Sixteen RQ1s back to back (Part 1–16 tone):

  | Gap between RQ1s | Replies |
  |------------------|---------|
  | 0 ms (one frame each, or all 16 in one frame) | 8/16 |
  | 5 ms | 13/16 |
  | 10, 20, 30, 50 ms | 16/16 |
  | wait for each reply (editor style) | 16/16, 68 ms each |

  Excess requests are dropped silently: no error, no late reply. The 20 ms
  rule from the MIDI Implementation applies over Wi-Fi unchanged. Packing many
  SysEx messages into one `02` frame is legal, but it does not bypass the limit.
- **Large replies:** an RQ1 for 77 bytes (the whole Part 1 block) came back as
  one 90-byte DT1. An RQ1 spanning `18 00 00 00` + `00 00 10 00` came back as 5
  DT1s, split by the device, as on MIDI.
- **Large writes:** a full-block DT1 works. The whole Part 1 SN-S MFX block
  (`19 01 02 00`, 145 bytes) went out as one 158-byte DT1 in one `02` frame
  with a 216-byte body. That is 8 times the editor's largest frame. A change to
  the last byte landed, and read-back matched. Writing the original back
  restored the block. The largest block in the address map is the 195-byte PCM
  Drum Kit key (`00 00 01 43`). That gives a 208-byte DT1, so no single-block
  DT1 reaches the 256-byte limit. No part held a PCM drum kit, so that size
  was not sent. A DT1 over 256 bytes was not tried: Integral never sends one
  (the librarian asserts `<= 256`).
- **Stray packets:** one CIN F packet `0F D6 9C 04` arrived with no preview
  playing. CIN 0 `00 00 00 07` packets still prefix some replies. Their
  meaning is still unknown; dropping both loses nothing.
- **Network MIDI was down** during the probe (`just ping` over "Network
  Integra" had no response), so the wireless link worked with no other MIDI
  path in use.

## Teardown

On quit, the iPad sends FIN on 23847 (19:39:13.98) and the INTEGRA-7 answers
with FIN. There is no goodbye frame.
