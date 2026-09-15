import React, { useEffect, useRef, useState, type JSX } from "react";
import useBaseUrl from "@docusaurus/useBaseUrl";
import { loadGeom, createViewer, type Viewer } from "../layout-renderer";
import styles from "./styles.module.css";

// Draw order (bottom first) + style per layer. These colors are the GDS layer
// colors and are deliberately theme-independent — see scripts/render_gds.py,
// whose LAYERS table must be kept in sync.
const LAYERS = [
  { key: "nwell", label: "Nwell", color: "#d8c8a0", alpha: 0.4, on: true },
  { key: "diff", label: "Diff", color: "#5aa14f", alpha: 0.7, on: true },
  { key: "poly", label: "Poly", color: "#c0392b", alpha: 0.72, on: true },
  { key: "li1", label: "Li1", color: "#8e7cc3", alpha: 0.5, on: true },
  { key: "met1", label: "Met1", color: "#4f7ab0", alpha: 0.62, on: true },
  { key: "met2", label: "Met2", color: "#65afff", alpha: 0.34, on: false },
];

const LAYER_STYLES = Object.fromEntries(
  LAYERS.map((l) => [l.key, { fill: l.color, alpha: l.alpha }]),
);
const DEFAULT_OFF = LAYERS.filter((l) => !l.on).map((l) => l.key);
const BG = "#0a0f1a";

const initialVisible = () =>
  Object.fromEntries(LAYERS.map((l) => [l.key, l.on])) as Record<
    string,
    boolean
  >;

export default function LayoutBrowser(): JSX.Element {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [viewer, setViewer] = useState<Viewer | null>(null);
  const [status, setStatus] = useState<string | null>(
    "Loading layout geometry…",
  );
  const [visible, setVisible] = useState<Record<string, boolean>>(
    initialVisible,
  );
  const geomUrl = useBaseUrl("/layout/layout-geom.bin");

  useEffect(() => {
    let cancelled = false;
    let created: Viewer | null = null;

    (async () => {
      let geom;
      try {
        geom = await loadGeom(geomUrl);
      } catch {
        if (!cancelled) {
          setStatus("Could not load the layout geometry in this browser.");
        }
        return;
      }
      if (cancelled || !canvasRef.current) return;
      created = createViewer({
        canvas: canvasRef.current,
        geom,
        pins: [],
        styles: LAYER_STYLES,
        bg: BG,
        hidden: DEFAULT_OFF,
      });
      setViewer(created);
      setStatus(null);
    })();

    return () => {
      cancelled = true;
      created?.destroy();
      setViewer(null);
    };
  }, [geomUrl]);

  // Keep the viewer's layer visibility in sync with the checkboxes.
  useEffect(() => {
    if (!viewer) return;
    for (const l of LAYERS) viewer.setLayerVisible(l.key, visible[l.key]);
  }, [viewer, visible]);

  const setAll = (on: boolean) =>
    setVisible(
      Object.fromEntries(LAYERS.map((l) => [l.key, on])) as Record<
        string,
        boolean
      >,
    );

  return (
    <div className={styles.lyb}>
      <div className={styles.tools}>
        <div className={styles.layers}>
          {LAYERS.map((l) => (
            <label className={styles.layer} key={l.key}>
              <input
                type="checkbox"
                checked={visible[l.key]}
                onChange={(e) =>
                  setVisible((v) => ({ ...v, [l.key]: e.target.checked }))
                }
              />
              <span
                className={styles.swatch}
                style={{ background: l.color }}
              />
              {l.label}
            </label>
          ))}
        </div>
        <div className={styles.actions}>
          <button
            type="button"
            className={styles.btn}
            onClick={() => setAll(true)}
          >
            All
          </button>
          <button
            type="button"
            className={styles.btn}
            onClick={() => setAll(false)}
          >
            None
          </button>
          <button
            type="button"
            className={styles.btn}
            onClick={() => viewer?.reset()}
          >
            Reset view
          </button>
        </div>
      </div>

      <div className={styles.viewport}>
        <canvas className={styles.canvas} ref={canvasRef} role="img"
          aria-label="SRAM22 macro layout. Use the layer controls above to inspect the geometry." />
        {status && <div className={styles.loading}>{status}</div>}
      </div>
      <p className={styles.hint}>
        Scroll to zoom · drag to pan · toggle layers above.
      </p>
    </div>
  );
}
