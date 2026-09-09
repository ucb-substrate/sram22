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

# ---------------- LIB: representative timing ----------------
txt = open(LIB).read()

def first_values(after_idx):
    """Return floats of the first `values(` row after position after_idx."""
    if after_idx < 0:
        return []
    vi = txt.find("values", after_idx)
    if vi < 0:
        return []
    # numbers are inside the first quoted string after values(
    q = txt.find('"', vi)
    q2 = txt.find('"', q + 1)
    if q < 0 or q2 < 0:
        return []
    return [float(x) for x in re.findall(r"[-\d.]+", txt[q:q2])]

def find_constraint(pin, ttype):
    """Find a setup_rising/hold_rising value for a given pin/bus base name.

    The arcs live under `pin (<name>)` (scalars) or `pin (<name>[0])` (buses).
    Search from that declaration to the *next* pin/bus declaration so large
    buses (e.g. din[31:0]) are fully covered.
    """
    decl = re.search(rf"\bpin \({re.escape(pin)}(?:\[0\])?\)", txt) \
        or re.search(rf"\b(?:bus|pin) \({re.escape(pin)}(?:\[\d+\])?\)", txt)
    if not decl:
        return None
    nxt = re.search(r"\n\s+(?:bus|pin) \(", txt[decl.end():])
    end = decl.end() + (nxt.start() if nxt else 14000)
    seg = txt[decl.start():end]
    ti = seg.find(f"timing_type : {ttype}")
    if ti < 0:
        return None
    vals = first_values(decl.start() + ti)
    return vals

def arc_repr(vals):
    return round(sum(vals) / len(vals), 4) if vals else None

timing = {"corner": "tt_025C_1v80", "corner_label": "TT, 1.8 V, 25 °C",
          "setup": {}, "hold": {}}
for pin in ["addr", "din", "we", "ce", "wmask", "rstb"]:
    su = find_constraint(pin, "setup_rising")
    ho = find_constraint(pin, "hold_rising")
    if su:
        timing["setup"][pin] = {"repr": arc_repr(su), "min": round(min(su), 4), "max": round(max(su), 4)}
    if ho:
        timing["hold"][pin] = {"repr": arc_repr(ho), "min": round(min(ho), 4), "max": round(max(ho), 4)}

# min pulse width / min period (clk pin)
clk = re.search(r"pin \(clk\)", txt)
if not clk:
    sys.exit("could not find 'pin (clk)' in the .lib")
seg = txt[clk.start():clk.start() + 4000]
mpw_i = seg.find("min_pulse_width")
per_i = seg.find("minimum_period")
mpw = first_values(clk.start() + mpw_i) if mpw_i >= 0 else []
per = first_values(clk.start() + per_i) if per_i >= 0 else []
if not mpw or not per:
    sys.exit("could not find min_pulse_width / minimum_period for clk in the .lib")
timing["min_pulse_width_high"] = round(min(mpw), 4)
timing["minimum_period"] = round(sum(per) / len(per), 4)
timing["fmax_mhz"] = round(1000.0 / (sum(per) / len(per)), 1)

# clk -> q (dout cell_rise): find first rising_edge cell_rise table, take min & max
re_i = txt.find("timing_type : rising_edge")
cr_i = txt.find("cell_rise", re_i) if re_i >= 0 else -1
nums = []
if cr_i >= 0:
    end = txt.find("rise_transition", cr_i)
    blk = txt[cr_i:end if end >= 0 else cr_i + 4000]
    nums = [float(x) for x in re.findall(r"[\d.]+", blk) if "." in x and float(x) > 1]
if not nums:
    sys.exit("could not extract clk->Q (rising_edge cell_rise) from the .lib")
timing["clk_q"] = {"min": round(min(nums), 4), "max": round(max(nums), 4)}

with open(f"{OUT}/timing.json", "w") as fh:
    json.dump(timing, fh, indent=2)
print("timing:", json.dumps(timing))
