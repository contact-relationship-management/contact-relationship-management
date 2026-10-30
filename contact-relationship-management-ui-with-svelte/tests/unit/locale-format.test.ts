// Numbers, percentages and dates in the page's locale (CRM-T172; spec/worldwide.md code rows 31 to 35,
// CRM-R95, CRM-D19), and a scan of `src/` that fails on new hand-rolled formatting: `toFixed`,
// `toLocale*`, a percent sign glued to a number, a timestamp sliced to a date, an ISO date field put
// straight into markup or a placeholder.
//
// Ported from the workforce planning management UI's `tests/unit/locale-format.test.ts` (WPM-T375),
// whose module this one shares in behaviour; the cases are WPM's, in the CRM's locales.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";

import {
  DASH,
  formatBasisPoints,
  formatDate,
  formatDateTime,
  formatDecimal,
  formatInteger,
  formatMonth,
  formatNumber,
  formatPercent,
  formatPercentValue,
  formatSigned,
  formatTime,
  numberToInput,
  parseNumber,
  parseScaled,
} from "../../src/lib/locale-format";
import { ratioLabel, ratioPercent, stageLabel } from "../../src/lib/format";

const NBSP = "\u00a0";
const NNBSP = "\u202f";

describe("numbers in the page's locale", () => {
  it("groups and marks decimals the locale's way, in its own digits", () => {
    expect(formatNumber(1234.5, {}, "en-001")).toBe("1,234.5");
    expect(formatNumber(1234.5, {}, "de-de")).toBe("1.234,5");
    expect(formatNumber(1234.5, {}, "fr-001")).toBe(`1${NNBSP}234,5`);
    expect(formatNumber(1234567, {}, "hi-001")).toBe("12,34,567");
    expect(formatNumber(1234.5, {}, "bn-001")).toBe("১,২৩৪.৫");
  });

  it("shows an unknown figure as a dash, never 0", () => {
    expect(formatNumber(null)).toBe(DASH);
    expect(formatNumber(undefined)).toBe(DASH);
    expect(formatNumber(Number.NaN)).toBe(DASH);
    expect(formatNumber(0, {}, "en-001")).toBe("0");
    expect(formatPercent(null)).toBe(DASH);
    expect(formatBasisPoints(null)).toBe(DASH);
  });

  it("writes fixed decimals, integers and signs", () => {
    expect(formatDecimal(2.5, 2, "en-001")).toBe("2.50");
    expect(formatDecimal(2.5, 2, "de-de")).toBe("2,50");
    expect(formatInteger(1234.4, "en-001")).toBe("1,234");
    expect(formatSigned(3, {}, "en-001")).toBe("+3");
    expect(formatSigned(-1.5, {}, "en-001")).toBe("-1.5");
    expect(formatSigned(0, {}, "en-001")).toBe("0");
  });

  it("writes a percentage the locale's way, sign and space included", () => {
    expect(formatPercent(0.25, 0, "en-001")).toBe("25%");
    expect(formatPercent(0.25, 0, "de-de")).toBe(`25${NBSP}%`);
    expect(formatPercent(0.25, 0, "fr-001")).toBe(`25${NBSP}%`);
    expect(formatPercentValue(8.3, 1, "en-001")).toBe("8.3%");
    expect(formatPercentValue(8.3, 1, "de-de")).toBe(`8,3${NBSP}%`);
  });

  it("places the point of basis points exactly", () => {
    expect(formatBasisPoints(10456, 2, "en-001")).toBe("104.56%");
    expect(formatBasisPoints(-5, 2, "en-001")).toBe("-0.05%");
    expect(formatBasisPoints(909, 2, "de-de")).toBe(`9,09${NBSP}%`);
    expect(formatBasisPoints(250, 0, "en-001")).toBe("3%");
  });

  it("is what the ratio and stage helpers in format.ts now write", () => {
    const half = { numerator: 1, denominator: 2, value: 0.5 };
    expect(ratioPercent(half, "en-001")).toBe("50%");
    expect(ratioPercent(half, "fr-001")).toBe(`50${NBSP}%`);
    expect(ratioLabel(half, DASH, "de-de")).toBe(`50${NBSP}% (1/2)`);
    expect(
      stageLabel({ name: "Won", probability_percent: 100 }, "en-001"),
    ).toBe("Won 100%");
    expect(
      stageLabel({ name: "Gagné", probability_percent: 40 }, "fr-001"),
    ).toBe(`Gagné 40${NBSP}%`);
  });
});

