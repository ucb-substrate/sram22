import pins from "@site/src/data/pins.json";
import React, { useCallback, useEffect, useRef, useState, type JSX, type ReactNode } from "react";
import Layout from "@theme/Layout";
import ThemedImage from "@theme/ThemedImage";
import GitHubIcon from "@theme/Icon/Socials/GitHub";
import Link from "@docusaurus/Link";
import useBaseUrl from "@docusaurus/useBaseUrl";
import styles from "./index.module.css";

const REPO = "https://github.com/ucb-substrate/sram22";
const INSTALL = `cargo install --git ${REPO} --locked sram22`;

// Lucide icons (ISC), matching the documentation cards on the Argon site.
const ICONS = {
  book: (
    <>
      <path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z" />
      <path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z" />
    </>
  ),
  code: (
    <>
      <polyline points="16 18 22 12 16 6" />
      <polyline points="8 6 2 12 8 18" />
    </>
  ),
  terminal: (
    <>
      <polyline points="4 17 10 11 4 5" />
      <line x1="12" y1="19" x2="20" y2="19" />
    </>
  ),
  window: (
    <>
      <rect x="2" y="4" width="20" height="16" rx="2" />
      <path d="M2 9h20" />
      <path d="M8 9v11" />
    </>
  ),
} satisfies Record<string, ReactNode>;

const DOC_GROUPS = [
  {
    icon: "book",
    title: "Quickstart",
    description: "Installation, TOML configuration, and command-line options.",
    to: "/docs/quickstart/",
    action: "Read the guide",
  },
  {
    icon: "code",
    title: "Interface",
    description: "Pin definitions, physical pin positions, and read/write timing.",
    to: "/docs/interface/pin-list/",
    action: "Interface reference",
  },
  {
    icon: "terminal",
    title: "Physical design",
    description: "OpenROAD and Cadence flows, macro placement, and orientations.",
    to: "/docs/tutorial/openroad/",
    action: "Integration guide",
  },
  {
    icon: "window",
    title: "Internals",
    description: "Layout geometry, control waveforms, and generation algorithms.",
    to: "/docs/internals/layout/",
    action: "Browse the layout",
  },
] satisfies Array<{
  icon: keyof typeof ICONS;
  title: string;
  description: string;
  to: string;
  action: string;
}>;

// Adapted from Argon's install strip (BSD-3-Clause; static/licenses/argon.txt).
function InstallCommand({ command }: { command: string }): JSX.Element {
  const [copied, setCopied] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => () => clearTimeout(timer.current), []);

  const copy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(command);
      setCopied(true);
      clearTimeout(timer.current);
      timer.current = setTimeout(() => setCopied(false), 2000);
    } catch {
      // The command remains selectable if clipboard access is unavailable.
    }
  }, [command]);

  return (
    <div className={styles.install}>
      <code>
        {command.split(" ").map((token, index) => (
          <React.Fragment key={index}>
            {index > 0 && " "}
            <span className={styles.token}>{token}</span>
          </React.Fragment>
        ))}
      </code>
      <button
        type="button"
        className={`${styles.copy}${copied ? ` ${styles.copied}` : ""}`}
        onClick={copy}
        aria-label={copied ? "Copied" : "Copy install command"}
        title={copied ? "Copied" : "Copy"}
      >
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          {copied ? (
            <path d="M20 6 9 17l-5-5" />
          ) : (
            <>
              <rect width="14" height="14" x="8" y="8" rx="2" ry="2" />
              <path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" />
            </>
          )}
        </svg>
      </button>
    </div>
  );
}

function Icon({ name }: { name: keyof typeof ICONS }): JSX.Element {
  return (
    <svg
      className={styles.icon}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.75"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {ICONS[name]}
    </svg>
  );
}

