import React, { type JSX } from "react";
import Link from "@docusaurus/Link";
import styles from "./styles.module.css";

/**
 * A titled, described link rendered as a card — the Starlight <LinkCard>
 * equivalent. The whole card is the click target.
 */
export default function LinkCard({
  title,
  href,
  description,
}: {
  title: string;
  href: string;
  description?: string;
}): JSX.Element {
  return (
    <Link className={styles.card} to={href}>
      <span className={styles.title}>
        {title}
        <span className={styles.arrow} aria-hidden="true">
          →
        </span>
      </span>
      {description && <span className={styles.desc}>{description}</span>}
    </Link>
  );
}
