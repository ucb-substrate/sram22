import React, { type ReactNode, type JSX } from "react";
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
  return (
    <figure className={styles.figure}>
      <img
        className={plate ? `${styles.img} ${styles.plate}` : styles.img}
        src={useBaseUrl(src)}
        alt={alt}
      />
      {caption && <figcaption className={styles.caption}>{caption}</figcaption>}
    </figure>
  );
}
