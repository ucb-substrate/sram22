import React, { type ReactNode, type JSX } from "react";
import styles from "./styles.module.css";

/**
 * Numbered walkthrough. Wraps a markdown ordered list and renders each item
 * with a numbered bullet joined by a vertical rule — the Starlight <Steps>
 * look, which Docusaurus has no built-in equivalent for.
 *
 * Usage (the blank lines matter, so MDX parses the list as markdown):
 *
 *   <Steps>
 *
 *   1. First do this.
 *   2. Then do that.
 *
 *   </Steps>
 */
export default function Steps({
  children,
}: {
  children: ReactNode;
}): JSX.Element {
  return <div className={styles.steps}>{children}</div>;
}
