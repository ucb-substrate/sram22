import React, { useEffect, useMemo, useRef, useState, type JSX } from "react";
import useBaseUrl from "@docusaurus/useBaseUrl";
import pinData from "@site/src/data/pins.json";
import { loadGeom, createViewer, type Pin, type Viewer } from "../layout-renderer";
import styles from "./styles.module.css";

// Categorical color per pin group — a functional palette chosen for maximum
// distinguishability between pin nets, independent of the brand palette.
const COLORS: Record<string, string> = {
  clk: "#65afff",
  rstb: "#9b8cff",
  ce: "#5cd6b0",
  we: "#ffd35c",
  wmask: "#ff9d5c",
  addr: "#ff6b9d",
  din: "#7ce0ff",
  dout: "#9dff8a",
  vdd: "#ff6b6b",
  vss: "#aab6cc",
};

// Per-layer draw style for the vector geometry.
const LAYER_STYLES = {
  nwell: { fill: "#d8c8a0", alpha: 0.32 },
  diff: { fill: "#5aa14f", alpha: 0.55 },
  poly: { fill: "#c0392b", alpha: 0.6 },
  li1: { fill: "#8e7cc3", alpha: 0.42 },
  met1: { fill: "#335c81", alpha: 0.6 },
  met2: { fill: "#65afff", alpha: 0.3 },
};
const BG = "#0a0f1a";

// Pins with a precomputed µm bounding box, for the renderer.
const PINS: Pin[] = pinData.pins.map((p) => {
  const rects = p.rects as [number, number, number, number][];
  let x0 = Infinity,
    y0 = Infinity,
    x1 = -Infinity,
    y1 = -Infinity;
  for (const [x, y, w, h] of rects) {
    x0 = Math.min(x0, x);
    y0 = Math.min(y0, y);
    x1 = Math.max(x1, x + w);
    y1 = Math.max(y1, y + h);
  }
  return {
    name: p.name,
    base: p.base,
    direction: p.direction,
    layer: p.layer,
    color: COLORS[p.base] ?? "#65afff",
    rects,
    bx: x0,
    by: y0,
    bw: x1 - x0,
    bh: y1 - y0,
    power: p.base === "vdd" || p.base === "vss",
  };
});

// vdd/vss are distributed met2 power grids, not point pins — keep them out of
// the locatable list and expose them via the "Show power grid" toggle instead.
const LIST_GROUPS = pinData.groups.filter(
  (g) => g.base !== "vdd" && g.base !== "vss",
);

const MEMBERS: Record<string, Pin[]> = Object.fromEntries(
  LIST_GROUPS.map((g) => [g.base, PINS.filter((p) => p.base === g.base)]),
);

const dirLabel: Record<string, string> = {
  input: "input",
  output: "output",
  inout: "inout",
};

type Selection =
  | { kind: "pin"; pin: Pin }
  | { kind: "group"; base: string; count: number; direction: string }
  | null;

const num = (v: number, d = 2) => v.toFixed(d);

function InfoBody({ sel }: { sel: Selection }): JSX.Element {
  if (!sel) {
    return (
      <span className={styles.dim}>
        Click a pin — in the list or on the layout — to identify it.
      </span>
    );
  }
  if (sel.kind === "group") {
    return (
      <>
        <div className={styles.infoName}>
          {sel.base}
          {sel.count > 1 ? ` [${sel.count - 1}:0]` : ""}
        </div>
        <dl className={styles.kv}>
          <dt>Bits</dt>
          <dd>{sel.count}</dd>
          <dt>Direction</dt>
          <dd>{dirLabel[sel.direction] || sel.direction}</dd>
        </dl>
        <span className={styles.dim}>
          Click an individual bit to identify it.
        </span>
      </>
    );
  }
  const { pin } = sel;
  return (
    <>
      <div className={styles.infoName}>
        <span className={styles.dot} style={{ background: pin.color }} />
        {pin.name}
      </div>
      <dl className={styles.kv}>
        <dt>Direction</dt>
        <dd>{dirLabel[pin.direction] || pin.direction}</dd>
        <dt>Layer</dt>
        <dd>{pin.layer}</dd>
        <dt>Position</dt>
        <dd>
          ({num(pin.bx)}, {num(pin.by)}) µm
        </dd>
        <dt>Size</dt>
        <dd>
          {num(pin.bw)} × {num(pin.bh)} µm
        </dd>
      </dl>
    </>
  );
}

