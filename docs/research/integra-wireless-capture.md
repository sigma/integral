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
- **Multi-client behaviour, reconnect after a dropped link, and the
  USB-MIDI Thru interaction** were not tested.

## Teardown

On quit, the iPad sends FIN on 23847 (19:39:13.98) and the INTEGRA-7 answers
with FIN. There is no goodbye frame.
