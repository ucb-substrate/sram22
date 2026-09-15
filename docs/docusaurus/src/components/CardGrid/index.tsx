import React, { type ReactNode, type JSX } from "react";
import styles from "./styles.module.css";

/** Responsive grid of <LinkCard>s — the Starlight <CardGrid> equivalent. */
export default function CardGrid({
  children,
}: {
  children: ReactNode;
}): JSX.Element {
  return <div className={styles.grid}>{children}</div>;
}
