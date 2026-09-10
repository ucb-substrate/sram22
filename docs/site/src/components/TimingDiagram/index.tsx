import React, { type JSX } from "react";
import Link from "@docusaurus/Link";
import timing from "@site/src/data/timing.json";
import styles from "./styles.module.css";

// One explicit table point, not averages or guaranteed interface limits.
const fmt = (v: number) => (v < 1 ? v.toFixed(3) : v.toFixed(2));
const tSU = fmt(timing.setup.addr.rise.example);
const tH = fmt(timing.hold.addr.rise.example);
const tCQ = fmt(timing.clk_q.rise.example);
const corner = timing.corner_label;

// ---- geometry (schematic; not to time-scale) ----
const W = 820;
const L = 92; // label gutter
const x0 = L + 8;
const xEnd = 792;
const rowH = 34;
const gap = 14;
const top0 = 24;

const rows = ["clk", "ce", "we", "addr", "din", "dout"];
const rowTop: Record<string, number> = {};
rows.forEach((r, i) => (rowTop[r] = top0 + i * (rowH + gap)));
const H = top0 + rows.length * (rowH + gap) + 30;

// rising clock edges
const E0 = 250; // READ launch
const E1 = 520; // WRITE launch
const hi = (t: string) => rowTop[t] + 6;
const lo = (t: string) => rowTop[t] + rowH - 6;
const mid = (t: string) => rowTop[t] + rowH / 2;

// 1-bit signal path; pts = [{x, v}] level v∈{0,1} starting at x.
function bit(row: string, pts: { x: number; v: 0 | 1 }[]) {
  const y = (v: 0 | 1) => (v ? hi(row) : lo(row));
  let d = `M ${pts[0].x} ${y(pts[0].v)}`;
  for (let i = 1; i < pts.length; i++) {
    d += ` H ${pts[i].x} V ${y(pts[i].v)}`;
  }
  d += ` H ${xEnd}`;
  return d;
}

// bus hexagon segments
const cw = 7;
function busPolys(row: string, segs: { x0: number; x1: number }[]) {
  return segs.map((s) => {
    const h = hi(row),
      l = lo(row),
      m = mid(row);
    // order: top-left -> top-right -> right -> bottom-right -> bottom-left -> left
    const tl = s.x0 <= x0 ? `${s.x0},${h}` : `${s.x0 + cw},${h}`;
    const tr = s.x1 >= xEnd ? `${s.x1},${h}` : `${s.x1 - cw},${h}`;
    const rr = s.x1 >= xEnd ? `${s.x1},${l}` : `${s.x1},${m} ${s.x1 - cw},${l}`;
    const bl = s.x0 <= x0 ? `${s.x0},${l}` : `${s.x0 + cw},${l} ${s.x0},${m}`;
    return `${tl} ${tr} ${rr} ${bl}`;
  });
}

// CLK: rises at E0, E1; falls half a period later
const clkPts: { x: number; v: 0 | 1 }[] = [
  { x: x0, v: 0 },
  { x: E0, v: 1 },
  { x: E0 + 110, v: 0 },
  { x: E1, v: 1 },
  { x: E1 + 110, v: 0 },
];
const clkPath = bit("clk", clkPts);
const cePath = bit("ce", [{ x: x0, v: 1 }]); // chip enabled throughout
const wePath = bit("we", [
  { x: x0, v: 0 },
  { x: E1 - 90, v: 1 },
  { x: E1 + 130, v: 0 },
]);

// ADDR: A0 valid for the read, A1 for the write (changes shortly before E1)
const addrSegs = [
  { x0: x0, x1: E1 - 90 },
  { x0: E1 - 90, x1: xEnd },
];
const addrPolys = busPolys("addr", addrSegs);
const addrLabels = [
  { x: (E0 + 38 + E1 - 90) / 2, y: mid("addr"), t: "A0 (read)" },
  { x: (E1 - 90 + xEnd) / 2, y: mid("addr"), t: "A1 (write)" },
];

// DIN: valid only around the write
const dinSegs = [{ x0: E1 - 90, x1: xEnd }];
const dinPolys = busPolys("din", dinSegs);
const dinLabel = { x: (E1 - 90 + xEnd) / 2, y: mid("din"), t: "D1" };
const dinPre = lo("din"); // before write, din is don't-care (single low line)

// DOUT: invalid until tCQ after E0, then Q(A0)
const doutChange = E0 + 150;
const doutSegs = [{ x0: doutChange, x1: xEnd }];
const doutPolys = busPolys("dout", doutSegs);
const doutLabel = { x: (doutChange + xEnd) / 2, y: mid("dout"), t: "Q(A0)" };

