# SRAM22 website

The [sram22.com](https://sram22.com) website: a custom landing page plus the
documentation, built with [Astro](https://astro.build/) and
[Starlight](https://starlight.astro.build/).

## Develop

```bash
npm install
npm run dev        # local dev server
npm run build      # static build into dist/
npm run preview    # preview the built site
```

## Project layout

```
src/
  pages/index.astro            custom landing page (/)
  content/docs/docs/**         Starlight documentation (served under /docs/)
  content/docs/404.md          custom 404
  components/                  Astro components, incl. the interactive widgets
  data/                        data the docs render from (see below)
  styles/theme.css             design tokens — the single source for the palette
  layout-renderer.ts           canvas vector renderer for the layout viewers
public/
  layout/                      layout-geom.bin (vector geometry) + landing thumbnail
  figures/                     block diagram, read-timing waveform
  CNAME                        custom domain for GitHub Pages
scripts/
  check-docs.mjs               consistency check (docs ⟷ Rust source)
  check-links.mjs              internal link/asset check (run after build)
  export_geom.py               GDS -> compact vector geometry binary (the viewers)
  render_gds.py                landing-page thumbnail image from a GDS
  extract_data.py             regenerate pins.json / timing.json from LEF/.lib
```

The pin-position and layout viewers are **vector** canvas renderers: they draw
the actual macro geometry (`public/layout/layout-geom.bin`, decoded client-side)
and redraw on every zoom, so they stay crisp at any depth with no zoom cap.

## Changing the color palette

All brand colors live in **`src/styles/theme.css`** as CSS custom properties.
Edit the five brand colors (and, optionally, the derived anchors) at the top of
that file; every page and component references the semantic `--sram-*` tokens,
so the change propagates everywhere, including Starlight's theme.

Two things carry baked-in colors and are regenerated rather than themed via CSS:

- the **logo** (`src/assets/logo-mark.svg`, plus an inline copy in
  `src/pages/index.astro`), and
- the **layout layer colors** — the `layerStyles` in `PinExplorer.astro` /
  `LayoutBrowser.astro` (the vector viewers) and the `LAYERS` table in
  `scripts/render_gds.py` (the landing thumbnail).

## Data and reproducibility

The documentation does not hardcode interface or timing facts; it renders them
from JSON in `src/data/`, generated from a real published macro
([`sram22_64x32m4w8`](https://github.com/ucb-substrate/sram22_sky130_macros)):

- `pins.json` — pin geometry, from the macro `.lef` (drives the pin explorer).
- `timing.json` — setup/hold/clk-Q/min-period, from the macro `.lib`.
- `macros.json` — the published macro catalog + silicon-validated flags.
- `interface.json`, `config.json` — the canonical pin and config tables, which
  `check-docs.mjs` verifies against `src/abs.rs` and `SramConfig`.

To regenerate from a different macro, download its `.lef`, `.lib`, and `.gds`
and run the scripts writing directly to the paths the site consumes (they need
Python with `gdstk`, `matplotlib`, and `Pillow`):

```bash
# pins.json + timing.json  → src/data/
python scripts/extract_data.py sram22_64x32m4w8 macro.lef macro.lib src/data
# vector geometry (gzip bytes) → the exact file the viewers fetch
python scripts/export_geom.py macro.gds public/layout/layout-geom.bin
# landing thumbnail
python scripts/render_gds.py macro.gds public/layout/composite_preview.webp
```

`macros.json`, `interface.json`, and `config.json` are hand-maintained;
`check-docs.mjs` verifies `interface.json`/`config.json` against `src/abs.rs`
and `SramConfig`.

## Consistency checks (CI)

```bash
npm run check:docs     # docs data must match the SRAM22 Rust source
npm run build
npm run check:links    # no broken internal links/assets
```

These run in `.github/workflows/docs.yaml` on every pull request; the same
workflow deploys to GitHub Pages on pushes to `master`.

## Deployment notes

The site is served from the repository root domain `sram22.com`, so
`astro.config.mjs` sets `site: "https://sram22.com"` and `base: "/"`. Internal
links are written as site-absolute paths (`/docs/...`); **keep `base` at `/`**
(or update those links and the `CNAME`) if you change hosting.
