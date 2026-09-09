/**
 * Vector layout renderer for SRAM22 macros.
 *
 * Draws the actual macro geometry (per-layer rectangles + polygons, decoded
 * from a compact binary) onto a canvas, redrawing on every zoom so it stays
 * crisp at any depth — no pre-rendered raster, no zoom cap. Pins are drawn as
 * vectors on top and are hover/click hit-tested for identification.
 *
 * World space: nanometres, y-up (GDS convention). Screen space: CSS px, y-down.
 */

export interface GeomLayer {
  key: string;
  rects: Int32Array; // [x,y,w,h] * n
  polyCounts: Uint16Array; // points per polygon
  polyPts: Int32Array; // [x,y] * totalPoints
  // spatial index (uniform grid) over rects + polys, built at load
  rectCells: Int32Array[]; // cell -> rect indices
  polyCells: Int32Array[]; // cell -> poly indices
  polyStart: Int32Array; // start offset (in points) of each polygon
  polyBBox: Int32Array; // [minx,miny,maxx,maxy] * nPolys
}

export interface Geom {
  wNm: number;
  hNm: number;
  layers: GeomLayer[];
  gx: number;
  gy: number;
  cellW: number;
  cellH: number;
}

export interface PinRect {
  x: number;
  y: number;
  w: number;
  h: number;
} // µm

export interface Pin {
  name: string;
  base: string;
  direction: string;
  layer: string;
  color: string;
  rects: [number, number, number, number][]; // µm
  // precomputed µm bbox
  bx: number;
  by: number;
  bw: number;
  bh: number;
  // Distributed power/ground net (a met2 strap grid, not a point). Drawn as its
  // real straps behind an opt-in toggle; never marker-boxed or click-selected.
  power?: boolean;
}

const GX = 64;
const GY = 34;

async function fetchDecoded(url: string): Promise<ArrayBuffer> {
  const resp = await fetch(url);
  const buf = await resp.arrayBuffer();
  const head = new Uint8Array(buf, 0, 2);
  if (head[0] === 0x1f && head[1] === 0x8b && "DecompressionStream" in window) {
    const stream = new Blob([buf])
      .stream()
      .pipeThrough(new (window as any).DecompressionStream("gzip"));
    return await new Response(stream).arrayBuffer();
  }
  return buf;
}

