#!/usr/bin/env python3
"""Regenerate the golden test vectors from the reference client.

Point PLOOM_REFERENCE at the directory containing ploom.py and run with an
interpreter that has bleak installed (imported, never used here).
"""

import json
import os
import random
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent

ref = os.environ.get("PLOOM_REFERENCE")
if not ref:
    sys.exit("set PLOOM_REFERENCE to the directory containing ploom.py")
sys.path.insert(0, str(Path(ref).expanduser()))
sys.dont_write_bytecode = True  # never write into the reference checkout
import ploom  # noqa: E402

PRESETS = ["DELICATE", "INTENSE", "BATTERY_SAVER"]

# Every boundary of the masters[7] -> CB-constant table, plus one value just
# outside each end.
LEAFLET_KEYS = [1121, 1122, 1145, 1146, 1170, 1171, 1194, 1195,
                1244, 1245, 1268, 1269, 1293, 1294, 1330, 1331]


def realistic_masters(rng, key, n=18):
    """Plausible calibration: resistances around 3000, masters[4] in tenths
    of a degree, masters[8] near masters[7]."""
    m = [rng.randint(2600, 3600) for _ in range(max(n, 16))]
    m[4] = rng.randint(2300, 2700)
    m[7] = key
    m[8] = rng.randint(1100, 1350)
    m[11] = rng.randint(800, 1600)
    m[15] = rng.randint(800, 1600)
    return m[:n]


def half_masters(m4):
    """Masters for which getRes lands exactly on .5 before rounding."""
    m = [3000 + i for i in range(18)]
    m[3] = 3001
    m[4] = m4
    m[7] = 1200
    m[8] = 600
    m[11] = 1500
    m[15] = 1500
    return m


def frames_or_error(mode, masters):
    try:
        out = ploom.build_frames(mode, masters)
    except RuntimeError as e:
        msg = str(e)
        if "неполная" in msg:
            kind = "tooShort"
        elif "masters[8]" in msg:
            kind = "zeroDivisor"
        elif "вне диапазона" in msg:
            kind = "outOfRange"
        else:
            raise
        return {"error": kind}
    return {"frames": [{"hex": fr.hex(), "awaits": exp} for fr, exp in out]}


def gen_build_frames(rng):
    cases = []
    for key in LEAFLET_KEYS:
        masters = realistic_masters(rng, key)
        for mode in PRESETS:
            cases.append({"name": f"{mode} key={key}", "mode": mode, "custom": None,
                          "masters": masters, "result": frames_or_error(mode, masters)})

    specials = {
        "half@270": half_masters(2695),
        "half@240": half_masters(2395),
        "near-half@270": half_masters(2694),
        "exactly-16": realistic_masters(rng, 1200, n=16),
        "too-short": realistic_masters(rng, 1200, n=15),
    }
    zero = realistic_masters(rng, 1200)
    zero[8] = 0
    specials["zero-divisor"] = zero
    for label, masters in specials.items():
        for mode in PRESETS:
            cases.append({"name": f"{mode} {label}", "mode": mode, "custom": None,
                          "masters": masters, "result": frames_or_error(mode, masters)})

    customs = [(240, 30), (320, 255), (270, 120), (100, 0), (999, 9999), (-50, -5),
               (255, 231), (281, 77), (241, 31), (319, 254)]
    masters_sets = [("half@270", half_masters(2695)),
                    ("realistic", realistic_masters(rng, 1250)),
                    ("realistic-low", realistic_masters(rng, 1130))]
    for base in PRESETS:
        for temp, secs in customs:
            for label, masters in masters_sets:
                ploom.make_custom(base, temp, secs)
                cases.append({
                    "name": f"custom {base} {temp}C {secs}s {label}",
                    "mode": base,
                    "custom": {"temp": temp, "secs": secs},
                    "masters": masters,
                    "result": frames_or_error("CUSTOM", masters),
                })
    return cases


