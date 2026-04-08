# INTEGRA-7 SysEx Protocol Reference

Source: Roland INTEGRA-7 MIDI Implementation (Model ID `00H 00H 64H`)

---

## 1. SysEx Protocol Overview

The Roland INTEGRA-7 uses MIDI System Exclusive (SysEx) messages for reading
and writing internal parameters. All Roland-specific messages share a common
header structure.

| Field             | Value           | Description                                      |
|-------------------|-----------------|--------------------------------------------------|
| Manufacturer ID   | `41H`           | Roland                                           |
| Model ID          | `00H 00H 64H`  | INTEGRA-7                                        |
| Device ID range   | `10H` - `1FH`  | Corresponds to device numbers 17-32              |
| Broadcast ID      | `7FH`           | Received regardless of device ID setting         |

Universal SysEx ID numbers:

| ID     | Type                             |
|--------|----------------------------------|
| `7EH`  | Universal Non-realtime Message   |
| `7FH`  | Universal Realtime Message       |

The INTEGRA-7 receives DT1, RQ1, Universal Non-realtime, and Universal
Realtime SysEx messages. It transmits only Universal Non-realtime messages
(Identity Reply) and DT1 messages.

---

## 2. DT1 (Data Set 1) Message Format

DT1 writes parameter data to the device. Command ID: `12H`.

### Byte Layout

```
F0 41 dev 00 00 64 12 aa bb cc dd ee ... ff sum F7
```

| Byte(s)          | Field           | Description                                                |
|------------------|-----------------|------------------------------------------------------------|
| `F0`             | Status          | Exclusive status (start of SysEx)                          |
| `41`             | Manufacturer ID | Roland                                                     |
| `dev`            | Device ID       | `10H`-`1FH` (receive), `00H`-`1FH` (transmit), or `7FH`  |
| `00 00 64`       | Model ID        | INTEGRA-7                                                  |
| `12`             | Command ID      | DT1 (Data Set 1)                                           |
| `aa`             | Address MSB     | Upper byte of the starting address                         |
| `bb`             | Address         | Upper-middle byte of the starting address                  |
| `cc`             | Address         | Lower-middle byte of the starting address                  |
| `dd`             | Address LSB     | Lower byte of the starting address                         |
| `ee` ... `ff`    | Data            | One or more data bytes, sent sequentially from the address |
| `sum`            | Checksum        | Roland checksum (see section 6)                            |
| `F7`             | EOX             | End of Exclusive                                           |

### Notes

- The amount of data that can be transmitted at one time depends on the
  parameter type. Refer to the Parameter Address Map for valid starting
  addresses and sizes.
- Data larger than 256 bytes is divided into packets of 256 bytes or less,
  each sent at an interval of approximately 20 ms.
- Not received when the Rx Exclusive parameter (`SYSTEM:MIDI`) is OFF.

---

## 3. RQ1 (Data Request 1) Message Format

RQ1 requests the device to transmit data. Command ID: `11H`. The device
responds with a DT1 message if the address and size are valid.

### Byte Layout

```
F0 41 dev 00 00 64 11 aa bb cc dd ss tt uu vv sum F7
```

| Byte(s)          | Field           | Description                                                |
|------------------|-----------------|------------------------------------------------------------|
| `F0`             | Status          | Exclusive status (start of SysEx)                          |
| `41`             | Manufacturer ID | Roland                                                     |
| `dev`            | Device ID       | `10H`-`1FH`, or `7FH`                                     |
| `00 00 64`       | Model ID        | INTEGRA-7                                                  |
| `11`             | Command ID      | RQ1 (Data Request 1)                                       |
| `aa`             | Address MSB     | Upper byte of the starting address                         |
| `bb`             | Address         | Upper-middle byte of the starting address                  |
| `cc`             | Address         | Lower-middle byte of the starting address                  |
| `dd`             | Address LSB     | Lower byte of the starting address                         |
| `ss`             | Size MSB        | Upper byte of the data size                                |
| `tt`             | Size            | Upper-middle byte of the data size                         |
| `uu`             | Size            | Lower-middle byte of the data size                         |
| `vv`             | Size LSB        | Lower byte of the data size                                |
| `sum`            | Checksum        | Roland checksum (see section 6)                            |
| `F7`             | EOX             | End of Exclusive                                           |

### Notes

- Data requests must use the fixed starting address and size for each
  parameter type as given in the Parameter Address Map.
- If the device cannot transmit (invalid address/size, or device is busy),
  nothing is sent in response.

---

## 4. Universal Non-realtime Messages

### Identity Request

Sent to the INTEGRA-7 to request device identification.

```
F0 7E dev 06 01 F7
```

| Byte   | Description                                     |
|--------|-------------------------------------------------|
| `F0`   | Exclusive status                                |
| `7E`   | ID number (Universal Non-realtime Message)      |
| `dev`  | Device ID (`10H`-`1FH`, or `7FH` for broadcast)|
| `06`   | Sub ID#1 (General Information)                  |
| `01`   | Sub ID#2 (Identity Request)                     |
| `F7`   | EOX                                             |

### Identity Reply

Transmitted by the INTEGRA-7 in response to an Identity Request.

```
F0 7E dev 06 02 41 64 02 00 00 00 00 00 00 F7
```

| Byte(s)            | Description                                |
|--------------------|--------------------------------------------|
| `F0`               | Exclusive status                           |
| `7E`               | ID number (Universal Non-realtime Message) |
| `dev`              | Device ID (`10H`-`1FH`)                   |
| `06`               | Sub ID#1 (General Information)             |
| `02`               | Sub ID#2 (Identity Reply)                  |
| `41`               | ID number (Roland)                         |
| `64 02`            | Device family code                         |
| `00 00`            | Device family number code                  |
| `00 00 00 00`      | Software revision level                    |
| `F7`               | EOX                                        |