describe("dates in the page's locale", () => {
  it("writes an ISO date the locale's way, the same day in every time zone", () => {
    expect(formatDate("2026-10-30", "medium", "en-001")).toBe("30 Oct 2026");
    expect(formatDate("2026-10-30", "medium", "en-us")).toBe("Oct 30, 2026");
    expect(formatDate("2026-10-30", "medium", "de-de")).toBe("30.10.2026");
    expect(formatDate("2026-10-30", "medium", "zh-cn")).toBe("2026年10月30日");
    expect(formatDate("2026-01-01", "medium", "en-001")).toBe("1 Jan 2026");
  });

  it("shows a missing date as a dash and leaves text that is not a date alone", () => {
    expect(formatDate(null)).toBe(DASH);
    expect(formatDate("")).toBe(DASH);
    expect(formatDate("soon")).toBe("soon");
    expect(formatDate("2026-13-45", "medium", "en-001")).toBe("2026-13-45");
  });

  it("writes a timestamp as a date, a date and time, or a time", () => {
    const at = "2026-10-30T12:00:00Z";
    expect(formatDate(at, "medium", "en-001")).toMatch(/Oct 2026$/);
    expect(formatDateTime(at, "en-001")).toMatch(/2026.*\d{1,2}[:.]\d{2}/);
    expect(formatTime(at, "en-001")).toMatch(/^\d{1,2}[:.]\d{2}/);
    // A civil date has no time of day.
    expect(formatDateTime("2026-10-30", "en-001")).toBe("30 Oct 2026");
  });

  it("writes a timestamp in the page's locale, not the browser's", () => {
    const at = "2026-10-30T12:00:00Z";
    expect(formatDateTime(at, "de-de")).toMatch(/^30\.10\.2026/);
    expect(formatDateTime(at, "zh-cn")).toMatch(/^2026年10月30日/);
  });

  it("writes a month as a month and year", () => {
    expect(formatMonth("2026-10", "short", "en-001")).toBe("Oct 2026");
    expect(formatMonth("2026-10-01", "long", "en-001")).toBe("October 2026");
    expect(formatMonth("2026-10", "long", "de-de")).toBe("Oktober 2026");
  });
});

describe("numbers typed in the page's locale", () => {
  it("reads the locale's decimal mark, through money.ts's parser", () => {
    expect(parseNumber("2,5", "de-de")).toBe(2.5);
    expect(parseNumber("2.5", "en-001")).toBe(2.5);
    expect(parseNumber("1.234,5", "de-de")).toBe(1234.5);
    expect(parseNumber("2,5", "fr-001")).toBe(2.5);
    expect(parseNumber("২.৫", "bn-001")).toBe(2.5);
    expect(parseNumber("two", "en-001")).toBeNull();
    expect(parseNumber("", "en-001")).toBeNull();
  });

  it("scales a typed percentage to the API's basis points without float error", () => {
    expect(parseScaled("2,5", 2, "de-de")).toBe(250);
    expect(parseScaled("0.8", 2, "en-001")).toBe(80);
    // 1.005 * 100 is 100.49999…; "1.005e2" is exactly 100.5.
    expect(parseScaled("1.005", 2, "en-001")).toBe(101);
    expect(parseScaled("x", 2, "en-001")).toBeNull();
  });

  it("writes a number back for a text input with no grouping", () => {
    expect(numberToInput(2.5, "de-de")).toBe("2,5");
    expect(numberToInput(1234.5, "en-001")).toBe("1234.5");
    expect(numberToInput(null)).toBe("");
  });
});

