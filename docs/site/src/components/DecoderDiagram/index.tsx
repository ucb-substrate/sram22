import React, { type JSX } from "react";
import styles from "./styles.module.css";

// Schematic of the hierarchical row decoder + wordline driver sizing chain.
// Buffer rectangles grow left-to-right to evoke logical-effort stage sizing.
const buffers = [
  { x: 432, w: 22, h: 26 },
  { x: 486, w: 30, h: 38 },
  { x: 552, w: 40, h: 54 },
  { x: 630, w: 52, h: 74 },
];

export default function DecoderDiagram(): JSX.Element {
  return (
    <figure className={styles.dd}>
      <svg
        viewBox="0 0 720 260"
        role="img"
        aria-label="Hierarchical decoder and wordline driver sizing chain"
      >
        <defs>
          <marker
            id="dd-ah"
            viewBox="0 0 10 10"
            refX="8"
            refY="5"
            markerWidth="7"
            markerHeight="7"
            orient="auto-start-reverse"
          >
            <path d="M0 0 L10 5 L0 10 z" fill="var(--sram-text-muted)" />
          </marker>
        </defs>
        {/* addr bus */}
        <text className={styles.lbl} x="8" y="48">
          addr
        </text>
        <line className={styles.bus} x1="40" y1="44" x2="92" y2="44" />
        <line className={styles.bus} x1="40" y1="120" x2="92" y2="120" />
        <text className={styles.tick} x="60" y="36">
          log₂R
        </text>

        {/* predecoders */}
        <g className={styles.block}>
          <rect x="92" y="24" width="96" height="40" rx="6" />
          <text x="140" y="48">
            Predecode A
          </text>
        </g>
        <g className={styles.block}>
          <rect x="92" y="100" width="96" height="40" rx="6" />
          <text x="140" y="124">
            Predecode B
          </text>
        </g>

        {/* predecode outputs (one-hot groups) */}
        <line className={styles.wire} x1="188" y1="44" x2="236" y2="44" />
        <line className={styles.wire} x1="188" y1="120" x2="236" y2="80" />

        {/* final NAND decode (row select) */}
        <g className={styles.gate}>
          <path d="M236 28 h26 a34 34 0 0 1 0 68 h-26 z" />
          <circle cx="300" cy="62" r="5" />
          <text x="252" y="66">
            NAND
          </text>
        </g>

        {/* wlen gate */}
        <line className={styles.wire} x1="311" y1="62" x2="348" y2="62" />
        <text className={styles.lbl} x="318" y="150">
          wlen
        </text>
        <line className={styles.wire} x1="338" y1="146" x2="338" y2="86" />
        <g className={styles.gate}>
          <path d="M348 40 a44 44 0 0 1 0 44 h-0 z M348 40 h22 a30 30 0 0 1 0 44 h-22 a44 44 0 0 0 0 -44 z" />
          <text x="372" y="66">
            AND
          </text>
        </g>

        {/* driver chain: graduated inverters */}
        {buffers.map((b) => (
          <g className={styles.inv} key={b.x}>
            <polygon
              points={`${b.x},${62 - b.h / 2} ${b.x},${62 + b.h / 2} ${
                b.x + b.w
              },62`}
            />
            <circle cx={b.x + b.w + 4} cy="62" r="3.5" />
          </g>
        ))}
        <line className={styles.wire} x1="408" y1="62" x2="432" y2="62" />
        <line className={styles.wire} x1="458" y1="62" x2="486" y2="62" />
        <line className={styles.wire} x1="520" y1="62" x2="552" y2="62" />
        <line className={styles.wire} x1="596" y1="62" x2="630" y2="62" />

        {/* wordline into array */}
        <line className={styles.wl} x1="686" y1="62" x2="712" y2="62" />
        <text className={styles.lbl} x="664" y="44">
          WL
        </text>

        {/* size annotation */}
        <path className={styles.arrow} d="M432 110 H 690" />
        <text className={styles.note} x="560" y="128">
          increasing drive strength (logical effort)
        </text>
        <text className={styles.note} x="560" y="146">
          sized to charge the wordline cap up to WORDLINE_CAP_MAX
        </text>
      </svg>
      <figcaption>
        Hierarchical decode: address bits are split across predecoders, ANDed in
        the final NAND stage to select one row, gated by <code>wlen</code>, then
        driven by a graduated buffer chain whose stages are sized by logical
        effort.
      </figcaption>
    </figure>
  );
}
