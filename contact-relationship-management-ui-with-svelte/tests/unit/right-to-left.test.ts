// Right-to-left (CRM-T174; spec/worldwide.md code rows 38 and 39, CRM-R95, CRM-D19). The Arabic page sets
// `<html dir="rtl">`, so a style that names a physical side (left, right) is on the wrong side there, and a literal
// "→" between two things points backwards. This scan fails on a new physical left/right property in the UI's own
// styles and on a literal arrow in markup or code; the ranges and arrows go through `formatDateRange` and `arrow()`.
//
// Ported from the workforce planning management UI's `tests/unit/right-to-left.test.ts` (WPM-T377),
// whose rules and helpers the CRM's share in behaviour; the cases are WPM's, in the CRM's locales.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";

import {
  arrow,
  formatDateRange,
  formatDateTimeRange,
  OPEN_END,
} from "../../src/lib/locale-format";

const here = join(__dirname, "..", "..");

// "file:trimmed line" → why a physical side is right there. Empty today; add an entry only with a reason (a chart axis
// that is drawn left to right in every language, say), never to silence the scan.
const ALLOW: Record<string, string> = {};

const PHYSICAL: [string, RegExp][] = [
  [
    "a physical side property",
    /(?:^|[\s;{"'`])(?:(?:margin|padding|border|inset|scroll-margin|scroll-padding)-(?:left|right)(?:-[a-z]+)?|left|right|border-(?:top|bottom)-(?:left|right)-radius)\s*:/,
  ],
  [
    "a physical alignment or float",
    /\b(?:text-align|float|clear|justify-(?:content|items|self))\s*:\s*(?:left|right)\b/,
  ],
  [
    "a physical style directive",
    /\bstyle:(?:left|right|(?:margin|padding|border)-(?:left|right)(?:-[a-z]+)?)\b/,
  ],
];
const LITERAL_ARROW: [string, RegExp] = [
  'a literal "→" or "←" (use formatDateRange or arrow())',
  /[→←]/,
];

function walk(dir: string): string[] {
  const out: string[] = [];
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) out.push(...walk(path));
    else if (/\.(svelte|ts|css)$/.test(path)) out.push(path);
  }
  return out;
}

/** Physical sides and literal arrows in one file's text, as "rule: line". */
export function leftToRightOnly(text: string, file = ""): string[] {
  const found: string[] = [];
  for (const raw of text.split("\n")) {
    const line = raw.trim();
    if (/^(\/\/|\*|\/\*|<!--)/.test(line)) continue;
    if (ALLOW[`${file}:${line}`]) continue;
    for (const [name, re] of [...PHYSICAL, LITERAL_ARROW])
      if (re.test(line)) found.push(`${name}: ${line.slice(0, 120)}`);
  }
  return found;
}

describe("no physical left/right styles or literal arrows in src/", () => {
  it("finds them when they are there (the scan works)", () => {
    expect(leftToRightOnly("  text-align: left;")).toHaveLength(1);
    expect(leftToRightOnly("  border-left: 4px solid red;")).toHaveLength(1);
    expect(leftToRightOnly("  margin-right: 1rem;")).toHaveLength(1);
    expect(leftToRightOnly("  padding-left: 0.25rem;")).toHaveLength(1);
    expect(leftToRightOnly("  border-left-color: var(--x);")).toHaveLength(1);
    expect(leftToRightOnly("  left: 0;")).toHaveLength(1);
    expect(leftToRightOnly('<div style="right: 0">')).toHaveLength(1);
    expect(leftToRightOnly("  float: right;")).toHaveLength(1);
    expect(leftToRightOnly("<div style:margin-left={x}>")).toHaveLength(1);
    expect(leftToRightOnly("{a} → {b}")).toHaveLength(1);
    // …and not for the logical forms, field names or prose in comments.
    expect(leftToRightOnly("  text-align: start;")).toEqual([]);
    expect(leftToRightOnly("  border-inline-start: 4px solid;")).toEqual([]);
    expect(leftToRightOnly("  inset-inline-end: 0;")).toEqual([]);
    expect(leftToRightOnly("days_left: number;")).toEqual([]);
    expect(leftToRightOnly("const x = box.left;")).toEqual([]);
    expect(leftToRightOnly('<Drawer side="left">')).toEqual([]);
    expect(leftToRightOnly("// draft → active")).toEqual([]);
    expect(leftToRightOnly(" * draft → active")).toEqual([]);
  });

  it("finds none", () => {
    const offenders: string[] = [];
    for (const file of walk(join(here, "src"))) {
      // The generated API types (comments only) and the module that owns the arrow.
      if (/schema\.d\.ts$|locale-format\.ts$/.test(file)) continue;
      const rel = relative(here, file);
      for (const f of leftToRightOnly(readFileSync(file, "utf8"), rel))
        offenders.push(`${rel}: ${f}`);
    }
    expect(offenders, offenders.join("\n")).toEqual([]);
  });

  it("allows nothing without a reason", () => {
    for (const [line, reason] of Object.entries(ALLOW))
      expect(reason.trim(), line).not.toBe("");
  });
});

describe("ranges and arrows follow the page's direction", () => {
  it("writes a range of dates with the locale's own separator, never an arrow", () => {
    expect(
      formatDateRange("2026-09-01", "2026-09-30", "medium", "en-001"),
    ).toBe("1–30 Sept 2026");
    expect(
      formatDateRange("2026-09-01", "2026-10-30", "medium", "en-001"),
    ).toMatch(/^1 Sept\s–\s30 Oct 2026$/);
    expect(
      formatDateRange("2026-09-01", "2026-09-01", "medium", "en-001"),
    ).toBe("1 Sept 2026");
    for (const loc of ["ar-001"]) {
      const text = formatDateRange("2026-09-01", "2026-10-30", "medium", loc);
      expect(text).not.toMatch(/[→←]/);
      expect(text).toContain("–");
      expect(text).toBe(
        new Intl.DateTimeFormat(loc, {
          dateStyle: "medium",
          timeZone: "UTC",
        }).formatRange(
          new Date("2026-09-01T00:00:00Z"),
          new Date("2026-10-30T00:00:00Z"),
        ),
      );
    }
  });

  it("shows an end that is not known as an ellipsis, and text that is not a date as given", () => {
    expect(formatDateRange("2026-09-01", null, "medium", "en-001")).toBe(
      `1 Sept 2026 – ${OPEN_END}`,
    );
    expect(formatDateRange(undefined, "2026-09-30", "medium", "en-001")).toBe(
      `${OPEN_END} – 30 Sept 2026`,
    );
    expect(formatDateRange("soon", "later", "medium", "en-001")).toBe(
      "soon – later",
    );
    // An end before its start is not a range Intl will write; both ends are shown as they are.
    expect(
      formatDateRange("2026-10-01", "2026-09-01", "medium", "en-001"),
    ).toBe("1 Oct 2026 – 1 Sept 2026");
  });

  it("writes a shift as one date-and-time range", () => {
    const text = formatDateTimeRange(
      "2026-09-01T09:00:00Z",
      "2026-09-01T17:00:00Z",
      "en-001",
    );
    expect(text).not.toMatch(/[→←]/);
    expect(text).toContain("1 Sept 2026");
    expect(formatDateTimeRange("2026-09-01T09:00:00Z", null, "en-001")).toMatch(
      /– …$/,
    );
  });

  it("points an arrow along the writing direction", () => {
    expect(arrow("en-001")).toBe("→");
    expect(arrow("de-de")).toBe("→");
    expect(arrow("zh-cn")).toBe("→");
    expect(arrow("ar-001")).toBe("←");
  });
});
