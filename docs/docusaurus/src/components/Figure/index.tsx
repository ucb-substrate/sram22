import React, { useState, type ReactNode, type JSX } from "react";
import useBaseUrl from "@docusaurus/useBaseUrl";
import styles from "./styles.module.css";

/**
 * A figure from static/: image plus optional caption.
 *
 * `plate` puts the image on a white backdrop — for exported plots that are
 * drawn in dark ink on transparency and would otherwise vanish in dark mode.
 */
export default function Figure({
  src,
  alt,
  caption,
  plate = false,
}: {
  src: string;
  alt: string;
  caption?: ReactNode;
  plate?: boolean;
}): JSX.Element {
  const [expanded, setExpanded] = useState(false);
  return (
    <figure className={styles.figure}>
      <div className={styles.controls}>
        <button
          type="button"
          className={styles.zoom}
          aria-pressed={expanded}
          onClick={() => setExpanded(!expanded)}
        >
          {expanded ? "Fit to page" : "Enlarge figure"}
        </button>
      </div>
      <div className={plate ? `${styles.frame} ${styles.plate}` : styles.frame}>
        <div
          className={styles.viewport}
          tabIndex={expanded ? 0 : undefined}
          role="region"
          aria-label={alt}
        >
          <img
            className={expanded ? `${styles.img} ${styles.expanded}` : styles.img}
            src={useBaseUrl(src)}
            alt={alt}
          />
        </div>
      </div>
      {caption && <figcaption className={styles.caption}>{caption}</figcaption>}
    </figure>
  );
}
