# SRAM22 website

The [sram22.com](https://sram22.com) website: a custom landing page plus the
documentation, built with [Docusaurus](https://docusaurus.io/).

## Develop

```bash
npm install
npm run dev        # local dev server
npm run build      # static build into build/
npm run serve      # preview the built site
```

## Project layout

```
docusaurus.config.ts           site config (URL, navbar, footer, search, theme)
sidebars.ts                    the docs sidebar: order and labels
docs/**                        the documentation (served under /docs/)
src/
  pages/index.tsx              custom landing page (/)
  components/                  React components, incl. the interactive widgets
  components/layout-renderer.ts  canvas vector renderer for the layout viewers
  theme/MDXComponents.tsx      components available in every MDX page
  theme/NotFound/Content.tsx   custom 404 body
  css/custom.css               design tokens — the single source for the palette
  data/                        data the docs render from (see below)
static/
  layout/                      layout-geom.bin (vector geometry) + landing thumbnail
  figures/                     block diagram, read-timing waveform
  img/logo-mark.svg            logo (navbar + landing hero)
  CNAME                        custom domain for GitHub Pages
scripts/
  check-docs.mjs               consistency check (docs ⟷ Rust source)
  check-links.mjs              internal link/asset check (run after build)
  export_geom.py               GDS -> compact vector geometry binary (the viewers)
  render_gds.py                landing-page thumbnail image from a GDS
  extract_data.py              regenerate pins.json / timing.json from LEF/.lib
```

The pin-position and layout viewers are **vector** canvas renderers: they draw
the actual macro geometry (`static/layout/layout-geom.bin`, decoded client-side)
and redraw on every zoom, so they stay crisp at any depth with no zoom cap.

## Components in MDX

Small layout primitives — `Steps`, `FileTree`, `CardGrid`, `LinkCard`, `Figure`,
and Docusaurus's `Tabs`/`TabItem` — are registered in
`src/theme/MDXComponents.tsx` and are available in every page without an import.

The heavier per-page widgets (`PinExplorer`, `LayoutBrowser`, `MacroTable`,
`PinTable`, `ConfigTable`, `TimingDiagram`, `DecoderDiagram`,
`OrientationDiagram`) are **imported explicitly** by the one page that uses each.
Keep it that way: `MDXComponents` is pulled into every doc page's bundle, and
those widgets carry the layout renderer and the 34 KB of pin geometry with them.

Note that MDX strips leading whitespace from every line inside a JSX block, so
indentation cannot express nesting there — this is why `FileTree` takes a
structured `entries` prop rather than an indented list.

## Changing the color palette

All brand colors live in **`src/css/custom.css`** as CSS custom properties.
Edit the five brand colors (and, optionally, the derived anchors) at the top of
that file; every page and component references the semantic `--sram-*` tokens,
so the change propagates everywhere, including Docusaurus's Infima theme.

The semantic tokens are defined twice — on `:root` for the light theme and on
`[data-theme="dark"]` for the dark one. Components never branch on the theme
themselves; they just read the tokens.

Three things carry baked-in colors and are regenerated rather than themed:

- the **logo** (`static/img/logo-mark.svg`),
- the **layout layer colors** — the `LAYERS`/`layerStyles` tables in
  `src/components/PinExplorer/` and `src/components/LayoutBrowser/` (the vector
  viewers) and in `scripts/render_gds.py` (the landing thumbnail). These are GDS
  layer colors, tuned against a dark backdrop, so those canvases stay dark in
  both themes.

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
python scripts/export_geom.py macro.gds static/layout/layout-geom.bin
# landing thumbnail
python scripts/render_gds.py macro.gds static/layout/composite_preview.webp
```

`macros.json`, `interface.json`, and `config.json` are hand-maintained;
`check-docs.mjs` verifies `interface.json`/`config.json` against `src/abs.rs`
and `SramConfig`.

## Consistency checks (CI)

```bash
npm run check          # TypeScript typecheck
npm run check:docs     # docs data must match the SRAM22 Rust source
npm run build
npm run check:links    # no broken internal links/assets
```

These run in `.github/workflows/docs.yaml` on every pull request; the same
workflow deploys to GitHub Pages on pushes to `master`.

Docusaurus itself fails the build on a broken internal *page* link
(`onBrokenLinks: "throw"`); `check-links.mjs` additionally covers assets.

## Deployment notes

The site is served from the repository root domain `sram22.com`, so
`docusaurus.config.ts` sets `url: "https://sram22.com"` and `baseUrl: "/"`.
Internal links are written as site-absolute paths (`/docs/...`); **keep
`baseUrl` at `/`** (or update those links and the `CNAME`) if you change
hosting.

`trailingSlash: true` is deliberate — every page is served at a URL ending in
`/`, matching the URLs the site has always used. Changing it would break
existing inbound links.