export default function TimingDiagram(): JSX.Element {
  return (
    <figure className={styles.td}>
      <div className={styles.viewport} tabIndex={0} role="region" aria-label="Read and write timing">
        <svg
          viewBox={`0 0 ${W} ${H}`}
          className={styles.svg}
          role="img"
          aria-label="SRAM read and write timing diagram"
        >
          {/* clock-edge guides */}
          {[E0, E1].map((e) => (
            <line
              className={styles.guide}
              key={e}
              x1={e}
              y1={top0 - 6}
              x2={e}
              y2={H - 24}
            />
          ))}
          <text className={styles.region} x={E0} y={14}>
            READ
          </text>
          <text className={styles.region} x={E1} y={14}>
            WRITE
          </text>

          {/* row labels */}
          {rows.map((r) => (
            <text className={styles.label} key={r} x={L} y={mid(r) + 4}>
              {r}
            </text>
          ))}

          {/* bit signals */}
          <path className={styles.wave} d={clkPath} />
          <path className={styles.wave} d={cePath} />
          <path className={styles.wave} d={wePath} />

          {/* addr bus */}
          {addrPolys.map((p) => (
            <polygon className={styles.bus} key={p} points={p} />
          ))}
          {addrLabels.map((s) => (
            <text className={styles.busLabel} key={s.t} x={s.x} y={s.y + 4}>
              {s.t}
            </text>
          ))}

          {/* din: low line before write, bus during write */}
          <line
            className={styles.wave}
            x1={x0}
            y1={dinPre}
            x2={E1 - 90}
            y2={dinPre}
          />
          {dinPolys.map((p) => (
            <polygon className={styles.bus} key={p} points={p} />
          ))}
          <text className={styles.busLabel} x={dinLabel.x} y={dinLabel.y + 4}>
            {dinLabel.t}
          </text>

          {/* dout: invalid (hatched) then valid bus */}
          <rect
            className={styles.invalid}
            x={x0}
            y={hi("dout")}
            width={doutChange - x0}
            height={lo("dout") - hi("dout")}
          />
          <text
            className={styles.invLabel}
            x={(x0 + doutChange) / 2}
            y={mid("dout") + 4}
          >
            invalid
          </text>
          {doutPolys.map((p) => (
            <polygon className={styles.bus} key={p} points={p} />
          ))}
          <text className={styles.busLabel} x={doutLabel.x} y={doutLabel.y + 4}>
            {doutLabel.t}
          </text>

          {/* annotations: tSU, tH on addr@E0 ; tCQ on dout */}
          {/* setup/hold are drawn exaggerated; labels carry the real values */}
          <g className={styles.ann}>
            <line
              x1={E0 - 60}
              y1={rowTop["addr"] - 4}
              x2={E0 - 60}
              y2={lo("addr") + 6}
            />
            <line x1={E0} y1={rowTop["addr"] - 4} x2={E0} y2={lo("addr") + 6} />
            <line
              className={styles.dim}
              x1={E0 - 60}
              y1={rowTop["addr"] - 2}
              x2={E0}
              y2={rowTop["addr"] - 2}
            />
            <text className={styles.annText} x={E0 - 30} y={rowTop["addr"] - 6}>
              tSU≈{tSU}ns
            </text>

            <line
              x1={E0 + 38}
              y1={rowTop["addr"] - 4}
              x2={E0 + 38}
              y2={lo("addr") + 6}
            />
            <line
              className={styles.dim}
              x1={E0}
              y1={lo("addr") + 10}
              x2={E0 + 38}
              y2={lo("addr") + 10}
            />
            <text className={styles.annText} x={E0 + 19} y={lo("addr") + 22}>
              tH≈{tH}ns
            </text>

            <line
              className={styles.dim}
              x1={E0}
              y1={mid("dout") - 14}
              x2={doutChange}
              y2={mid("dout") - 14}
            />
            <text
              className={styles.annText}
              x={(E0 + doutChange) / 2}
              y={mid("dout") - 18}
            >
              t(clk→Q)≈{tCQ}ns
            </text>
          </g>
        </svg>
      </div>
      <figcaption>
        Synchronous read then write, with <code>rstb</code> and all <code>wmask</code>
        {" "}bits high (schematic — setup/hold shown exaggerated).
        Illustrative rising-transition table entries at {corner}. Clock and input slew
        are {timing.example_conditions.clock_slew_ns} ns; output load is {timing.example_conditions.output_load_pf} pF. Full rise/fall constraints are in the{" "}
        <code>.lib</code>; see{" "}
        <Link to="/docs/internals/waveforms/">Waveforms</Link> for the physical
        bitline behavior.
      </figcaption>
    </figure>
  );
}
