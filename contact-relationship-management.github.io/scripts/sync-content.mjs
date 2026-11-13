#!/usr/bin/env node
// Copy the repository documents this site shows into src/content/, so the site builds on its own once `git subtree`
// has exported this directory (the exported repository has no ../README.md to read).
//
//   node scripts/sync-content.mjs           rewrite src/content/docs.json and src/content/briefs.json
//   node scripts/sync-content.mjs --check   exit 1 if either differs from the monorepo files
//
// Run from the monorepo only (it reads the parent directory). The list of documents is here and nowhere else:
// src/lib/site.ts reads the brief index (briefs.json) and the server reads the text (docs.json).
//
// The path-to-production pages are every locales/<locale>/path-to-production/index.md.
// The role briefs are every locales/<locale>/for/<slug>/index.md. Their slugs are in each locale's own script
// (Arabic, Chinese, Hindi and Welsh slugs differ from the English ones), so a brief is matched across locales by its
// `.locale-peer-id`, the same 32-character id in every locale for the same role (spec/locale/index.md).
import { readFileSync, writeFileSync, readdirSync, existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const SITE = join(dirname(fileURLToPath(import.meta.url)), "..");
const ROOT = join(SITE, "..");
const DOCS_OUT = join(SITE, "src", "content", "docs.json");
const BRIEFS_OUT = join(SITE, "src", "content", "briefs.json");

// The documents, by repository path.
const DOCS = [
  "README.md",
  "COMPARISONS.md",
  "INSTALL.md",
  "docs/tutorial-first-tour.md",
  "docs/tutorial-family-bridges.md",
  "spec/index.md",
  "spec/locale/index.md",
  "locales/en-001/evaluate/index.md",
];

// The comparison pages, /compare/<product>/.
const COMPARE = readdirSync(join(ROOT, "docs", "compare"))
  .filter((f) => f.endsWith(".md"))
  .sort()
  .map((f) => `docs/compare/${f}`);

function briefs() {
  const out = [];
  for (const locale of readdirSync(join(ROOT, "locales")).sort()) {
    const forDir = join(ROOT, "locales", locale, "for");
    if (!existsSync(forDir)) continue;
    for (const slug of readdirSync(forDir).sort()) {
      const path = `locales/${locale}/for/${slug}/index.md`;
      const peerFile = join(forDir, slug, ".locale-peer-id");
      if (!existsSync(join(ROOT, path))) continue;
      if (!existsSync(peerFile)) throw new Error(`no .locale-peer-id beside ${path}`);
      out.push({ locale, slug, peer: readFileSync(peerFile, "utf8").trim(), path });
    }
  }
  return out;
}

// The locale home pages: locales/<locale>/index.md, with the same `.locale-peer-id` in every locale that has one.
function homes() {
  return readdirSync(join(ROOT, "locales"))
    .sort()
    .filter((locale) => existsSync(join(ROOT, "locales", locale, "index.md")))
    .map((locale) => {
      if (!existsSync(join(ROOT, "locales", locale, ".locale-peer-id")))
        throw new Error(`no .locale-peer-id beside locales/${locale}/index.md`);
      return locale;
    });
}

// The path-to-production pages: locales/<locale>/path-to-production/index.md, in the locales that have one.
function productionPages() {
  return readdirSync(join(ROOT, "locales"))
    .sort()
    .filter((locale) => existsSync(join(ROOT, "locales", locale, "path-to-production", "index.md")));
}

/** @param {unknown} value */
const json = (value) => JSON.stringify(value, null, 2) + "\n";

function render() {
  const list = briefs();
  const homeLocales = homes();
  const production = productionPages();
  /** @type {Record<string, string>} */
  const files = {};
  for (const p of [...DOCS, ...COMPARE, ...homeLocales.map((l) => `locales/${l}/index.md`), ...production.map((l) => `locales/${l}/path-to-production/index.md`), ...list.map((b) => b.path)]) files[p] = readFileSync(join(ROOT, p), "utf8");
  return {
    docs: json({ generated: "by scripts/sync-content.mjs from the monorepo; do not edit by hand", files }),
    briefs: json({
      generated: "by scripts/sync-content.mjs from the monorepo; do not edit by hand",
      homes: homeLocales,
      production,
      compare: COMPARE.map((p) => p.replace(/^docs\/compare\/|\.md$/g, "")),
      briefs: list.map(({ locale, slug, peer }) => ({ locale, slug, peer })),
    }),
  };
}

if (!existsSync(join(ROOT, "locales")) || !existsSync(join(ROOT, "spec", "index.md"))) {
  console.error("sync-content: run this inside the monorepo (the parent directory has no locales/ or spec/)");
  process.exit(2);
}
const out = render();
if (process.argv.includes("--check")) {
  /** @type {[string, string][]} */
  const pairs = [
    [DOCS_OUT, out.docs],
    [BRIEFS_OUT, out.briefs],
  ];
  const stale = pairs.filter(([file, text]) => (existsSync(file) ? readFileSync(file, "utf8") : "") !== text);
  if (stale.length > 0) {
    console.error("src/content/ is stale: run `pnpm run sync` (node scripts/sync-content.mjs)");
    process.exit(1);
  }
  console.log("src/content/docs.json and src/content/briefs.json are current");
} else {
  writeFileSync(DOCS_OUT, out.docs);
  writeFileSync(BRIEFS_OUT, out.briefs);
  console.log(`wrote src/content/ (${Object.keys(JSON.parse(out.docs).files).length} documents)`);
}
