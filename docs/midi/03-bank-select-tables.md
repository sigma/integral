# Bank Select Reference

This document provides the complete Bank Select mapping for the Roland INTEGRA-7.
Bank Select is sent as two Control Change messages: CC#0 (MSB) and CC#32 (LSB),
followed by a Program Change to select the specific tone or kit.

> **Note:** Bank Select is not received when the Rx Bank Select parameter
> (SYSTEM:MIDI) or (PART VIEW:MIDI) is OFF.

---

## Studio Sets

| MSB | LSB | Program Number | Group      | Number Range |
|-----|-----|----------------|------------|--------------|
| 85  | 0   | 1--64          | Studio Set | 01--64       |

---

## SuperNATURAL Acoustic Tones

| MSB | LSB   | Program Number | Group                    | Number Range |
|-----|-------|----------------|--------------------------|--------------|
| 89  | 0--1  | 1--128         | User SN Acoustic Tone    | 0001--0256   |
| 89  | 64--65| 1--128         | Preset SN Acoustic Tone  | 0001--0256   |

---

## SuperNATURAL Synth Tones

| MSB | LSB   | Program Number | Group                 | Number Range |
|-----|-------|----------------|-----------------------|--------------|
| 95  | 0--3  | 1--128         | User SN Synth Tone    | 0001--0512   |
| 95  | 64    | 1--128         | Preset SN Synth Tone  | 0001--0128   |
| 95  | 65    | 1--128         | Preset SN Synth Tone  | 0129--0256   |
| 95  | 66    | 1--128         | Preset SN Synth Tone  | 0257--0384   |
| 95  | 67    | 1--128         | Preset SN Synth Tone  | 0385--0512   |
| 95  | 68    | 1--128         | Preset SN Synth Tone  | 0513--0640   |
| 95  | 69    | 1--128         | Preset SN Synth Tone  | 0641--0768   |
| 95  | 70    | 1--128         | Preset SN Synth Tone  | 0769--0896   |
| 95  | 71    | 1--128         | Preset SN Synth Tone  | 0897--1024   |
| 95  | 72    | 1--85          | Preset SN Synth Tone  | 1025--1109   |

---

## SuperNATURAL Drum Kits

| MSB | LSB | Program Number | Group               | Number Range |
|-----|-----|----------------|---------------------|--------------|
| 88  | 0   | 1--64          | User SN Drum Kit    | 0001--0064   |
| 88  | 64  | 1--26          | Preset SN Drum Kit  | 0001--0026   |

---

## PCM Synth Tones

| MSB | LSB   | Program Number | Group                | Number Range |
|-----|-------|----------------|----------------------|--------------|
| 87  | 0--1  | 1--128         | User PCM Synth Tone  | 0001--0256   |
| 87  | 64--70| 1--128         | Preset PCM Synth Tone| 0001--0896   |
| 121 | 0     | 1--128         | GM2 Tone             | 0001--0256   |

---

## PCM Drum Kits

| MSB | LSB | Program Number | Group              | Number Range |
|-----|-----|----------------|--------------------|--------------|
| 86  | 0   | 1--32          | User PCM Drum Kit  | 0001--0032   |
| 86  | 64  | 1--14          | Preset PCM Drum Kit| 0001--0014   |
| 120 | 0   | 1--57          | GM2 Drum Kit       | 0001--0009   |

---

## Expansion Sounds

### Expansion PCM Tones (MSB 93)

| MSB | LSB   | Program Number | Group                         | Number Range |
|-----|-------|----------------|-------------------------------|--------------|
| 93  | 0     | 1--41          | Expansion PCM Tone (SRX-01)   | 0001--0041   |
| 93  | 1     | 1--50          | Expansion PCM Tone (SRX-02)   | 0001--0050   |
| 93  | 2     | 1--128         | Expansion PCM Tone (SRX-03)   | 0001--0128   |
| 93  | 3     | 1--128         | Expansion PCM Tone (SRX-04)   | 0001--0128   |
| 93  | 4     | 1--128         | Expansion PCM Tone (SRX-05)   | 0001--0128   |
| 93  | 5     | 1--128         | Expansion PCM Tone (SRX-05)   | 0129--0256   |
| 93  | 6     | 1--56          | Expansion PCM Tone (SRX-05)   | 0257--0312   |
| 93  | 7     | 1--128         | Expansion PCM Tone (SRX-06)   | 0001--0128   |
| 93  | 8     | 1--128         | Expansion PCM Tone (SRX-06)   | 0129--0256   |
| 93  | 9     | 1--128         | Expansion PCM Tone (SRX-06)   | 0257--0384   |
| 93  | 10    | 1--65          | Expansion PCM Tone (SRX-06)   | 0385--0449   |
| 93  | 11    | 1--128         | Expansion PCM Tone (SRX-07)   | 0001--0128   |
| 93  | 12    | 1--128         | Expansion PCM Tone (SRX-07)   | 0129--0256   |
| 93  | 13    | 1--128         | Expansion PCM Tone (SRX-07)   | 0257--0384   |
| 93  | 14    | 1--91          | Expansion PCM Tone (SRX-07)   | 0385--0475   |
| 93  | 15    | 1--128         | Expansion PCM Tone (SRX-08)   | 0001--0128   |
| 93  | 16    | 1--128         | Expansion PCM Tone (SRX-08)   | 0129--0256   |
| 93  | 17    | 1--128         | Expansion PCM Tone (SRX-08)   | 0257--0384   |
| 93  | 18    | 1--64          | Expansion PCM Tone (SRX-08)   | 0385--0448   |
| 93  | 19    | 1--128         | Expansion PCM Tone (SRX-09)   | 0001--0128   |
| 93  | 20    | 1--128         | Expansion PCM Tone (SRX-09)   | 0129--0256   |
| 93  | 21    | 1--128         | Expansion PCM Tone (SRX-09)   | 0257--0384   |
| 93  | 22    | 1--30          | Expansion PCM Tone (SRX-09)   | 0385--0414   |
| 93  | 23    | 1--100         | Expansion PCM Tone (SRX-10)   | 0001--0100   |
| 93  | 24    | 1--42          | Expansion PCM Tone (SRX-11)   | 0001--0042   |
| 93  | 26    | 1--50          | Expansion PCM Tone (SRX-12)   | 0001--0050   |