def gen_get_res(rng):
    temps = sorted({0, 1000, 221, 240, 245, 250, 255, 256, 275, 300, 320,
                    -220, -245, -255, -300, -320, *range(240, 321)})
    sets = [realistic_masters(rng, 1100 + 20 * i) for i in range(6)]
    sets += [half_masters(2695), half_masters(2395), half_masters(2694)]
    out = []
    for masters in sets:
        out.append({"masters": masters,
                    "results": [[t, ploom.get_res(t, masters)] for t in temps]})
    return out


# Live captures from PROTOCOL.md plus synthetic frames for every opcode the
# reference decodes, short payloads and unknown opcodes.
PARSE_FRAMES = [
    "10 30 43 14 00 00 00 00 00 00 ff 02 08 01 00 01 00",  # live 0x30: locked, auto-start, downloaded
    "1030 0000 0000 60 01 00 00 00 ff 00 00 00 01 00",  # 0x30: in-use+preheat bits, unlocked
    "1030 0100 0000 10 11 02 00 00 00 00 00 00 02 00",  # charging bit, feature off, p2p
    "1030 0100 0000 08 05 07 00 00 00 00 00 01 07 01",  # temp error, unknown enums
    "0f30 0100 0000 00 00 00 00 00 00 00 00 00 00",     # 0x30 one byte short
    "0c18 08 01 03 00 00 00 00 01 01 00 00",            # 0x18
    "0c18 00 11 04 00 00 00 00 00 00 02 01",            # 0x18 feature off
    "0b18 00 11 04 00 00 00 00 00 00 02",               # 0x18 short
    "04310100 00", "04310000 01", "0431020000", "0431040000", "0431080000",
    "0431100000", "0431200000", "0431400000", "0431060000", "0431030000",
    "0431800000", "0431810000", "0431ff0000", "0131",
    "02335f", "0133",
    "0634000300 0000", "0634000700 0000", "0634000600 0000", "0634000900 0000", "0234 00",
    "0535060007 00", "0435060007",
    "1336b03300006801000000010001 0e00000000b095",       # live 0x36
    "1236b03300006801000000010001 0e00000000b0",         # 0x36 short
    "13370e005d009000d000e60009012401 57010000",         # live 0x37
    "13380000000000000000000000000000 00000000",
    "1338 0100 0200 0000 0400 0000 0000 0000 0000 0900",
    "0f39 1000 1100 0000 0000 0000 0000 0000",
    "0237 05", "0137",
    "133a1b01ad02a401620d9c0b00000000 00000000",         # live 0x3A
    "0a3a 1b01 ad02 a401 620d 9c",
    "133b030000002f010000fd0d960ed00b e60b0000",         # live 0x3B (LEN 19)
    "113b030000002f010000fd0d960ed00b e60b",
    "103b030000002f010000fd0d960ed00b e6",
    "033c5500", "023c64", "013c",
    "023d01", "023d00", "023d02", "013d",
    "083e000100000000 00", "083e000200000000 00", "083e000300000000 00",
    "053e000e0000", "023e00",
    "0f400200 01000000 5f 00000000000000", "0f40 0001 00000100 64 00000000000000",
    "0740 0200 0100 0000",
    "0a50 0102030405060708 09", "0150",
    "0af4 aabbccddeeff001122",
    "027a00", "027a01", "017a",
    "0c9f00 00000000000000000000", "0c9f01 00000000000000000000", "029f11", "029f05",
    "02fcc4", "02fc00", "02fc7f", "02fc80",
    "023f00", "023f01", "024900", "024b02",
    "0d44 0100 0200 0300 0400 0500 0600", "0d45 0700 b004 0900 0a00 0b00 0c00",
    "0946 0d00 0e00 0f00 1000",
    "0af2 0900 1c0c 0000 0000 00", "11f2 0700 0100 c801 1c00 0000 0000 0000",
    "02e300", "029900", "05fe01020304",
]

STATUS_KEYS = {
    "battery_pct", "battery_health_pct", "rssi_dbm", "is_vibrating", "state", "lock",
    "charge_mode", "is_charging", "profile_downloaded_active", "heating_mode_downloaded",
    "uptime_s", "is_temperature_error", "is_in_use", "is_preheating", "shake",
    "auto_start_heating", "end_by_puff", "profile_source", "last_cigarette_allowed",
    "unacquired_logs", "heating_mode",
}