export async function loadGeom(url: string): Promise<Geom> {
  const buf = await fetchDecoded(url);
  const dv = new DataView(buf);
  let o = 0;
  const u32 = () => {
    const v = dv.getUint32(o, true);
    o += 4;
    return v;
  };
  const i32 = () => {
    const v = dv.getInt32(o, true);
    o += 4;
    return v;
  };
  u32(); // magic
  u32(); // version
  const wNm = i32();
  const hNm = i32();
  const numLayers = dv.getUint16(o, true);
  o += 2;

  const cellW = Math.ceil(wNm / GX);
  const cellH = Math.ceil(hNm / GY);
  const layers: GeomLayer[] = [];

  for (let L = 0; L < numLayers; L++) {
    const keyLen = dv.getUint8(o);
    o += 1;
    const key = new TextDecoder().decode(new Uint8Array(buf, o, keyLen));
    o += keyLen;
    const rectCount = u32();
    const ptCount = u32();
    const polyCount = u32();

    const rects = new Int32Array(buf.slice(o, o + rectCount * 16));
    o += rectCount * 16;
    const polyCounts = new Uint16Array(buf.slice(o, o + polyCount * 2));
    o += polyCount * 2;
    const polyPts = new Int32Array(buf.slice(o, o + ptCount * 8));
    o += ptCount * 8;

    // per-poly start + bbox
    const polyStart = new Int32Array(polyCount);
    const polyBBox = new Int32Array(polyCount * 4);
    let acc = 0;
    for (let p = 0; p < polyCount; p++) {
      polyStart[p] = acc;
      const n = polyCounts[p];
      let minx = Infinity,
        miny = Infinity,
        maxx = -Infinity,
        maxy = -Infinity;
      for (let k = 0; k < n; k++) {
        const x = polyPts[(acc + k) * 2];
        const y = polyPts[(acc + k) * 2 + 1];
        if (x < minx) minx = x;
        if (x > maxx) maxx = x;
        if (y < miny) miny = y;
        if (y > maxy) maxy = y;
      }
      polyBBox[p * 4] = minx;
      polyBBox[p * 4 + 1] = miny;
      polyBBox[p * 4 + 2] = maxx;
      polyBBox[p * 4 + 3] = maxy;
      acc += n;
    }

    // uniform-grid spatial index
    const cellOf = (cx: number, cy: number) => cy * GX + cx;
    const clampX = (c: number) => Math.max(0, Math.min(GX - 1, c));
    const clampY = (c: number) => Math.max(0, Math.min(GY - 1, c));
    const rectBuckets: number[][] = Array.from({ length: GX * GY }, () => []);
    for (let i = 0; i < rectCount; i++) {
      const x = rects[i * 4],
        y = rects[i * 4 + 1],
        w = rects[i * 4 + 2],
        h = rects[i * 4 + 3];
      const cx0 = clampX(Math.floor(x / cellW));
      const cx1 = clampX(Math.floor((x + w) / cellW));
      const cy0 = clampY(Math.floor(y / cellH));
      const cy1 = clampY(Math.floor((y + h) / cellH));
      for (let cy = cy0; cy <= cy1; cy++)
        for (let cx = cx0; cx <= cx1; cx++) rectBuckets[cellOf(cx, cy)].push(i);
    }
    const polyBuckets: number[][] = Array.from({ length: GX * GY }, () => []);
    for (let p = 0; p < polyCount; p++) {
      const cx0 = clampX(Math.floor(polyBBox[p * 4] / cellW));
      const cx1 = clampX(Math.floor(polyBBox[p * 4 + 2] / cellW));
      const cy0 = clampY(Math.floor(polyBBox[p * 4 + 1] / cellH));
      const cy1 = clampY(Math.floor(polyBBox[p * 4 + 3] / cellH));
      for (let cy = cy0; cy <= cy1; cy++)
        for (let cx = cx0; cx <= cx1; cx++) polyBuckets[cellOf(cx, cy)].push(p);
    }

    layers.push({
      key,
      rects,
      polyCounts,
      polyPts,
      polyStart,
      polyBBox,
      rectCells: rectBuckets.map((a) => Int32Array.from(a)),
      polyCells: polyBuckets.map((a) => Int32Array.from(a)),
    });
  }

  return { wNm, hNm, layers, gx: GX, gy: GY, cellW, cellH };
}

// per-layer draw style
interface LayerStyle {
  fill: string;
  alpha: number;
}

export interface ViewerOpts {
  canvas: HTMLCanvasElement;
  geom: Geom;
  pins: Pin[];
  styles: Record<string, LayerStyle>;
  bg: string;
  hidden?: string[]; // layer keys hidden initially
  onSelect?: (pin: Pin | null) => void;
  onHover?: (pin: Pin | null) => void;
}

export interface Viewer {
  reset: () => void;
  selectPin: (name: string | null, focus?: boolean) => void;
  highlight: (names: string[] | null, focus?: boolean) => void;
  setLayerVisible: (key: string, on: boolean) => void;
  setPowerVisible: (on: boolean) => void;
  destroy: () => void;
}

