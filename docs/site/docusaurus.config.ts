import type { Config } from "@docusaurus/types";
import type * as Preset from "@docusaurus/preset-classic";
import { themes as prismThemes } from "prism-react-renderer";

// SRAM22 is hosted on a custom domain (GitHub Pages + CNAME at static/CNAME).
// With a root custom domain the base path is "/".
const config: Config = {
  title: "SRAM22",
  tagline: "A configurable, open-source SRAM generator for the SKY130 process.",
  favicon: "favicon.svg",

  url: "https://sram22.com",
  baseUrl: "/",
  // The previous site served every page with a trailing slash. Keep that so
  // existing links to sram22.com stay valid.
  trailingSlash: true,

  organizationName: "rahulk29",
  projectName: "sram22",

  // Broken links are a build error: the docs cross-reference each other
  // heavily, and scripts/check-links.mjs covers assets on top of this.
  onBrokenLinks: "throw",
  markdown: {
    hooks: {
      onBrokenMarkdownLinks: "throw",
    },
  },

  i18n: { defaultLocale: "en", locales: ["en"] },

  presets: [
    [
      "classic",
      {
        docs: {
          // Docs live under /docs/, leaving / for the custom landing page.
          routeBasePath: "/docs",
          path: "docs",
          sidebarPath: "./sidebars.ts",
          editUrl: "https://github.com/rahulk29/sram22/edit/master/docs/site/",
          showLastUpdateTime: true,
        },
        blog: false,
        theme: {
          customCss: "./src/css/custom.css",
        },
      } satisfies Preset.Options,
    ],
  ],

  // Offline search index, built at compile time — no external service.
  themes: [
    [
      "@easyops-cn/docusaurus-search-local",
      {
        hashed: true,
        indexBlog: false,
        docsRouteBasePath: "/docs",
        highlightSearchTermsOnTargetPage: true,
        explicitSearchResultPath: true,
        searchResultLimits: 8,
      },
    ],
  ],

  themeConfig: {
    image: "layout/composite_preview.webp",
    colorMode: {
      defaultMode: "dark",
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: "SRAM22",
      logo: {
        alt: "SRAM22",
        src: "img/logo-mark.svg",
      },
      items: [
        {
          type: "docSidebar",
          sidebarId: "docs",
          position: "left",
          label: "Documentation",
        },
        {
          href: "https://github.com/rahulk29/sram22",
          label: "GitHub",
          position: "right",
        },
      ],
    },
    footer: {
      style: "dark",
      links: [
        {
          title: "Documentation",
          items: [
            { label: "Quickstart", to: "/docs/quickstart/" },
            { label: "Pin list", to: "/docs/interface/pin-list/" },
            { label: "Timing diagrams", to: "/docs/interface/timing/" },
            { label: "Macro catalog", to: "/docs/macros/" },
          ],
        },
        {
          title: "Tutorials",
          items: [
            { label: "OpenROAD flow", to: "/docs/tutorial/openroad/" },
            { label: "Cadence flow", to: "/docs/tutorial/cadence/" },
          ],
        },
        {
          title: "More",
          items: [
            {
              label: "GitHub",
              href: "https://github.com/rahulk29/sram22",
            },
            {
              label: "Prebuilt macros",
              href: "https://github.com/ucb-substrate/sram22_sky130_macros",
            },
            {
              label: "SKY130 PDK",
              href: "https://skywater-pdk.readthedocs.io/",
            },
          ],
        },
      ],
      copyright: `SRAM22 — developed at UC Berkeley. Built with Docusaurus.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.vsDark,
      additionalLanguages: ["bash", "toml", "rust", "tcl", "verilog", "makefile"],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
