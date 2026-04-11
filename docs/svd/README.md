# SVD File Format — INTEGRA-7

The `.SVD` (Synthesizer Voice Data) file is Roland's proprietary binary
format for backing up and restoring user memory on synthesizers. This
document describes the format as reverse-engineered from INTEGRA-7 backup
files. There is no official public specification from Roland.

## Overview

An SVD file is a flat binary container holding one or more **chunks**, each
containing an array of fixed-size **entries**. Each entry stores a single
patch (tone, studio set, drum kit, etc.) as a **bit-packed** representation
of its SysEx parameters.

```
+------------------+
| File Header      |  16 bytes
+------------------+
| Chunk Directory  |  N x 16 bytes + null terminator
+------------------+
| Chunk 0 Data     |  zone header + entries
+------------------+
| Chunk 1 Data     |
+------------------+
| ...              |
+------------------+
```

## Related Documents

- [Container Format](01-container.md) — header, chunk directory, zone headers
- [Bit-Packing Encoding](02-encoding.md) — how SysEx parameters map to SVD bits
- [SN Synth Tone (SHPa)](03-sn-synth.md) — validated mapping for SuperNATURAL Synth
- [Chunk Type Reference](04-chunk-types.md) — all 6 chunk types and their entry sizes
