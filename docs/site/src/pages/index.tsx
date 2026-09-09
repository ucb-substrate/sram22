import React, { type JSX } from "react";
import Layout from "@theme/Layout";
import Link from "@docusaurus/Link";
import CodeBlock from "@theme/CodeBlock";
import useBaseUrl from "@docusaurus/useBaseUrl";
import useDocusaurusContext from "@docusaurus/useDocusaurusContext";
import macros from "@site/src/data/macros.json";
import styles from "./index.module.css";

const REPO = "https://github.com/rahulk29/sram22";
const MACROS_REPO = macros.repo;

const total = macros.macros.length;
const validated = macros.macros.filter((m) => m.silicon_validated).length;

const FEATURES = [
  {
    title: "Parametric",
    body: "Depth, word width, column-mux ratio (4 or 8), and write granularity are set in a TOML file. The decoders, drivers, and self-timed control are sized accordingly.",
  },
  {
    title: "Complete view set",
    body: "Each run emits GDS, a LEF abstract, a SPICE netlist, Liberty (.lib) timing for multiple PVT corners, and a Verilog behavioral model.",
  },
  {
    title: "P&R integration",
    body: "Instantiated as a hard macro in OpenROAD or Cadence Genus/Innovus, placed rotated 90° in one of four legal orientations.",
  },
  {
    title: "Measured in silicon",
    body: `${validated} of the ${total} published macros have been taped out on SKY130 and verified functional at VDD = 1.8 V, 25 MHz.`,
  },
];

const INSTALL = `git clone ${REPO}.git
cd sram22 && make install && cd -

# describe the SRAM you want
cat > sram22.toml <<'TOML'
num_words  = 64
data_width = 32
mux_ratio  = 4
write_size = 8
TOML

sram22            # generates the macro`;

function GitHubIcon(): JSX.Element {
  return (
    <svg viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z" />
    </svg>
  );
}

function Hero(): JSX.Element {
  return (
    <header className={styles.hero}>
      <div className={styles.container}>
        <img
          className={styles.heroMark}
          src={useBaseUrl("/img/logo-mark.svg")}
          alt=""
          width={64}
          height={64}
        />
        <p className={styles.eyebrow}>
          Single-port SRAM generator · SKY130 · open source
        </p>
        <h1 className={styles.title}>
          Configurable SRAM macros for <span className={styles.grad}>SKY130</span>
          .
        </h1>
        <p className={styles.lede}>
          SRAM22 generates single-port SRAM macros from a short TOML description
          — depth, word width, column-mux ratio, and write granularity. Each run
          produces GDS, LEF, SPICE, Liberty timing across PVT corners, and a
          Verilog model.
        </p>
        <div className={styles.cta}>
          <Link className={`${styles.btn} ${styles.btnPrimary}`} to="/docs/">
            Documentation
          </Link>
          <Link className={`${styles.btn} ${styles.btnGhost}`} to={REPO}>
            <GitHubIcon /> Source on GitHub
          </Link>
        </div>

        <dl className={styles.stats}>
          <div>
            <dt>{total}</dt>
            <dd>published macros</dd>
          </div>
          <div>
            <dt>{validated}</dt>
            <dd>silicon-validated</dd>
          </div>
          <div>
            <dt>SKY130</dt>
            <dd>open PDK</dd>
          </div>
          <div>
            <dt>BSD-3</dt>
            <dd>licensed</dd>
          </div>
        </dl>
      </div>
    </header>
  );
}

function Features(): JSX.Element {
  return (
    <section className={`${styles.container} ${styles.features}`}>
      {FEATURES.map((f) => (
        <article className={styles.card} key={f.title}>
          <h3>{f.title}</h3>
          <p>{f.body}</p>
        </article>
      ))}
    </section>
  );
}

function Showcase(): JSX.Element {
  return (
    <section className={`${styles.container} ${styles.showcase}`}>
      <div>
        <h2>Inspect a generated macro</h2>
        <p>
          The layout below is <code>sram22_64x32m4w8</code> — a 64-word × 32-bit
          macro — rendered from its GDS. Browse it layer by layer, inspect each
          pin&apos;s physical position, and view the internal read waveforms in
          the docs.
        </p>
        <div className={styles.showcaseLinks}>
          <Link to="/docs/internals/layout/">Layout browser →</Link>
          <Link to="/docs/interface/pin-positions/">Pin positions →</Link>
          <Link to="/docs/internals/waveforms/">Waveforms →</Link>
        </div>
      </div>
      <Link className={styles.showcaseFigure} to="/docs/internals/layout/">
        <img
          src={useBaseUrl("/layout/composite_preview.webp")}
          width={2200}
          height={1166}
          alt="GDS layout of the sram22_64x32m4w8 macro"
          loading="lazy"
        />
        <span className={styles.showcaseCap}>
          sram22_64x32m4w8 · 360.32 × 191.00 µm
        </span>
      </Link>
    </section>
  );
}

function GetStarted(): JSX.Element {
  return (
    <section className={`${styles.container} ${styles.getStarted}`}>
      <h2>Install and generate</h2>
      <p>Install the generator, write a config, and run:</p>
      <div className={styles.code}>
        <CodeBlock language="bash">{INSTALL}</CodeBlock>
      </div>
      <div className={styles.cta}>
        <Link
          className={`${styles.btn} ${styles.btnPrimary}`}
          to="/docs/quickstart/"
        >
          Quickstart guide
        </Link>
        <Link className={`${styles.btn} ${styles.btnGhost}`} to={MACROS_REPO}>
          Pre-built macros ↗
        </Link>
      </div>
    </section>
  );
}

export default function Home(): JSX.Element {
  const { siteConfig } = useDocusaurusContext();
  return (
    <Layout
      title={`${siteConfig.title} — configurable SRAM generator for SKY130`}
      description="Configurable single-port SRAM generator for the SKY130 process. Produces GDS, LEF, SPICE, Liberty timing, and a Verilog model from a short TOML description."
    >
      <Hero />
      <main>
        <Features />
        <Showcase />
        <GetStarted />
      </main>
    </Layout>
  );
}
