#!/usr/bin/env python
"""Flatten a SKY130 GDS into a compact binary of per-layer geometry for the
browser canvas renderer.

Binary layout (little-endian):
  magic  u32  'SRM1'
  version u32
  bboxW_nm i32, bboxH_nm i32      (origin shifted to 0,0)
  numLayers u16
  per layer:
    keyLen u8, key bytes
    rectCount u32
    polyPtCount u32   (total number of (x,y) points across this layer's polys)
    polyCount  u32
    rects:  rectCount * (x,y,w,h) i32   nm
    polys:  polyCount * (nPoints u16) then polyPtCount * (x,y) i32
The rects/polys sections are contiguous so the client can make typed-array views.
"""
import sys, struct, gzip, io, collections
import gdstk

GDS = sys.argv[1]
# Exact output path the site fetches, e.g. public/layout/layout-geom.bin. The
# bytes are gzip-compressed; the .bin name is intentional (the client detects
# the gzip magic and inflates via DecompressionStream).
OUT = sys.argv[2]

# (layer,datatype) -> key, in draw order (bottom first)
LAYERS = [
    ((64, 20), "nwell"),
    ((65, 20), "diff"),
    ((66, 20), "poly"),
    ((67, 20), "li1"),
    ((68, 20), "met1"),
    ((69, 20), "met2"),
]
want = {ld: key for ld, key in LAYERS}

lib = gdstk.read_gds(GDS)
top = lib.top_level()[0]
(x0, y0), (x1, y1) = top.bounding_box()
S = 1000.0  # µm -> nm
W = round((x1 - x0) * S)
H = round((y1 - y0) * S)

buckets = {key: {"rects": [], "polys": []} for _, key in LAYERS}

def is_rect(pts):
    if len(pts) != 4:
        return None
    xs = sorted(set(round(p[0]) for p in (pts * S)))
    ys = sorted(set(round(p[1]) for p in (pts * S)))
    if len(xs) == 2 and len(ys) == 2:
        return xs[0], ys[0], xs[1] - xs[0], ys[1] - ys[0]
    return None

nrect = npoly = 0
for p in top.get_polygons():
    ld = (p.layer, p.datatype)
    key = want.get(ld)
    if key is None:
        continue
    pts = p.points - [x0, y0]
    r = is_rect(pts)
    if r:
        buckets[key]["rects"].append(r)
        nrect += 1
    else:
        ip = [(round(px * S), round(py * S)) for px, py in pts]
        buckets[key]["polys"].append(ip)
        npoly += 1

print(f"bbox {W}x{H} nm; {nrect} rects, {npoly} polys")

buf = io.BytesIO()
buf.write(struct.pack("<II", 0x314D5253, 1))   # 'SRM1', v1
buf.write(struct.pack("<ii", W, H))
buf.write(struct.pack("<H", len(LAYERS)))
for _, key in LAYERS:
    b = buckets[key]
    rects = b["rects"]
    polys = b["polys"]
    ptcount = sum(len(pl) for pl in polys)
    kb = key.encode()
    buf.write(struct.pack("<B", len(kb)))
    buf.write(kb)
    buf.write(struct.pack("<III", len(rects), ptcount, len(polys)))
    # rects
    ra = bytearray()
    for (x, y, w, h) in rects:
        ra += struct.pack("<iiii", x, y, w, h)
    buf.write(ra)
    # polys: counts then points
    for pl in polys:
        buf.write(struct.pack("<H", len(pl)))
    pa = bytearray()
    for pl in polys:
        for (x, y) in pl:
            pa += struct.pack("<ii", x, y)
    buf.write(pa)
    print(f"  {key:6} rects={len(rects):7d} polys={len(polys):5d}")

raw = buf.getvalue()
gz = gzip.compress(raw, 9)
with open(OUT, "wb") as fh:
    fh.write(gz)
print(f"raw {len(raw)//1024} KB  -> gz {len(gz)//1024} KB  ({OUT})")
