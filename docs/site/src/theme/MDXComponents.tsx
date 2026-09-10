import MDXComponents from "@theme-original/MDXComponents";
import FileTree from "@site/src/components/FileTree";
import CardGrid from "@site/src/components/CardGrid";
import LinkCard from "@site/src/components/LinkCard";
import Figure from "@site/src/components/Figure";

// Small, cross-cutting layout primitives are registered globally so the docs
// don't repeat an import block on every page.
//
// The per-page data widgets (PinExplorer, LayoutBrowser, MacroTable, …) are
// deliberately NOT registered here: MDXComponents is pulled into every doc
// page's bundle, and those widgets carry the layout renderer and the pin
// geometry with them. They stay explicit imports on the one page that uses
// each.
export default {
  ...MDXComponents,
  FileTree,
  CardGrid,
  LinkCard,
  Figure,
};
