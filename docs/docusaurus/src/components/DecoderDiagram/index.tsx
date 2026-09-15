import React, { type JSX } from "react";
import styles from "./styles.module.css";

export default function DecoderDiagram(): JSX.Element {
  return (
    <figure className={styles.dd}>
      <div className={styles.viewport} tabIndex={0} role="region" aria-label="Row decoder signal flow">
        <svg viewBox="0 0 820 170" role="img" aria-label="Registered address polarities gated by wlen before hierarchical row decoding and sized wordline drivers">
          <text className={styles.lbl} x="8" y="55">addr Q / Q̅</text>
          <line className={styles.bus} x1="96" y1="66" x2="135" y2="66" />
          <g className={styles.block}>
            <rect x="135" y="34" width="140" height="64" rx="5" />
            <text x="205" y="60">AND gates</text>
            <text x="205" y="80">both polarities</text>
            <rect x="325" y="34" width="215" height="64" rx="5" />
            <text x="432.5" y="60">Hierarchical decoder</text>
            <text x="432.5" y="80">2-/3-input stages</text>
            <rect x="590" y="34" width="160" height="64" rx="5" />
            <text x="670" y="60">Sized drivers</text>
            <text x="670" y="80">wordline load</text>
          </g>
          <line className={styles.wire} x1="275" y1="66" x2="325" y2="66" />
          <line className={styles.wire} x1="540" y1="66" x2="590" y2="66" />
          <line className={styles.wl} x1="750" y1="66" x2="808" y2="66" />
          <text className={styles.lbl} x="775" y="53">WL</text>
          <line className={styles.wire} x1="205" y1="128" x2="205" y2="98" />
          <text className={styles.lbl} x="187" y="148">wlen</text>
        </svg>
      </div>
      <figcaption>
        Conceptual signal flow: <code>wlen</code> gates registered row-address
        inputs before decoding. Driver stages are part of the sized decoder tree;
        their effective wordline load is capped at 500 fF for sizing.
      </figcaption>
    </figure>
  );
}