### Expansion PCM Drums (MSB 92)

| MSB | LSB | Program Number | Group                         | Number Range |
|-----|-----|----------------|-------------------------------|--------------|
| 92  | 0   | 1--79          | Expansion PCM Drum (SRX-01)   | 0001--0079   |
| 92  | 2   | 1--12          | Expansion PCM Drum (SRX-03)   | 0001--0012   |
| 92  | 4   | 1--34          | Expansion PCM Drum (SRX-05)   | 0001--0034   |
| 92  | 7   | 1--5           | Expansion PCM Drum (SRX-06)   | 0001--0005   |
| 92  | 11  | 1--11          | Expansion PCM Drum (SRX-07)   | 0001--0011   |
| 92  | 15  | 1--21          | Expansion PCM Drum (SRX-08)   | 0001--0021   |
| 92  | 19  | 1--12          | Expansion PCM Drum (SRX-09)   | 0001--0012   |

### Expansion SuperNATURAL Tones (MSB 89)

| MSB | LSB | Program Number | Group                        | Number Range |
|-----|-----|----------------|------------------------------|--------------|
| 89  | 96  | 1--17          | Expansion SN Tone (ExSN1)    | 0001--0017   |
| 89  | 97  | 1--17          | Expansion SN Tone (ExSN2)    | 0001--0017   |
| 89  | 98  | 1--50          | Expansion SN Tone (ExSN3)    | 0001--0050   |
| 89  | 99  | 1--12          | Expansion SN Tone (ExSN4)    | 0001--0012   |
| 89  | 100 | 1--12          | Expansion SN Tone (ExSN5)    | 0001--0012   |

### Expansion SuperNATURAL Drum Kits (MSB 88)

| MSB | LSB | Program Number | Group                        | Number Range |
|-----|-----|----------------|------------------------------|--------------|
| 88  | 101 | 1--7           | Expansion SN Drum (ExSN6)    | 0001--0007   |

### Expansion PCM Tones -- ExPCM (MSB 97)

| MSB | LSB | Program Number | Group                        | Number Range |
|-----|-----|----------------|------------------------------|--------------|
| 97  | 0   | 1--128         | Expansion PCM Tone (ExPCM)   | 0001--0128   |
| 97  | 1   | 1--128         | Expansion PCM Tone (ExPCM)   | 0129--0256   |
| 97  | 2   | 1--128         | Expansion PCM Tone (ExPCM)   | 0257--0384   |
| 97  | 3   | 1--128         | Expansion PCM Tone (ExPCM)   | 0385--0512   |

### Expansion PCM Drums -- ExPCM (MSB 96)

| MSB | LSB | Program Number | Group                        | Number Range |
|-----|-----|----------------|------------------------------|--------------|
| 96  | 0   | 1--19          | Expansion PCM Drum (ExPCM)   | 0001--0019   |

### Expansion GM2 (MSB 121 / 120)

| MSB | LSB | Program Number | Group                        | Number Range |
|-----|-----|----------------|------------------------------|--------------|
| 121 | 0   | 1--128         | Expansion GM2 Tone (GM2#)    | 0001--0256   |
| 120 | 0   | 1--57          | Expansion GM2 Drum (GM2#)    | 0001--0009   |

---

## Quick Reference: MSB Summary

| MSB | Type                                |
|-----|-------------------------------------|
| 85  | Studio Set                          |
| 86  | PCM Drum Kit (User / Preset)        |
| 87  | PCM Synth Tone (User / Preset)      |
| 88  | SN Drum Kit (User / Preset / ExSN6) |
| 89  | SN Acoustic Tone (User / Preset / ExSN1--5) |
| 92  | Expansion PCM Drum (SRX)            |
| 93  | Expansion PCM Tone (SRX)            |
| 95  | SN Synth Tone (User / Preset)       |
| 96  | Expansion PCM Drum (ExPCM)          |
| 97  | Expansion PCM Tone (ExPCM)          |
| 120 | GM2 Drum Kit / Expansion GM2 Drum   |
| 121 | GM2 Tone / Expansion GM2 Tone       |