def clean(d):
    return {k: v for k, v in d.items() if k not in ("op", "raw", "t")}


def gen_parse():
    out = []
    for text in PARSE_FRAMES:
        raw = bytes.fromhex(text.replace(" ", ""))
        out.append({"hex": raw.hex(), "decoded": clean(ploom.parse(raw))})
    return out


IDLE = "10304314000000000000ff020801000100"  # live capture
SMOKING_30 = "1030 4314 0000 20 01 00 00 00 00 00 01 00 01 00"

MERGE_SCENARIOS = {
    "state change beats older idle status": ([(1000, IDLE), (2000, "0431040000")], 3000),
    "newer idle status beats older state change": ([(1000, "0431040000"), (2000, IDLE)], 3000),
    "preheating then smoking": ([(0, IDLE), (500, "0431020000"), (900, "0431040000")], 1000),
    "standby clears charging": ([(0, "0634000300 0000"), (100, "0431000000")], 200),
    "charging bit then charger removed": (
        [(0, "1030 0100 0000 10 01 00 00 00 00 00 01 00 01 00"), (50, "0634000700 0000")], 100),
    "charging bit cannot clear charging": ([(0, "0634000300 0000"), (10, IDLE)], 20),
    "window boundary is inclusive": ([(0, "023c50"), (5000, "02335f")], 15000),
    "expired frames are dropped": ([(0, "023c50"), (5000, "02335f")], 15001),
    "full poll burst, downloaded intense": (
        [(0, IDLE), (10, "02335f"), (20, "0c9f00 00000000000000000000"), (30, "033c5500"),
         (40, "02fcc4"), (50, "023d01"), (60, "083e000200000000 00")], 100),
    "factory profile active": ([(0, "023d00"), (10, "083e000200000000 00")], 20),
    "profile source fallback": ([(0, "10304314000000000000ff020801000000")], 10),
    "downloaded but unknown which": ([(0, IDLE)], 10),
    "multi-bit state": ([(0, IDLE), (10, "0431060000")], 20),
    "temperature error": ([(0, IDLE), (10, "0431100000")], 20),
    "lock notification after status": ([(0, IDLE), (10, "029f01")], 20),
    "configuration frame": ([(0, "0c18 08 11 03 00 00 00 00 01 01 00 00")], 10),
    "smoking status then standby change": ([(0, SMOKING_30), (10, "0431000000")], 20),
    "vibration and logs": ([(0, "027a00"), (10, "0535060007 00"), (20, "027a01")], 30),
    "unknown state keeps flags": ([(0, SMOKING_30), (10, "0431800000")], 20),
    "out of order arrival": ([(0, "023c40"), (2000, "023c41"), (1000, "023c42")], 2000),
}


def gen_merge():
    out = []
    for name, (frames, now_ms) in MERGE_SCENARIOS.items():
        parsed = []
        hexes = []
        for t_ms, text in frames:
            raw = bytes.fromhex(text.replace(" ", ""))
            f = ploom.parse(raw)
            f["t"] = t_ms / 1000
            parsed.append(f)
            hexes.append({"hex": raw.hex(), "tMs": t_ms})
        merged = ploom.merge_window(parsed, (now_ms - 15000) / 1000)
        out.append({"name": name, "frames": hexes, "nowMs": now_ms,
                    "merged": {k: v for k, v in merged.items() if k in STATUS_KEYS}})
    return out


def gen_leaflet():
    out = []
    for key in range(1100, 1351):
        m = [1] * 16
        m[7] = key
        out.append([key, ploom.leaflet_const(m)])
    return out


def write(name, data):
    path = HERE / name
    path.write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")
    print(f"wrote {path.name}: {len(data)} entries")


def main():
    rng = random.Random(0x9100)
    write("build_frames.json", gen_build_frames(rng))
    write("get_res.json", gen_get_res(rng))
    write("parse.json", gen_parse())
    write("merge_window.json", gen_merge())
    write("leaflet.json", gen_leaflet())


if __name__ == "__main__":
    main()
