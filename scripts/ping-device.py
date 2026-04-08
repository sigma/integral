#!/usr/bin/env python3
"""Ping the Roland INTEGRA-7 via SysEx Identity Request.

Sends a Universal Non-realtime Identity Request message and verifies
the reply contains the INTEGRA-7 device family code.

Usage:
    python scripts/ping-device.py [--port NAME] [--timeout SECONDS]

Defaults to searching for a port containing "Integra" in its name.
"""

import argparse
import sys
import time

import mido

# Identity Request: F0 7E 7F 06 01 F7
# 7E = Universal Non-realtime, 7F = broadcast, 06 = General Information, 01 = Identity Request
IDENTITY_REQUEST = mido.Message("sysex", data=[0x7E, 0x7F, 0x06, 0x01])

# Expected in reply: manufacturer=41 (Roland), family=64 02 (INTEGRA-7)
ROLAND_ID = 0x41
INTEGRA7_FAMILY = (0x64, 0x02)


def find_port(pattern: str) -> str | None:
    """Find a MIDI port whose name contains `pattern` (case-insensitive)."""
    pattern_lower = pattern.lower()
    for name in mido.get_output_names():
        if pattern_lower in name.lower():
            # Verify it exists as both input and output
            for iname in mido.get_input_names():
                if iname == name:
                    return name
    return None


def ping(port_name: str, timeout: float) -> dict | None:
    """Send Identity Request and wait for Identity Reply.

    Returns a dict with device info on success, None on timeout.
    """
    mido.set_backend("mido.backends.rtmidi")

    with mido.open_output(port_name) as out, mido.open_input(port_name) as inp:
        # Drain any pending messages
        while inp.poll() is not None:
            pass

        out.send(IDENTITY_REQUEST)
        deadline = time.monotonic() + timeout

        while time.monotonic() < deadline:
            msg = inp.poll()
            if msg is None:
                time.sleep(0.01)
                continue

            if msg.type != "sysex":
                continue

            data = msg.data
            # Identity Reply: 7E dev 06 02 41 64 02 00 00 xx xx xx xx
            if (
                len(data) >= 10
                and data[0] == 0x7E
                and data[2] == 0x06
                and data[3] == 0x02  # Identity Reply
                and data[4] == ROLAND_ID
                and data[5] == INTEGRA7_FAMILY[0]
                and data[6] == INTEGRA7_FAMILY[1]
            ):
                return {
                    "device_id": data[1],
                    "manufacturer": "Roland",
                    "family_code": f"{data[5]:02X} {data[6]:02X}",
                    "family_number": f"{data[7]:02X} {data[8]:02X}",
                    "revision": " ".join(f"{b:02X}" for b in data[9:13]),
                }

    return None


def main() -> int:
    parser = argparse.ArgumentParser(description="Ping the INTEGRA-7 via SysEx")
    parser.add_argument(
        "--port",
        default="Integra",
        help="MIDI port name or substring to match (default: Integra)",
    )
    parser.add_argument(
        "--timeout",
        type=float,
        default=2.0,
        help="Timeout in seconds (default: 2.0)",
    )
    parser.add_argument(
        "--quiet",
        action="store_true",
        help="Only output exit code, no text",
    )
    args = parser.parse_args()

    port_name = find_port(args.port)
    if port_name is None:
        if not args.quiet:
            print(f"FAIL: No MIDI port matching '{args.port}' found")
            print(f"Available ports: {mido.get_output_names()}")
        return 1

    if not args.quiet:
        print(f"Pinging INTEGRA-7 on '{port_name}'...", end=" ", flush=True)

    result = ping(port_name, args.timeout)

    if result is None:
        if not args.quiet:
            print("FAIL: No response (device off or wrong port?)")
        return 1

    if not args.quiet:
        print("OK")
        print(f"  Device ID:     {result['device_id']:02X}H")
        print(f"  Manufacturer:  {result['manufacturer']}")
        print(f"  Family:        {result['family_code']}")
        print(f"  Family Number: {result['family_number']}")
        print(f"  Revision:      {result['revision']}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
