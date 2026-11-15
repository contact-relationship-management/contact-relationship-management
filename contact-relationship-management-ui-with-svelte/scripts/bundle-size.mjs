#!/usr/bin/env node
// The size of the built UI's browser code (CRM-T242): what `pnpm build` wrote to
// `.svelte-kit/output/client`, raw and gzipped, as a table for BENCHMARKS.md.
//
//   pnpm build && node scripts/bundle-size.mjs [--json]
//
// "Every page" is SvelteKit's start and app entries and the root layout (node 0), which each
// page loads before its own node; a page's own cost is its node and the chunks it imports,
// which this does not trace. Sizes are of the files as built, before any server compression.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { gzipSync } from "node:zlib";

const ROOT = new URL("..", import.meta.url).pathname;
const CLIENT = join(ROOT, ".svelte-kit/output/client");
const IMMUTABLE = join(CLIENT, "_app/immutable");

function files(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

let all;
try {
  all = files(IMMUTABLE);
} catch {
  console.error(`No build at ${IMMUTABLE}: run \`pnpm build\` first.`);
  process.exit(1);
}

const sized = all
  .filter((path) => /\.(js|css)$/.test(path))
  .map((path) => {
    const bytes = readFileSync(path);
    return {
      file: relative(IMMUTABLE, path),
      type: path.endsWith(".css") ? "css" : "js",
      raw: bytes.length,
      gzip: gzipSync(bytes, { level: 9 }).length,
    };
  });

const sum = (list, key) => list.reduce((total, f) => total + f[key], 0);
const kib = (bytes) => (bytes / 1024).toFixed(1);
const shell = sized.filter(
  (f) => /^entry\/(start|app)\./.test(f.file) || /^nodes\/0\./.test(f.file),
);

const rows = [
  ["JavaScript, all files", sized.filter((f) => f.type === "js")],
  ["CSS, all files", sized.filter((f) => f.type === "css")],
  ["Every page (entries and root layout)", shell],
].map(([what, list]) => ({
  what,
  files: list.length,
  raw: sum(list, "raw"),
  gzip: sum(list, "gzip"),
}));
const largest = [...sized].sort((a, b) => b.gzip - a.gzip).slice(0, 10);

if (process.argv.includes("--json")) {
  console.log(JSON.stringify({ rows, largest }, null, 2));
} else {
  console.log("| What | Files | Raw KiB | Gzip KiB |");
  console.log("| --- | --- | --- | --- |");
  for (const r of rows) {
    console.log(`| ${r.what} | ${r.files} | ${kib(r.raw)} | ${kib(r.gzip)} |`);
  }
  console.log("\nLargest files, gzipped:\n");
  console.log("| File | Raw KiB | Gzip KiB |");
  console.log("| --- | --- | --- |");
  for (const f of largest) {
    console.log(`| \`${f.file}\` | ${kib(f.raw)} | ${kib(f.gzip)} |`);
  }
}
