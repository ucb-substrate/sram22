#!/usr/bin/env python
"""Render a SKY130 SRAM22 GDS into a themed landing-page thumbnail.

The interactive viewers draw vector geometry (see export_geom.py). This script
renders the homepage thumbnails by compositing the layers into a WebP using
the selected theme.

Usage:
    python render_gds.py <macro.gds> <out.webp> [px_width] [--theme light|dark]
"""
import argparse
import sys, io, time
import gdstk
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.collections import PolyCollection
from PIL import Image

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("gds")
parser.add_argument("output")
parser.add_argument("px_width", nargs="?", type=int, default=2200)
parser.add_argument("--theme", choices=("light", "dark"), default="dark")
args = parser.parse_args()
GDS, OUT, PX_W = args.gds, args.output, args.px_width

# (layer, datatype) -> fill, composite_alpha, z  (draw order)
LAYER_PALETTES = {"dark": [
    ((64, 20), "#d8c8a0", 0.40, 1),
    ((65, 20), "#5aa14f", 0.70, 2),
    ((66, 20), "#c0392b", 0.72, 3),
    ((67, 20), "#8e7cc3", 0.45, 4),
    ((68, 20), "#274060", 0.45, 5),
    ((69, 20), "#65afff", 0.28, 6),
], "light": [
    ((64, 20), "#a58b42", 0.32, 1),
    ((65, 20), "#387c34", 0.60, 2),
    ((66, 20), "#aa392d", 0.68, 3),
    ((67, 20), "#7564a4", 0.40, 4),
    ((68, 20), "#63748b", 0.36, 5),
    ((69, 20), "#2455b8", 0.32, 6),
]}
LAYERS = LAYER_PALETTES[args.theme]
BACKGROUND = {"light": "#f8fafc", "dark": "#0d1526"}[args.theme]

t0 = time.time()
lib = gdstk.read_gds(GDS)
tops = lib.top_level()
if not tops:
    sys.exit(f"{GDS} has no top-level cell")
top = tops[0]
(x0, y0), (x1, y1) = top.bounding_box()
W, H = x1 - x0, y1 - y0
print(f"read {GDS} in {time.time()-t0:.1f}s  bbox={W:.2f}x{H:.2f}um")

want = {ld for ld, *_ in LAYERS}
buckets = {ld: [] for ld in want}
for p in top.get_polygons():
    ld = (p.layer, p.datatype)
    if ld in want:
        buckets[ld].append(p.points - [x0, y0])

PX_H = round(PX_W * H / W)
dpi = 100
fig = plt.figure(figsize=(PX_W / dpi, PX_H / dpi), dpi=dpi, facecolor=BACKGROUND)
ax = fig.add_axes([0, 0, 1, 1])
ax.set_xlim(0, W)
ax.set_ylim(0, H)
ax.set_axis_off()
ax.set_aspect("equal")
ax.add_patch(plt.Rectangle((0, 0), W, H, facecolor=BACKGROUND, edgecolor="none"))
for ld, fill, alpha, z in sorted(LAYERS, key=lambda r: r[-1]):
    polys = buckets[ld]
    if polys:
        ax.add_collection(
            PolyCollection(polys, facecolors=fill, edgecolors="none",
                           alpha=alpha, antialiaseds=True))

png = io.BytesIO()
fig.savefig(png, format="png", dpi=dpi)
plt.close(fig)
png.seek(0)
Image.open(png).convert("RGB").save(OUT, "WEBP", quality=80, method=6)
print(f"wrote {OUT}  {PX_W}x{PX_H}px in {time.time()-t0:.1f}s")
