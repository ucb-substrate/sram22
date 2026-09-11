#!/usr/bin/env python
"""Extract pin geometry (from LEF) and timing (from .lib) for one SRAM22 macro
into JSON for the website. All distances in microns."""
import sys, json, re

MACRO = sys.argv[1]            # e.g. sram22_64x32m4w8
LEF = sys.argv[2]
LIB = sys.argv[3]
OUT = sys.argv[4]              # output dir (write pins.json + timing.json here)

# --- parse macro params from name: sram22_<words>x<width>m<mux>w<wsize> ---
m = re.match(r"sram22_(\d+)x(\d+)m(\d+)w(\d+)", MACRO)
if not m:
    sys.exit(f"MACRO name '{MACRO}' must look like sram22_<words>x<width>m<mux>w<wsize>")
words, dwidth, mux, wsize = (int(x) for x in m.groups())

# ---------------- LEF: macro size + pin geometry ----------------
size_w = size_h = None
pins = []            # individual pins with geometry (for the widget)
cur = None
layer = None
with open(LEF) as fh:
    for line in fh:
        s = line.strip()
        if s.startswith("SIZE"):
            mm = re.match(r"SIZE\s+([\d.]+)\s+BY\s+([\d.]+)", s)
            if mm:
                size_w, size_h = float(mm.group(1)), float(mm.group(2))
        elif s.startswith("PIN "):
            cur = {"name": s.split()[1], "direction": None, "use": None,
                   "layer": None, "rects": []}
        elif cur is not None and s.startswith("DIRECTION"):
            cur["direction"] = s.split()[1].lower()
        elif cur is not None and s.startswith("USE"):
            cur["use"] = s.split()[1].lower()
        elif cur is not None and s.startswith("LAYER"):
            layer = s.split()[1]
            if cur["layer"] is None:
                cur["layer"] = layer
        elif cur is not None and s.startswith("RECT"):
            x1, y1, x2, y2 = (float(v) for v in s.split()[1:5])
            cur["rects"].append([round(min(x1, x2), 4), round(min(y1, y2), 4),
                                 round(abs(x2 - x1), 4), round(abs(y2 - y1), 4)])
        elif cur is not None and s.startswith("END") and len(s.split()) > 1 \
                and s.split()[1] == cur["name"]:
            pins.append(cur)
            cur = None

# group base name + bus index
def split_name(n):
    mm = re.match(r"([A-Za-z_]+)\[(\d+)\]", n)
    if mm:
        return mm.group(1), int(mm.group(2))
    return n, None

for p in pins:
    base, idx = split_name(p["name"])
    p["base"], p["index"] = base, idx

# grouped summary for the pin-list table
order = ["clk", "rstb", "ce", "we", "wmask", "addr", "din", "dout", "vdd", "vss"]
groups = {}
for p in pins:
    g = groups.setdefault(p["base"], {"base": p["base"], "direction": p["direction"],
                                      "layer": p["layer"], "width": 0})
    g["width"] += 1
groups = [groups[b] for b in order if b in groups]

geom = {
    "macro": MACRO, "num_words": words, "data_width": dwidth,
    "mux_ratio": mux, "write_size": wsize,
    "addr_width": (words - 1).bit_length(), "wmask_width": dwidth // wsize,
    "um_width": size_w, "um_height": size_h,
    "pins": [{"name": p["name"], "base": p["base"], "index": p["index"],
              "direction": p["direction"], "layer": p["layer"],
              "use": p["use"], "rects": p["rects"]} for p in pins],
    "groups": groups,
}
# The site imports src/data/pins.json, so write that exact name (run with
# OUT pointed at src/data to refresh in place).
with open(f"{OUT}/pins.json", "w") as fh:
    json.dump(geom, fh, separators=(",", ":"))
print(f"pins: {len(pins)} individual, groups={[ (g['base'],g['width'],g['direction'],g['layer']) for g in groups ]}")
print(f"size: {size_w} x {size_h} um")

# ---------------- LIB: full rise/fall table ranges ----------------
from liberty_data import extract_timing
import hashlib

timing = extract_timing(open(LIB).read(), MACRO)
timing["source_sha256"] = hashlib.sha256(open(LIB, "rb").read()).hexdigest()
with open(f"{OUT}/timing.json", "w") as fh:
    json.dump(timing, fh, indent=2, ensure_ascii=False)
    fh.write("\n")
print("timing: complete rise/fall tables extracted for", MACRO)
