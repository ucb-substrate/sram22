import React, { type ReactNode } from "react";
import clsx from "clsx";
import Link from "@docusaurus/Link";
import Heading from "@theme/Heading";
import type { Props } from "@theme/NotFound/Content";

export default function NotFoundContent({ className }: Props): ReactNode {
  return (
    <main className={clsx("container margin-vert--xl", className)}>
      <div className="row">
        <div className="col col--6 col--offset-3">
          <Heading as="h1" className="hero__title">
            Page not found
          </Heading>
          <p>
            That page doesn&apos;t exist. Head back to the{" "}
            <Link to="/docs/">documentation home</Link> or the{" "}
            <Link to="/">SRAM22 landing page</Link>.
          </p>
        </div>
      </div>
    </main>
  );
}
