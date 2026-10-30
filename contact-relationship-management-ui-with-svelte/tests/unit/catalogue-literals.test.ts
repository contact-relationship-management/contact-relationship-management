// Every grid header and every table's name and empty text comes from the catalogue (CRM-T175;
// spec/worldwide.md code row 42, CRM-R95, CRM-D19), so the Arabic page does not show an English column. This scan
// fails on a column `header: "…"` (or a template string) written as a literal, and on a DataTable whose
// `label="…"` or `empty="…"` is a literal string, in the pages and components; each goes through `t`, `tf` or `tn`.
//
// Mirrors the scans in `locale-format.test.ts` and `right-to-left.test.ts`.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";

const here = join(__dirname, "..", "..");

// "file:trimmed line" → why a literal is right there. Empty today; add an entry only with a reason (a product name
// that is the same in every language, say), never to silence the scan.
const ALLOW: Record<string, string> = {};

const HEADER = /\bheader:\s*["'`]/;

function walk(dir: string): string[] {
  const out: string[] = [];
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) out.push(...walk(path));
    else if (path.endsWith(".svelte")) out.push(path);
  }
  return out;
}

/** The text of each `<DataTable …>` opening tag, read to its closing `>` outside braces and quotes. */
function dataTableTags(text: string): string[] {
  const tags: string[] = [];
  let from = 0;
  for (;;) {
    const start = text.indexOf("<DataTable", from);
    if (start < 0) return tags;
    let depth = 0;
    let quote = "";
    let end = start;
    for (; end < text.length; end++) {
      const c = text[end]!;
      if (quote) {
        if (c === quote) quote = "";
      } else if (c === "{") depth++;
      else if (c === "}") depth--;
      else if (depth === 0 && (c === '"' || c === "'")) quote = c;
      else if (depth === 0 && c === ">") break;
    }
    tags.push(text.slice(start, end + 1));
    from = end + 1;
  }
}

/** English written straight into a grid header or a DataTable's label or empty text, as "rule: text". */
export function literalStrings(text: string, file = ""): string[] {
  const found: string[] = [];
  for (const raw of text.split("\n")) {
    const line = raw.trim();
    if (/^(\/\/|\*|\/\*|<!--)/.test(line)) continue;
    if (ALLOW[`${file}:${line}`]) continue;
    if (HEADER.test(line))
      found.push(`a literal column header: ${line.slice(0, 120)}`);
  }
  for (const tag of dataTableTags(text))
    for (const match of tag.matchAll(/\s(label|empty)=(["'])([^"']*)\2/g)) {
      const attribute = `${match[1]}=${match[2]}${match[3]}${match[2]}`;
      if (ALLOW[`${file}:${attribute}`]) continue;
      found.push(`a literal DataTable ${match[1]}: ${attribute}`);
    }
  return found;
}

describe("no literal grid headers or DataTable labels in src/", () => {
  it("finds them when they are there (the scan works)", () => {
    expect(literalStrings(`    { id: "kind", header: "Kind" },`)).toHaveLength(
      1,
    );
    expect(
      literalStrings("    header: `Withdrawals (${days}d)`,"),
    ).toHaveLength(1);
    expect(
      literalStrings(
        `<DataTable\n  label="Stale deals"\n  columns={cols}\n  empty="No findings."\n/>`,
      ),
    ).toHaveLength(2);
    expect(
      literalStrings(
        `<DataTable label="Touches" columns={cols.map((c) => c)} rows={rows} />`,
      ),
    ).toHaveLength(1);
  });

  it("lets the catalogue through", () => {
    expect(
      literalStrings(`    { id: "kind", header: t("col.kind") },`),
    ).toEqual([]);
    expect(
      literalStrings(
        `<DataTable\n  label={t("exec.stale")}\n  columns={cols}\n  empty={t("exec.noFindings")}\n  testid="exec-stale"\n/>`,
      ),
    ).toEqual([]);
    expect(literalStrings(`// header: "a comment"`)).toEqual([]);
    expect(literalStrings(`<label for="x">{t("a.b")}</label>`)).toEqual([]);
  });

  it("finds none in src/routes or src/lib/components", () => {
    const files = [
      ...walk(join(here, "src", "routes")),
      ...walk(join(here, "src", "lib", "components")),
    ];
    expect(files.length).toBeGreaterThan(10);
    const offenders = files.flatMap((path) => {
      const file = relative(here, path);
      return literalStrings(readFileSync(path, "utf8"), file).map(
        (finding) => `${file}: ${finding}`,
      );
    });
    expect(offenders).toEqual([]);
  });
});
