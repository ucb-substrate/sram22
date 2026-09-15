#!/usr/bin/env node
/**
 * Internal link & asset checker.
 *
 * Crawls the built site in build/ and verifies that every internal href/src
 * resolves to a real file (page or asset). External links, mailto:, tel:, and
 * pure #fragment links are ignored. Run after `docusaurus build`.
 */
import { readFileSync, readdirSync, statSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve, join, posix } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const buildDir = resolve(here, "..", "build");

if (!existsSync(buildDir)) {
  console.error("✗ build/ not found — run `npm run build` first.");
  process.exit(1);
}

// collect all html files
function walk(dir, acc = []) {
  for (const e of readdirSync(dir)) {
    const p = join(dir, e);
    const s = statSync(p);
    if (s.isDirectory()) walk(p, acc);
    else if (e.endsWith(".html")) acc.push(p);
  }
  return acc;
}

const htmlFiles = walk(buildDir);
const attrRe = /(?:href|src)\s*=\s*"([^"]*)"/g;

// resolve a URL path (site-absolute, base "/") to a build/ filesystem path
function targetExists(urlPath) {
  const p = urlPath.split("#")[0].split("?")[0];
  if (p === "") return true;
  let dec;
  try {
    dec = decodeURIComponent(p);
  } catch {
    dec = p; // malformed percent-encoding: check the raw path, don't crash
  }
  const fsPath = join(buildDir, dec);
  if (p.endsWith("/")) {
    // The site sets trailingSlash: true, so page URLs end in "/".
    return existsSync(join(fsPath, "index.html"));
  }
  // No trailing slash → must be a real file (an asset). A directory here means
  // the link dropped its trailing slash and would 404 on strict hosts.
  return existsSync(fsPath) && statSync(fsPath).isFile();
}

let broken = 0;
let checked = 0;

for (const file of htmlFiles) {
  const html = readFileSync(file, "utf8");
  // the page's URL path (for resolving relative links)
  const pageUrl =
    "/" + posix.relative(buildDir.replaceAll("\\", "/"), file.replaceAll("\\", "/"));
  const pageDir = posix.dirname(pageUrl).replace(/\/index\.html$/, "/");

  const seen = new Set();
  for (const m of html.matchAll(attrRe)) {
    let link = m[1].trim();
    if (seen.has(link)) continue;
    seen.add(link);
    if (
      link === "" ||
      link.startsWith("#") ||
      link.startsWith("http://") ||
      link.startsWith("https://") ||
      link.startsWith("mailto:") ||
      link.startsWith("tel:") ||
      link.startsWith("data:")
    )
      continue;

    // resolve relative to the page directory
    let abs = link.startsWith("/")
      ? link
      : posix.normalize(posix.join(pageDir, link));

    checked++;
    if (!targetExists(abs)) {
      broken++;
      console.error(
        `  ✗ ${posix.relative("/", pageUrl)} → "${link}" (resolved ${abs})`,
      );
    }
  }
}

console.log("");
if (broken) {
  console.error(
    `✗ link check FAILED: ${broken} broken internal link(s) across ${htmlFiles.length} pages.`,
  );
  process.exit(1);
}
console.log(
  `✓ link check passed: ${checked} internal links across ${htmlFiles.length} pages all resolve.`,
);
