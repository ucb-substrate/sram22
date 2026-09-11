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

  organizationName: "ucb-substrate",
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
          editUrl: "https://github.com/ucb-substrate/sram22/edit/main/docs/site/",
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
      defaultMode: "light",
      respectPrefersColorScheme: true,
    },
    navbar: {
      logo: {
        alt: "SRAM22",
        src: "img/logo-mark.svg",
        srcDark: "img/logo-mark-dark.svg",
      },
      items: [
        {
          type: "docSidebar",
          sidebarId: "docs",
          position: "left",
          label: "Documentation",
        },
        {
          href: "https://github.com/ucb-substrate/sram22",
          label: "GitHub",
          position: "right",
        },
      ],
    },
    footer: {
      links: [
        {
          label: "GitHub",
          href: "https://github.com/ucb-substrate/sram22",
        },
        {
          label: "Issues",
          href: "https://github.com/ucb-substrate/sram22/issues",
        },
        {
          label: "License",
          href: "https://github.com/ucb-substrate/sram22/blob/main/LICENSE",
        },
      ],
      copyright: "SRAM22 is distributed under the BSD-3-Clause license.",
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.oneDark,
      additionalLanguages: ["bash", "toml", "rust", "tcl", "verilog", "makefile"],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
