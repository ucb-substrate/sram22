import React, { type JSX } from "react";
import styles from "./styles.module.css";

export default function DecoderDiagram(): JSX.Element {
  return (
    <figure className={styles.dd}>
      <svg viewBox="0 0 720 180" role="img" aria-label="Registered address polarities gated by wlen before hierarchical row decoding and sized wordline drivers">
        <text className={styles.lbl} x="8" y="57">addr Q / Q̅</text>
        <line className={styles.bus} x1="88" y1="65" x2="125" y2="65" />
        <g className={styles.block}>
          <rect x="125" y="35" width="100" height="60" rx="5" />
          <text x="175" y="60">AND gates</text>
          <text x="175" y="78">both polarities</text>
          <rect x="275" y="35" width="180" height="60" rx="5" />
          <text x="365" y="60">Hierarchical decoder</text>
          <text x="365" y="78">two-/three-input stages</text>
          <rect x="505" y="35" width="145" height="60" rx="5" />
          <text x="577" y="60">Sized drivers</text>
          <text x="577" y="78">wordline load</text>
        </g>
        <line className={styles.wire} x1="225" y1="65" x2="275" y2="65" />
        <line className={styles.wire} x1="455" y1="65" x2="505" y2="65" />
        <line className={styles.wl} x1="650" y1="65" x2="708" y2="65" />
        <text className={styles.lbl} x="675" y="52">WL</text>
        <line className={styles.wire} x1="175" y1="130" x2="175" y2="95" />
        <text className={styles.lbl} x="157" y="146">wlen</text>
        <text className={styles.note} x="465" y="133">Logical-effort sizing; effective wordline load capped at 500 fF</text>
      </svg>
      <figcaption>
        Conceptual signal flow: <code>wlen</code> gates registered row-address
        inputs before decoding. Driver stages are part of the sized decoder tree.
      </figcaption>
    </figure>
  );
}