export function createViewer(opts: ViewerOpts): Viewer {
  const { canvas, geom, pins, styles, bg } = opts;
  const ctx = canvas.getContext("2d")!;
  const S = 1000; // µm -> nm

  let s = 1; // css px per nm
  let ox = 0;
  let oy = 0; // screen px of world (0, hNm) i.e. top-left
  let dpr = Math.max(1, window.devicePixelRatio || 1);
  let cssW = 0,
    cssH = 0;
  const visible: Record<string, boolean> = {};
  geom.layers.forEach((l) => (visible[l.key] = true));
  (opts.hidden || []).forEach((k) => (visible[k] = false));

  let selected: Pin | null = null;
  let hovered: Pin | null = null;
  let highlightSet: Set<string> | null = null;
  let powerVisible = false; // overlay vdd/vss strap grid (opt-in)

  // Pre-rendered "overview" bitmap of the full geometry. When zoomed out we
  // blit this in a single drawImage instead of re-pathing ~270k shapes; when
  // zoomed in past its resolution we switch to (culled, cheap) vector drawing.
  const OVER_W = 5000;
  let overCanvas: HTMLCanvasElement | null = null;
  let overScale = Infinity; // px per nm of the overview bitmap
  let overDirty = true; // rebuild when the visible-layer set changes

  // world(x,y nm) -> screen css px
  const sx = (x: number) => ox + x * s;
  const syTop = (y: number, h: number) => oy + (geom.hNm - (y + h)) * s;

  function resize() {
    dpr = Math.max(1, window.devicePixelRatio || 1);
    cssW = canvas.clientWidth;
    cssH = canvas.clientHeight;
    canvas.width = Math.round(cssW * dpr);
    canvas.height = Math.round(cssH * dpr);
  }

  function fitScale() {
    return Math.min(cssW / geom.wNm, cssH / geom.hNm) * 0.97;
  }

  function clampScale() {
    const minS = fitScale() * 0.6;
    const maxS = 0.25; // 250 px per µm — very deep, still vector-crisp
    s = Math.max(minS, Math.min(maxS, s));
  }

  // Keep the macro from being panned entirely off-screen. Centers a dimension
  // when the macro is smaller than the viewport, otherwise clamps so at least
  // `m` px of it stays visible on each edge.
  function clampView() {
    const w = geom.wNm * s;
    const h = geom.hNm * s;
    const m = 48;
    ox = w <= cssW ? (cssW - w) / 2 : Math.min(cssW - m, Math.max(m - w, ox));
    oy = h <= cssH ? (cssH - h) / 2 : Math.min(cssH - m, Math.max(m - h, oy));
  }

  function reset() {
    resize();
    s = fitScale();
    ox = (cssW - geom.wNm * s) / 2;
    oy = (cssH - geom.hNm * s) / 2;
    schedule();
  }

  let raf = 0;
  function schedule() {
    if (raf) return;
    raf = requestAnimationFrame(() => {
      raf = 0;
      draw();
    });
  }

  function visibleCellRange() {
    // world x visible: screenX in [0,cssW] -> x in [(0-ox)/s,(cssW-ox)/s]
    const wx0 = (0 - ox) / s;
    const wx1 = (cssW - ox) / s;
    // screenY in [0,cssH]; screenY = oy + (hNm - y)*s -> y = hNm - (screenY-oy)/s
    const wy0 = geom.hNm - (cssH - oy) / s;
    const wy1 = geom.hNm - (0 - oy) / s;
    const cx0 = Math.max(0, Math.floor(wx0 / geom.cellW));
    const cx1 = Math.min(GX - 1, Math.floor(wx1 / geom.cellW));
    const cy0 = Math.max(0, Math.floor(wy0 / geom.cellH));
    const cy1 = Math.min(GY - 1, Math.floor(wy1 / geom.cellH));
    return { cx0, cx1, cy0, cy1 };
  }

  // Paint one layer's shapes into a target context.
  //   screenX = oX + x*sc ; screenYtop = oY + (hNm - (y+h))*sc   (y-flip)
  // `cull` limits to a visible cell range (zoomed-in vector path); when null,
  // every shape is painted (used to build the overview bitmap).
  function paintLayer(
    c: CanvasRenderingContext2D,
    layer: GeomLayer,
    oX: number,
    oY: number,
    sc: number,
    cull: { cx0: number; cx1: number; cy0: number; cy1: number } | null,
  ) {
    const st = styles[layer.key];
    if (!st) return;
    const hNm = geom.hNm;
    const R = layer.rects;
    c.globalAlpha = st.alpha;
    c.fillStyle = st.fill;

    c.beginPath();
    if (cull) {
      for (let cy = cull.cy0; cy <= cull.cy1; cy++)
        for (let cx = cull.cx0; cx <= cull.cx1; cx++) {
          const bucket = layer.rectCells[cy * GX + cx];
          for (let bi = 0; bi < bucket.length; bi++) {
            const i = bucket[bi];
            c.rect(
              oX + R[i * 4] * sc,
              oY + (hNm - (R[i * 4 + 1] + R[i * 4 + 3])) * sc,
              R[i * 4 + 2] * sc,
              R[i * 4 + 3] * sc,
            );
          }
        }
    } else {
      const n = R.length / 4;
      for (let i = 0; i < n; i++)
        c.rect(
          oX + R[i * 4] * sc,
          oY + (hNm - (R[i * 4 + 1] + R[i * 4 + 3])) * sc,
          R[i * 4 + 2] * sc,
          R[i * 4 + 3] * sc,
        );
    }
    c.fill();

    if (layer.polyCounts.length) {
      const P = layer.polyPts;
      c.beginPath();
      const paintPoly = (p: number) => {
        const start = layer.polyStart[p];
        const np = layer.polyCounts[p];
        for (let k = 0; k < np; k++) {
          const X = oX + P[(start + k) * 2] * sc;
          const Y = oY + (hNm - P[(start + k) * 2 + 1]) * sc;
          if (k === 0) c.moveTo(X, Y);
          else c.lineTo(X, Y);
        }
        c.closePath();
      };
      if (cull) {
        for (let cy = cull.cy0; cy <= cull.cy1; cy++)
          for (let cx = cull.cx0; cx <= cull.cx1; cx++) {
            const bucket = layer.polyCells[cy * GX + cx];
            for (let bi = 0; bi < bucket.length; bi++) paintPoly(bucket[bi]);
          }
      } else {
        for (let p = 0; p < layer.polyCounts.length; p++) paintPoly(p);
      }
      c.fill();
    }
    c.globalAlpha = 1;
  }

  // Render the full geometry into the overview bitmap (once, or on toggle).
  function buildOverview() {
    overScale = OVER_W / geom.wNm;
    const w = OVER_W;
    const h = Math.round(geom.hNm * overScale);
    if (!overCanvas) overCanvas = document.createElement("canvas");
    overCanvas.width = w;
    overCanvas.height = h;
    const octx = overCanvas.getContext("2d")!;
    octx.setTransform(1, 0, 0, 1, 0, 0);
    octx.fillStyle = bg;
    octx.fillRect(0, 0, w, h);
    for (const layer of geom.layers)
      if (visible[layer.key]) paintLayer(octx, layer, 0, 0, overScale, null);
    overDirty = false;
  }

  function draw() {
    if (overDirty) buildOverview();
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, cssW, cssH);
    ctx.fillStyle = bg;
    ctx.fillRect(0, 0, cssW, cssH);

    if (s <= overScale && overCanvas) {
      // Zoomed out: one downscaled blit of the pre-rendered overview.
      ctx.imageSmoothingEnabled = true;
      ctx.imageSmoothingQuality = "high";
      ctx.drawImage(overCanvas, ox, oy, geom.wNm * s, geom.hNm * s);
    } else {
      // Zoomed in past the overview resolution: crisp vectors, culled to the
      // visible cells (few shapes are in view here, so this is cheap).
      const cull = visibleCellRange();
      for (const layer of geom.layers)
        if (visible[layer.key]) paintLayer(ctx, layer, ox, oy, s, cull);
    }

    drawPins();
  }

  function pinScreenBox(pin: Pin) {
    // union of the pin's rects in screen space
    let l = Infinity,
      t = Infinity,
      r = -Infinity,
      b = -Infinity;
    for (const [x, y, w, h] of pin.rects) {
      const X = sx(x * S),
        Y = syTop(y * S, h * S);
      l = Math.min(l, X);
      t = Math.min(t, Y);
      r = Math.max(r, X + w * S * s);
      b = Math.max(b, Y + h * S * s);
    }
    return { l, t, r, b };
  }

  // Draw one power net as its real met2 straps (with a 1px floor so the grid
  // stays visible when zoomed out), rather than a whole-macro union box.
  function drawPowerNet(pin: Pin) {
    ctx.globalAlpha = 0.5;
    ctx.fillStyle = pin.color;
    ctx.beginPath();
    for (const [x, y, w, h] of pin.rects) {
      let rx = sx(x * S);
      let ry = syTop(y * S, h * S);
      let rw = w * S * s;
      let rh = h * S * s;
      if (rw < 1) {
        rx -= (1 - rw) / 2;
        rw = 1;
      }
      if (rh < 1) {
        ry -= (1 - rh) / 2;
        rh = 1;
      }
      if (rx + rw < 0 || rx > cssW || ry + rh < 0 || ry > cssH) continue;
      ctx.rect(rx, ry, rw, rh);
    }
    ctx.fill();
    ctx.globalAlpha = 1;
  }

  function drawPins() {
    // Power/ground grid (context) beneath the signal markers, only when enabled.
    if (powerVisible) {
      for (const pin of pins) if (pin.power) drawPowerNet(pin);
    }

    for (const pin of pins) {
      if (pin.power) continue; // power is a grid overlay, not a point marker
      const isSel = pin === selected;
      const isHov = pin === hovered;
      const isHi = isSel || (highlightSet !== null && highlightSet.has(pin.name));
      let { l, t, r, b } = pinScreenBox(pin);
      // minimum marker size so tiny pins remain visible/clickable
      const minPx = isSel ? 12 : isHi ? 9 : 7;
      if (r - l < minPx) {
        const cx = (l + r) / 2;
        l = cx - minPx / 2;
        r = cx + minPx / 2;
      }
      if (b - t < minPx) {
        const cy = (t + b) / 2;
        t = cy - minPx / 2;
        b = cy + minPx / 2;
      }
      if (r < 0 || l > cssW || b < 0 || t > cssH) continue;

      ctx.lineWidth = isSel ? 2.5 : isHov || isHi ? 2 : 1.2;
      ctx.strokeStyle = isSel ? "#ffffff" : pin.color;
      ctx.fillStyle = pin.color;
      ctx.globalAlpha = isSel ? 0.55 : isHov ? 0.45 : isHi ? 0.4 : 0.22;
      ctx.fillRect(l, t, r - l, b - t);
      ctx.globalAlpha = 1;
      ctx.strokeRect(l, t, r - l, b - t);
    }
    // label the selected pin last (on top)
    if (selected && !selected.power) {
      const { l, t, r } = pinScreenBox(selected);
      const cx = Math.max(0, Math.min(cssW, (l + r) / 2));
      const label = selected.name;
      ctx.font =
        "600 12px 'JetBrains Mono Variable', ui-monospace, monospace";
      const tw = ctx.measureText(label).width;
      let lx = cx - tw / 2 - 6;
      let ly = Math.max(0, t) - 24;
      lx = Math.max(2, Math.min(cssW - tw - 14, lx));
      ly = Math.max(2, ly);
      ctx.fillStyle = "rgba(8,14,26,0.92)";
      ctx.strokeStyle = selected.color;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.roundRect(lx, ly, tw + 12, 19, 4);
      ctx.fill();
      ctx.stroke();
      ctx.fillStyle = "#eaf2ff";
      ctx.textBaseline = "middle";
      ctx.fillText(label, lx + 6, ly + 10);
    }
  }

  // ---- hit testing ----
  function pinAt(px: number, py: number): Pin | null {
    let best: Pin | null = null;
    let bestD = Infinity;
    const tol = 6;
    for (const pin of pins) {
      if (pin.power) continue; // power grid isn't click-selectable
      let { l, t, r, b } = pinScreenBox(pin);
      const minPx = 8;
      if (r - l < minPx) {
        const c = (l + r) / 2;
        l = c - minPx / 2;
        r = c + minPx / 2;
      }
      if (b - t < minPx) {
        const c = (t + b) / 2;
        t = c - minPx / 2;
        b = c + minPx / 2;
      }
      if (px >= l - tol && px <= r + tol && py >= t - tol && py <= b + tol) {
        const dx = px - (l + r) / 2;
        const dy = py - (t + b) / 2;
        const d = dx * dx + dy * dy;
        if (d < bestD) {
          bestD = d;
          best = pin;
        }
      }
    }
    return best;
  }

  // ---- interaction ----
  let dragging = false;
  let moved = false;
  let lastX = 0,
    lastY = 0;

  const onWheel = (e: WheelEvent) => {
    if (e.deltaY === 0) return; // horizontal scroll: don't zoom
    e.preventDefault();
    const rect = canvas.getBoundingClientRect();
    const cx = e.clientX - rect.left;
    const cy = e.clientY - rect.top;
    const f = e.deltaY < 0 ? 1.2 : 1 / 1.2;
    const prev = s;
    s *= f;
    clampScale();
    const real = s / prev;
    ox = cx - (cx - ox) * real;
    oy = cy - (cy - oy) * real;
    clampView();
    schedule();
  };

  const onDown = (e: PointerEvent) => {
    dragging = true;
    moved = false;
    lastX = e.clientX;
    lastY = e.clientY;
    canvas.setPointerCapture(e.pointerId);
    canvas.classList.add("is-grabbing");
  };
  const onMove = (e: PointerEvent) => {
    const rect = canvas.getBoundingClientRect();
    if (dragging) {
      const dx = e.clientX - lastX;
      const dy = e.clientY - lastY;
      if (Math.abs(dx) + Math.abs(dy) > 2) moved = true;
      ox += dx;
      oy += dy;
      clampView();
      lastX = e.clientX;
      lastY = e.clientY;
      schedule();
    } else {
      const h = pinAt(e.clientX - rect.left, e.clientY - rect.top);
      if (h !== hovered) {
        hovered = h;
        canvas.style.cursor = h ? "pointer" : "grab";
        opts.onHover?.(h);
        schedule();
      }
    }
  };
  const onUp = (e: PointerEvent) => {
    const wasDrag = dragging && moved;
    dragging = false;
    canvas.classList.remove("is-grabbing");
    try {
      canvas.releasePointerCapture(e.pointerId);
    } catch {}
    if (!wasDrag) {
      const rect = canvas.getBoundingClientRect();
      const hit = pinAt(e.clientX - rect.left, e.clientY - rect.top);
      selectPin(hit ? hit.name : null, false);
    }
  };

  // Zoom/pan so a µm bounding box fills `frac` of the smaller viewport dim.
  function focusBBoxUm(
    bx: number,
    by: number,
    bw: number,
    bh: number,
    frac = 0.35,
  ) {
    const cxNm = (bx + bw / 2) * S;
    const cyNm = (by + bh / 2) * S;
    const spanNm = Math.max(bw, bh, 0.4) * S;
    s = (Math.min(cssW, cssH) * frac) / spanNm;
    clampScale();
    ox = cssW / 2 - cxNm * s;
    oy = cssH / 2 - (geom.hNm - cyNm) * s;
    clampView();
    schedule();
  }

  function groupBBox(names: Set<string>) {
    let x0 = Infinity,
      y0 = Infinity,
      x1 = -Infinity,
      y1 = -Infinity;
    for (const p of pins) {
      if (!names.has(p.name)) continue;
      x0 = Math.min(x0, p.bx);
      y0 = Math.min(y0, p.by);
      x1 = Math.max(x1, p.bx + p.bw);
      y1 = Math.max(y1, p.by + p.bh);
    }
    return { x0, y0, x1, y1 };
  }

  function selectPin(name: string | null, focus = true) {
    selected = name ? pins.find((p) => p.name === name) || null : null;
    highlightSet = null;
    if (selected && focus)
      focusBBoxUm(selected.bx, selected.by, selected.bw, selected.bh, 0.2);
    else schedule();
    opts.onSelect?.(selected);
  }

  function highlight(names: string[] | null, focus = true) {
    selected = null;
    highlightSet = names && names.length ? new Set(names) : null;
    if (highlightSet && focus) {
      const b = groupBBox(highlightSet);
      focusBBoxUm(b.x0, b.y0, b.x1 - b.x0, b.y1 - b.y0, 0.6);
    } else schedule();
    opts.onSelect?.(null);
  }

  const onResize = () => reset();

  canvas.addEventListener("wheel", onWheel, { passive: false });
  canvas.addEventListener("pointerdown", onDown);
  canvas.addEventListener("pointermove", onMove);
  canvas.addEventListener("pointerup", onUp);
  canvas.addEventListener("pointercancel", onUp);
  window.addEventListener("resize", onResize);

  buildOverview(); // pre-render the overview bitmap once, up front
  reset();

  return {
    reset,
    selectPin,
    highlight,
    setLayerVisible: (key, on) => {
      visible[key] = on;
      overDirty = true;
      schedule();
    },
    setPowerVisible: (on) => {
      powerVisible = on;
      schedule();
    },
    destroy: () => {
      canvas.removeEventListener("wheel", onWheel);
      canvas.removeEventListener("pointerdown", onDown);
      canvas.removeEventListener("pointermove", onMove);
      canvas.removeEventListener("pointerup", onUp);
      canvas.removeEventListener("pointercancel", onUp);
      window.removeEventListener("resize", onResize);
      if (raf) cancelAnimationFrame(raf);
    },
  };
}