// The scan. A line that matches is hand-rolled formatting of something a person reads. Allowed:
// comments, CSS, and a line in ALLOW with its reason.
const here = join(__dirname, "..", "..");
const ALLOW: Record<string, string> = {
  "src/routes/notifications/+page.svelte:<time datetime={n.created_at}>{when(n.created_at)}</time>":
    "the ISO timestamp is the machine-readable `datetime` attribute; the text shown is `when()`, which is `formatDateTime`",
};
const FIELD = String.raw`[\w.?]*?(?:\w+_(?:on|at|date|start|end|until|from|due|day)\b|as_of|\.date\b|\.on\b|\.month\b)`;
const RULES: [string, RegExp][] = [
  ["toFixed", /\.toFixed\(/],
  ["toLocale*", /\.toLocale(?:String|DateString|TimeString)\(/],
  ["a percent sign glued to a number", /(?:\$\{[^}]*\}|\}|\))%(?![\w;"])/],
  [
    "a timestamp sliced to a date",
    /\b\w+(?:_at|month)\b[^;\n]*?\.slice\(0, (?:7|10|16|19)\)/,
  ],
  [
    "an ISO date in markup",
    new RegExp(String.raw`\{\s*${FIELD}\s*(?:\?\?[^}]*)?\}`),
  ],
  [
    "an ISO date in a template string",
    new RegExp(String.raw`\$\{\s*${FIELD}\s*\}`),
  ],
];

function walk(dir: string): string[] {
  const out: string[] = [];
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) out.push(...walk(path));
    else if (/\.(svelte|ts)$/.test(path)) out.push(path);
  }
  return out;
}

/** Hand-rolled formatting in one file's text, as "rule: line". */
export function handRolled(text: string, file = ""): string[] {
  const found: string[] = [];
  for (const raw of text.split("\n")) {
    const line = raw.trim();
    if (/^(\/\/|\*|\/\*|<!--)/.test(line)) continue;
    if (/style=|width:|height:|type="date"/.test(line)) continue;
    if (ALLOW[`${file}:${line}`]) continue;
    for (const [name, re] of RULES)
      if (re.test(line)) found.push(`${name}: ${line.slice(0, 120)}`);
  }
  return found;
}

describe("no hand-rolled number or date formatting in src/", () => {
  it("finds it when it is there (the scan works)", () => {
    expect(handRolled("const x = (n / 100).toFixed(2);")).toHaveLength(1);
    expect(handRolled("const s = `${Math.round(v * 100)}%`;")).toHaveLength(1);
    expect(handRolled("<td>{row.close_on}</td>")).toHaveLength(1);
    expect(handRolled("{a.occurred_at.slice(0, 16)}")).toHaveLength(1);
    expect(handRolled("new Date(x).toLocaleString()")).toHaveLength(1);
    expect(handRolled("new Date(x).toLocaleTimeString([])")).toHaveLength(1);
    // …and not when the value goes through the module.
    expect(handRolled("<td>{formatDate(row.close_on)}</td>")).toEqual([]);
    expect(handRolled('<div style="width: {pct}%"></div>')).toEqual([]);
  });

  it("finds none", () => {
    const offenders: string[] = [];
    for (const file of walk(join(here, "src"))) {
      // The two formatting modules; the generated API types (comments only); the tour's
      // stand-in for the service, which writes the API's own ISO values, not text a person reads.
      if (
        /locale-format\.ts$|money\.ts$|schema\.d\.ts$|server[/\\]demo\.ts$/.test(
          file,
        )
      )
        continue;
      const rel = relative(here, file);
      for (const f of handRolled(readFileSync(file, "utf8"), rel))
        offenders.push(`${rel}: ${f}`);
    }
    expect(offenders, offenders.join("\n")).toEqual([]);
  });

  it("allows nothing without a reason", () => {
    for (const [line, reason] of Object.entries(ALLOW))
      expect(reason.trim(), line).not.toBe("");
  });
});
