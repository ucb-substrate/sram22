import React, { type JSX } from "react";
import styles from "./styles.module.css";

// The SRAM22 macro is drawn in a landscape "as-generated" frame with its pin
// row along one long edge and an origin marker at one corner. Because the macro
// is meant to be placed rotated 90°, the four legal orientations are R90 / R270
// and their mirror images. Each tile applies the corresponding transform to the
// same glyph so the rotation and mirroring are visually unambiguous.
const tiles = [
  { name: "R90", def: "W", rot: -90, mirror: false },
  { name: "R270", def: "E", rot: 90, mirror: false },
  { name: "R90 + mirror", def: "FW", rot: -90, mirror: true },
  { name: "R270 + mirror", def: "FE", rot: 90, mirror: true },
];
const cx = 75;
const cy = 75;

export default function OrientationDiagram(): JSX.Element {
  return (
    <div className={styles.orient}>
      <div className={styles.grid}>
        {tiles.map((t) => (
          <figure className={styles.tile} key={t.name}>
            <svg
              viewBox="0 0 150 150"
              role="img"
              aria-label={`${t.name} orientation`}
            >
              <g
                transform={`translate(${cx} ${cy}) rotate(${t.rot}) scale(${
                  t.mirror ? -1 : 1
                } 1)`}
              >
                {/* macro body (landscape in local space) */}
                <rect
                  className={styles.body}
                  x="-58"
                  y="-34"
                  width="116"
                  height="68"
                  rx="7"
                />
                {/* array column hints */}
                <g className={styles.array}>
                  <line x1="-30" y1="-30" x2="-30" y2="30" />
                  <line x1="-10" y1="-30" x2="-10" y2="30" />
                  <line x1="10" y1="-30" x2="10" y2="30" />
                  <line x1="30" y1="-30" x2="30" y2="30" />
                </g>
                {/* pin row along the bottom edge */}
                <g className={styles.pins}>
                  {[-40, -24, -8, 8, 24, 40].map((x) => (
                    <rect key={x} x={x - 3} y="27" width="6" height="6" rx="1" />
                  ))}
                </g>
                {/* origin marker (bottom-left corner) */}
                <path
                  className={styles.origin}
                  d="M -58 34 L -48 34 L -58 24 Z"
                />
                {/* asymmetric "F" so rotation + mirroring read clearly */}
                <g className={styles.f}>
                  <rect x="-8" y="-18" width="6" height="34" />
                  <rect x="-8" y="-18" width="20" height="6" />
                  <rect x="-8" y="-3" width="14" height="5" />
                </g>
              </g>
            </svg>
            <figcaption>
              <strong>{t.name}</strong>
              <span className={styles.def}>DEF&nbsp;{t.def}</span>
            </figcaption>
          </figure>
        ))}
      </div>
      <p className={styles.key}>
        <span className={styles.legend}>
          <span className={`${styles.sw} ${styles.swPin}`} /> pin edge
        </span>
        <span className={styles.legend}>
          <span className={`${styles.sw} ${styles.swOrigin}`} /> origin corner
        </span>
        <span className={styles.legend}>
          <span className={`${styles.sw} ${styles.swF}`} /> orientation marker
        </span>
      </p>
    </div>
  );
}