export default function Home(): JSX.Element {
  return (
    <Layout
      wrapperClassName="front-page"
      description="SRAM22 generates single-port SRAM macros for SKY130 from a TOML configuration. Installation, configuration, interface reference, and physical-design integration."
    >
      <main className={styles.main}>
        <section className={styles.hero} aria-labelledby="sram22-title">
          <div className={styles.container}>
            <h1 id="sram22-title">SRAM22</h1>
            <p className={styles.tagline}>
              A parametric single-port SRAM generator for the SKY130 process.
            </p>
            <div className={styles.actions}>
              <Link
                className={`${styles.btn} ${styles.btnPrimary}`}
                to="/docs/quickstart/"
              >
                Quickstart
              </Link>
              <Link
                className={`${styles.btn} ${styles.btnSecondary}`}
                to={REPO}
              >
                <GitHubIcon
                  className={styles.githubIcon}
                  aria-hidden="true"
                  focusable="false"
                />
                Source on GitHub
              </Link>
            </div>
            <InstallCommand command={INSTALL} />
          </div>
        </section>

        <div className={`${styles.container} ${styles.shotWrap}`}>
          <figure className={styles.shot}>
            <Link to="/docs/internals/layout/">
              <ThemedImage
                sources={{
                  light: useBaseUrl("/layout/composite_preview_light.webp"),
                  dark: useBaseUrl("/layout/composite_preview.webp"),
                }}
                width={2200}
                height={1166}
                alt={`GDS layout of ${pins.macro}. Open the interactive layout browser.`}
              />
            </Link>
            <figcaption>
              <code>{pins.macro}</code> · {pins.num_words} words × {pins.data_width} bits · {pins.um_width.toFixed(2)} × {pins.um_height.toFixed(2)} µm.
              {" "}<Link to="/docs/internals/layout/">Open the layout browser</Link>.
            </figcaption>
          </figure>
        </div>

        <section
          className={`${styles.container} ${styles.section}`}
          aria-labelledby="features-title"
        >
          <h2 id="features-title" className={styles.heading}>
            Features
          </h2>
          <div className={styles.details}>
            <div>
              <h3>Parametric generation</h3>
              <p>
                Configurable depth, word width, column multiplexing, and write-mask
                granularity. SRAM22 generates the array geometry and sizes the
                decoders and wordline drivers for each{" "}
                <Link to="/docs/quickstart/#configuration-reference">
                  configuration
                </Link>.
              </p>
            </div>
            <div>
              <h3>Physical-design views</h3>
              <p>
                GDS layout, LEF abstracts, SPICE netlists, and Verilog models for
                simulation and place-and-route. BWRC builds also support Liberty
                characterization and DRC, LVS, and parasitic extraction through
                commercial tools.
              </p>
            </div>
            <div>
              <h3>Self-timed architecture</h3>
              <p>
                Replica-based timing controls the wordline pulse and sense-amplifier
                activation. A single bitcell array shares precharge, transmission-gate
                column muxes, sense amplifiers, and masked write drivers. See the{" "}
                <Link to="/docs/internals/algorithms/#self-timed-control">control sequence</Link>.
              </p>
            </div>
          </div>
        </section>

        <nav className={styles.band} aria-labelledby="docs-title">
          <div className={`${styles.container} ${styles.section}`}>
            <h2 id="docs-title" className={styles.heading}>Documentation</h2>
            <div className={styles.cards}>
              {DOC_GROUPS.map((group) => (
                <div className={styles.card} key={group.title}>
                  <span className={styles.iconWrap}>
                    <Icon name={group.icon} />
                  </span>
                  <h3>{group.title}</h3>
                  <p>{group.description}</p>
                  <Link
                    className={`${styles.btn} ${styles.btnSecondary} ${styles.btnSmall}`}
                    to={group.to}
                  >
                    {group.action}
                  </Link>
                </div>
              ))}
            </div>
          </div>
        </nav>

        <section
          className={`${styles.container} ${styles.section}`}
          aria-labelledby="involved-title"
        >
          <h2 id="involved-title" className={styles.heading}>Get involved</h2>
          <p className={styles.involved}>
            SRAM22 is developed on <Link to={REPO}>GitHub</Link>. Bug reports
            and pull requests are welcome. Report problems in the{" "}
            <Link to={`${REPO}/issues`}>issue tracker</Link>, or follow the{" "}
            <Link to={`${REPO}#local-checkout-or-custom-fork`}>
              local checkout instructions
            </Link>{" "}
            to make changes and contribute.
          </p>
        </section>
      </main>
    </Layout>
  );
}