---

## 5. Universal Realtime Messages

All Universal Realtime messages use Device ID `7FH` (broadcast).

### Master Volume

```
F0 7F 7F 04 01 ll mm F7
```

| Byte   | Description                               |
|--------|-------------------------------------------|
| `F0`   | Exclusive status                          |
| `7F`   | ID number (Universal Realtime Message)    |
| `7F`   | Device ID (Broadcast)                     |
| `04`   | Sub ID#1 (Device Control)                 |
| `01`   | Sub ID#2 (Master Volume)                  |
| `ll`   | Master Volume lower byte (treated as 00H) |
| `mm`   | Master Volume upper byte                  |
| `F7`   | EOX                                       |

- The lower byte (`ll`) is handled as `00H`.
- Changes the Master Level parameter (`SYSTEM:SOUND`).

### Master Fine Tuning

```
F0 7F 7F 04 03 ll mm F7
```

| Byte   | Description                               |
|--------|-------------------------------------------|
| `F0`   | Exclusive status                          |
| `7F`   | ID number (Universal Realtime Message)    |
| `7F`   | Device ID (Broadcast)                     |
| `04`   | Sub ID#1 (Device Control)                 |
| `03`   | Sub ID#2 (Master Fine Tuning)             |
| `ll`   | Master Fine Tuning LSB                    |
| `mm`   | Master Fine Tuning MSB                    |
| `F7`   | EOX                                       |

- Value range: `mm ll` = `00 00H` - `40 00H` - `7F 7FH` (-100 to 0 to +99.9 cents)
- Changes the Master Tune parameter (`SYSTEM:SOUND`).

### Master Coarse Tuning

```
F0 7F 7F 04 04 ll mm F7
```

| Byte   | Description                               |
|--------|-------------------------------------------|
| `F0`   | Exclusive status                          |
| `7F`   | ID number (Universal Realtime Message)    |
| `7F`   | Device ID (Broadcast)                     |
| `04`   | Sub ID#1 (Device Control)                 |
| `04`   | Sub ID#2 (Master Coarse Tuning)           |
| `ll`   | Master Coarse Tuning LSB (ignored as 00H) |
| `mm`   | Master Coarse Tuning MSB                  |
| `F7`   | EOX                                       |

- `mm` value range: `28H` - `40H` - `58H` (-24 to 0 to +24 semitones)
- `ll` is ignored (processed as `00H`).
- Changes the Master Key Shift parameter (`SYSTEM:SOUND`).

---

## 6. Checksum Algorithm

Roland SysEx messages (DT1 and RQ1) include a checksum byte immediately
before the `F7` terminator. The checksum validates that the address and data
(or size) bytes were received correctly.

### Formula

Given address bytes `aa bb cc dd` and data/size bytes `ee ff ...`:

```
sum = aa + bb + cc + dd + ee + ff + ...
checksum = (128 - (sum % 128)) % 128
```

Equivalently: add all address and data (or size) bytes, divide by 128, and
subtract the remainder from 128. If the remainder is 0, the checksum is 0.

### Verification

When the checksum is correct, adding all address bytes, data bytes, and the
checksum itself will produce a value whose lower 7 bits are zero:

```
(aa + bb + cc + dd + ee + ff + ... + checksum) % 128 == 0
```

### Worked Example

Setting Studio Set Reverb Type to "Room 2" via DT1:

```
Address computation:
  Temporary Studio Set base:  18 00 00 00
  + Reverb offset:               06 00
  + Reverb Type offset:          00 00
  = Final address:            18 00 06 00

Data value:
  Room 2 = 02H

Checksum calculation:
  18H + 00H + 06H + 00H + 02H = 24 + 0 + 6 + 0 + 2 = 32
  32 % 128 = 32
  128 - 32 = 96 = 60H

Complete message:
  F0 41 10 00 00 64 12 18 00 06 00 02 60 F7
```

---

## 7. Transmission Rules

### 256-Byte Packet Limit

Data larger than 256 bytes must be divided into packets of 256 bytes or
fewer. Each packet is a complete SysEx message with its own header, address,
checksum, and terminator. The address of each subsequent packet advances by
the number of data bytes in the previous packet.

### 20 ms Inter-Packet Interval

Consecutive packets must be separated by an interval of approximately **20
ms**. This gives the receiving device time to process each packet before the
next one arrives.

### Nibblized Data

Parameters marked with `#` in the Parameter Address Map use nibblized
(nibble-split) data encoding. Each byte of the original value is split into
two 7-bit MIDI bytes carrying 4 bits each (high nibble first, low nibble
second).

For example, the hex value `ABH` is transmitted as two bytes: `0AH 0BH`.

To encode a multi-byte value, split each byte into its high and low nibbles
and transmit them in order:

```
Original byte:   ABH
High nibble:     0AH  (A = 10)
Low nibble:      0BH  (B = 11)
Transmitted as:  0A 0B
```

To decode nibblized data, recombine each pair:

```
value = (high_nibble << 4) | low_nibble
0AH * 16 + 0BH = 10 * 16 + 11 = 171 = ABH
```

For a 4-byte nibblized value `0a 0b 0c 0d`:

```
value = ((a * 16 + b) * 16 + c) * 16 + d
```

### Active Sensing

The INTEGRA-7 transmits Active Sensing (`FEH`) at intervals of approximately
250 ms.

### Rx Exclusive Parameter

DT1 and RQ1 messages are not received when the Rx Exclusive parameter
(`SYSTEM:MIDI`) is set to OFF.
