#!/usr/bin/env python
"""Render a SKY130 SRAM22 GDS into the landing-page thumbnail.

The interactive viewers draw vector geometry (see export_geom.py); the only
raster asset the site consumes is the decorative homepage thumbnail. This
script renders the composited layers and writes that WebP directly to the exact
path the landing page loads.

Usage:
    python render_gds.py <macro.gds> <out.webp> [px_width]
e.g. python render_gds.py macro.gds ../public/layout/composite_preview.webp
"""
import sys, io, time
import gdstk
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.collections import PolyCollection
from PIL import Image

GDS = sys.argv[1]
OUT = sys.argv[2]  # exact output path, e.g. .../composite_preview.webp
PX_W = int(sys.argv[3]) if len(sys.argv) > 3 else 2200

# (layer, datatype) -> fill, composite_alpha, z  (draw order)
LAYERS = [
    ((64, 20), "#d8c8a0", 0.40, 1),
    ((65, 20), "#5aa14f", 0.70, 2),
    ((66, 20), "#c0392b", 0.72, 3),
    ((67, 20), "#8e7cc3", 0.45, 4),
    ((68, 20), "#274060", 0.45, 5),
    ((69, 20), "#65afff", 0.28, 6),
]

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
fig = plt.figure(figsize=(PX_W / dpi, PX_H / dpi), dpi=dpi)
ax = fig.add_axes([0, 0, 1, 1])
ax.set_xlim(0, W)
ax.set_ylim(0, H)
ax.set_axis_off()
ax.set_aspect("equal")
ax.add_patch(plt.Rectangle((0, 0), W, H, facecolor="#0d1526", edgecolor="none"))
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
