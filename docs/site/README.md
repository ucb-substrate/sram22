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
  pages/index.tsx              homepage: install command, layout preview + docs (/)
  components/                  React components, incl. the interactive widgets
  components/layout-renderer.ts  canvas vector renderer for the layout viewers
  theme/MDXComponents.tsx      components available in every MDX page
  theme/NotFound/Content.tsx   custom 404 body
  css/custom.css               design tokens — the single source for the palette
  data/                        data the docs render from (see below)
static/
  img/                         light/dark [S22] logo
  favicon.svg                  logo with colors for the browser's theme
  layout/                      layout-geom.bin (vector geometry) + landing thumbnail
  figures/                     block diagram, read-timing waveform
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

Small layout primitives — `FileTree`, `CardGrid`, `LinkCard`, and `Figure` — are
registered in `src/theme/MDXComponents.tsx` and are available in every page without
an import.

The heavier per-page widgets (`PinExplorer`, `LayoutBrowser`, `MacroTable`,
`PinTable`, `ConfigTable`, `TimingDiagram`, `DecoderDiagram`,
`OrientationDiagram`) are **imported explicitly** by the one page that uses each.
Keep it that way: `MDXComponents` is pulled into every doc page's bundle, and
those widgets carry the layout renderer and the 34 KB of pin geometry with them.

Note that MDX strips leading whitespace from every line inside a JSX block, so
indentation cannot express nesting there — this is why `FileTree` takes a
structured `entries` prop rather than an indented list.

## Styling and color palette

The site follows [Argon's documentation style](https://github.com/ucb-substrate/argon/tree/main/docs),
with blue accents and the same front-page proportions, type scale, overlapping
layout preview, documentation cards, and compact footer. The hero provides the
direct Cargo install command, and the final section links to contribution
resources. Installation details and configuration examples live in the quickstart.
Fira Sans (body), Space Grotesk (headings and navigation), and JetBrains Mono
(code) are self-hosted through Fontsource. Light mode is the default; the site
respects the system preference and retains the light/dark toggle.

The navbar uses the `[S22]` mark without adjacent title text, matching Argon's
[September 10, 2026 documentation updates](https://github.com/ucb-substrate/argon/commit/bf8ec4ee02c05634973f1f9140afc4fc9b32b5d7).
`static/img/logo-mark.svg` and `logo-mark-dark.svg` have transparent backgrounds
and explicit light/dark colors. The lettering is outlined JetBrains Mono, so the
logo does not depend on font loading. `static/favicon.svg` uses the same mark and
follows the browser's color preference. Keep its paths in sync with both logos.

Brand colors and semantic `--sram-*` tokens live in **`src/css/custom.css`**.
Update the brand anchors, light/dark semantic values, and Infima primary-color
shades there when changing the palette. Pages and components consume these
shared tokens rather than defining their own interface colors.

The semantic tokens are defined twice — on `:root` for the light theme and on
`[data-theme="dark"]` for the dark one. Components read these tokens; the homepage
uses Docusaurus's `ThemedImage` to select its light or dark layout thumbnail.

The **layout layer colors** use fixed values in the `LAYERS`/`layerStyles`
tables in `src/components/PinExplorer/` and `src/components/LayoutBrowser/`.
Those interactive canvases retain a dark backdrop. The homepage thumbnails use
separate light and dark palettes in `scripts/render_gds.py`, rendered from the
same GDS geometry.

## Data and reproducibility

The site renders interface, configuration, pin geometry, timing, and catalog data
from `src/data/`. These files have different sources and verification coverage:

- `pins.json`: pin geometry and macro dimensions from the published LEF.
- `timing.json`: complete rise/fall constraint ranges, explicit illustrative table
  entries, operating conditions, and the input Liberty SHA-256.
- `macros.json`: a manually maintained catalog and upstream reported silicon flags.
- `interface.json` / `config.json`: manually maintained descriptions. The checker
  compares pin names/directions/width expressions/layers and config field names to Rust;
  it does not validate prose, behavior, numeric timing, or configuration constraints.

The checked-in example uses the published `sram22_64x32m4w8` macro. To regenerate:

```bash
python3 scripts/extract_data.py sram22_64x32m4w8 macro.lef macro.lib src/data
python3 scripts/export_geom.py macro.gds static/layout/layout-geom.bin
python3 scripts/render_gds.py macro.gds static/layout/composite_preview.webp
python3 scripts/render_gds.py macro.gds static/layout/composite_preview_light.webp --theme light
```

The LEF/Liberty extractor uses only the Python standard library. Geometry/image
scripts also require `gdstk`, `matplotlib`, and `Pillow`. Decompress published
`.gds.gz` files first. Use all views from the same artifact revision. Dimensions
and layout-example captions derive from `pins.json`; review illustrative RTL and
configuration examples separately if changing the featured macro. Update the
published Liberty source link in `docs/interface/timing.mdx` when replacing that
artifact. Its operating conditions and hash are extracted automatically. The parser
rejects incompatible units and missing required timing groups instead of guessing.

The timing diagram labels are explicit first-table entries at documented slew/load
coordinates; they are not averaged constraints or guaranteed operating limits.
`python3 -m unittest discover -s scripts -p 'test_*.py'` checks extraction of full
rise/fall tables, including negative constraints and changed operating conditions.

## Consistency checks (CI)

```bash
npm run check          # TypeScript typecheck
npm run check:docs     # pin attributes and config field names match Rust
npm run build
npm run check:links    # no broken internal links/assets
```

These run in `.github/workflows/docs.yaml` for pull requests matching its path
filters; the workflow also tests the Liberty extractor. It deploys to GitHub Pages
on matching pushes to `master`. These checks do not establish technical accuracy
of every paragraph or qualify the EDA integration outlines.

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

The source button uses Docusaurus's GitHub icon, as in Argon. The adapted Argon
styles and bracket geometry retain their BSD-3-Clause notice in
`static/licenses/argon.txt`.
