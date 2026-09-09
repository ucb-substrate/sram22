// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import mdx from "@astrojs/mdx";

// SRAM22 is hosted on a custom domain (GitHub Pages + CNAME).
// With a root custom domain the base path is "/".
export default defineConfig({
  site: "https://sram22.com",
  base: "/",
  trailingSlash: "always",
  integrations: [
    starlight({
      title: "SRAM22",
      description:
        "A configurable, open-source SRAM generator for the SKY130 process.",
      logo: {
        // Icon mark only; the "SRAM22" title renders next to it in Geist.
        // (Web fonts do not apply to SVG loaded as <img>, so we avoid SVG text.)
        src: "./src/assets/logo-mark.svg",
        replacesTitle: false,
      },
      favicon: "/favicon.svg",
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/rahulk29/sram22",
        },
      ],
      customCss: ["./src/styles/theme.css"],
      // The docs live under /docs/, leaving / for the custom landing page.
      sidebar: [
        { label: "Overview", link: "/docs/" },
        { label: "Quickstart", link: "/docs/quickstart/" },
        {
          label: "Tutorials",
          items: [
            { label: "OpenROAD flow", link: "/docs/tutorial/openroad/" },
            { label: "Cadence flow", link: "/docs/tutorial/cadence/" },
          ],
        },
        {
          label: "Interface",
          items: [
            { label: "Pin list", link: "/docs/interface/pin-list/" },
            { label: "Pin positions", link: "/docs/interface/pin-positions/" },
            { label: "Timing diagrams", link: "/docs/interface/timing/" },
          ],
        },
        {
          label: "Internals",
          items: [
            { label: "Waveforms", link: "/docs/internals/waveforms/" },
            { label: "Layout", link: "/docs/internals/layout/" },
            { label: "Algorithms", link: "/docs/internals/algorithms/" },
          ],
        },
        { label: "Macro catalog", link: "/docs/macros/" },
        { label: "Orientations", link: "/docs/orientations/" },
      ],
      editLink: {
        baseUrl:
          "https://github.com/rahulk29/sram22/edit/master/docs/site/",
      },
      lastUpdated: true,
    }),
    mdx(),
  ],
});
