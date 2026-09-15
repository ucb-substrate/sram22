import type { SidebarsConfig } from "@docusaurus/plugin-content-docs";

// Hand-written to control the order and the labels independently of each
// page's <h1> (e.g. "SRAM22 documentation" shows as "Overview" here).
const sidebars: SidebarsConfig = {
  docs: [
    { type: "doc", id: "index", label: "Overview" },
    { type: "doc", id: "quickstart", label: "Quickstart" },
    {
      type: "category",
      label: "Tutorials",
      collapsed: false,
      items: [
        { type: "doc", id: "tutorial/openroad", label: "OpenROAD flow" },
        { type: "doc", id: "tutorial/cadence", label: "Cadence flow" },
      ],
    },
    {
      type: "category",
      label: "Interface",
      collapsed: false,
      items: [
        { type: "doc", id: "interface/pin-list", label: "Pin list" },
        { type: "doc", id: "interface/pin-positions", label: "Pin positions" },
        { type: "doc", id: "interface/timing", label: "Timing diagrams" },
      ],
    },
    {
      type: "category",
      label: "Internals",
      collapsed: false,
      items: [
        { type: "doc", id: "internals/waveforms", label: "Waveforms" },
        { type: "doc", id: "internals/layout", label: "Layout" },
        { type: "doc", id: "internals/algorithms", label: "Algorithms" },
      ],
    },
    { type: "doc", id: "macros", label: "Macro catalog" },
    { type: "doc", id: "orientations", label: "Orientations" },
  ],
};

export default sidebars;