export default function PinExplorer(): JSX.Element {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [viewer, setViewer] = useState<Viewer | null>(null);
  const [status, setStatus] = useState<string | null>(
    "Loading layout geometry…",
  );
  const [sel, setSel] = useState<Selection>(null);
  const [query, setQuery] = useState("");
  const [power, setPower] = useState(false);
  const [openGroups, setOpenGroups] = useState<Record<string, boolean>>({});
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
        pins: PINS,
        styles: LAYER_STYLES,
        bg: BG,
        // Fired by canvas clicks and by selectPin(); highlight() fires it with
        // null, and the group handler then sets its own selection afterwards.
        onSelect: (pin) => setSel(pin ? { kind: "pin", pin } : null),
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

  useEffect(() => {
    viewer?.setPowerVisible(power);
  }, [viewer, power]);

  const q = query.trim().toLowerCase();

  // Groups (and the pins inside them) surviving the filter.
  const filtered = useMemo(
    () =>
      LIST_GROUPS.map((g) => {
        const members = MEMBERS[g.base];
        const matching = q
          ? members.filter((p) => p.name.toLowerCase().includes(q))
          : members;
        const shown =
          !q || g.base.toLowerCase().includes(q) || matching.length > 0;
        return { group: g, pins: matching, shown };
      }),
    [q],
  );

  // A search that matches pins inside a collapsed group opens it.
  useEffect(() => {
    if (!q) return;
    setOpenGroups((prev) => {
      const next = { ...prev };
      let changed = false;
      for (const g of LIST_GROUPS) {
        const any = MEMBERS[g.base].some((p) =>
          p.name.toLowerCase().includes(q),
        );
        if (any && !next[g.base]) {
          next[g.base] = true;
          changed = true;
        }
      }
      return changed ? next : prev;
    });
  }, [q]);

  const selectGroup = (base: string) => {
    const members = MEMBERS[base];
    viewer?.highlight(
      members.map((p) => p.name),
      true,
    );
    setSel({
      kind: "group",
      base,
      count: members.length,
      direction: members[0]?.direction ?? "",
    });
  };

  const resetView = () => {
    viewer?.selectPin(null, false);
    viewer?.highlight(null, false);
    viewer?.reset();
    setSel(null);
  };

  const activePin = sel?.kind === "pin" ? sel.pin.name : null;
  const activeGroup = sel?.kind === "group" ? sel.base : null;

  return (
    <div className={styles.pinx}>
      <aside className={styles.side}>
        <input
          className={styles.search}
          type="search"
          placeholder="Filter pins…"
          aria-label="Filter pins"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <div className={styles.groups}>
          {filtered.map(({ group: g, pins, shown }) =>
            shown ? (
              <details
                className={styles.grp}
                key={g.base}
                open={openGroups[g.base] ?? false}
                onToggle={(e) =>
                  setOpenGroups((prev) => ({
                    ...prev,
                    [g.base]: (e.currentTarget as HTMLDetailsElement).open,
                  }))
                }
              >
                <summary>
                  <span
                    className={styles.swatch}
                    style={{ background: COLORS[g.base] ?? "#65afff" }}
                  />
                  <button
                    className={`${styles.grpBtn} ${
                      activeGroup === g.base ? styles.active : ""
                    }`}
                    type="button"
                    onClick={(e) => {
                      // Keep the click from also toggling the <details>.
                      e.preventDefault();
                      selectGroup(g.base);
                    }}
                  >
                    {g.base}
                  </button>
                  <span className={styles.grpMeta}>
                    {g.width > 1 ? `[${g.width - 1}:0]` : ""} · {g.direction}
                  </span>
                </summary>
                <ul>
                  {pins.map((p) => (
                    <li key={p.name}>
                      <button
                        className={`${styles.pin} ${
                          activePin === p.name ? styles.active : ""
                        }`}
                        type="button"
                        onClick={() => viewer?.selectPin(p.name, true)}
                      >
                        {p.name}
                      </button>
                    </li>
                  ))}
                </ul>
              </details>
            ) : null,
          )}
        </div>
        <div className={styles.panel}>
          <div className={styles.infoBody}>
            <InfoBody sel={sel} />
          </div>
        </div>
      </aside>

      <div className={styles.stage}>
        <div className={styles.viewport}>
          <canvas className={styles.canvas} ref={canvasRef} role="img"
            aria-label="SRAM22 pin locations on the macro layout. Select a named pin from the list to inspect its position." />
          {status && <div className={styles.loading}>{status}</div>}
        </div>
        <div className={styles.bar}>
          <button className={styles.reset} type="button" onClick={resetView}>
            Reset view
          </button>
          <label className={styles.power}>
            <input
              type="checkbox"
              checked={power}
              onChange={(e) => setPower(e.target.checked)}
            />{" "}
            Show power grid (vdd/vss)
          </label>
          <span className={styles.hint}>
            Scroll to zoom · drag to pan · click a pin to identify it
          </span>
        </div>
      </div>
    </div>
  );
}
