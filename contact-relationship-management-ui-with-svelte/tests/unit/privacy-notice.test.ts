// CRM-R97: docs/privacy-notice.md lists everything the UI and the site keep in the browser. A census of the source:
// every line in either project that writes a cookie or a storage item must be one of WRITES below, and every name in
// WRITES must be in the notice's tables, and the other way round. A new write therefore fails here until it is added
// to WRITES and to the notice. The site has no unit test runner, so its source is read from here.
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative, resolve as resolvePath } from "node:path";
import { describe, expect, it } from "vitest";

const ROOT = resolvePath(__dirname, "../../..");
const UI = "contact-relationship-management-ui-with-svelte/src";
const SITE = "contact-relationship-management.github.io/src";

// A line that writes, or names what a component writes. `storageKey: ""` keeps nothing and is not a write.
const WRITE =
  /\.setItem\(|document\.cookie\s*=|cookies\.set\(|storageKey\s*[:=](?!\s*"")|cookieName\s*[:=](?!\s*"")|keptChoice\("/;

/** Each write: the file, a piece of the line that identifies it, and the name the notice gives it. */
const WRITES: { file: string; line: string; name: string | null }[] = [
  // The helper writes the key its callers pass; the callers are listed with their names.
  {
    file: `${UI}/lib/kept-choice.ts`,
    line: "storage?.setItem(key, next)",
    name: null,
  },
  {
    file: `${SITE}/lib/kept-choice.ts`,
    line: "storage?.setItem(key, next)",
    name: null,
  },
  {
    file: `${UI}/routes/+layout.svelte`,
    line: 'keptChoice("lily-text-size")',
    name: "lily-text-size",
  },
  {
    file: `${UI}/routes/+layout.svelte`,
    line: "cookieName: THEME_COOKIE",
    name: "crm-theme",
  },
  {
    file: `${UI}/lib/i18n.svelte.ts`,
    line: "localStorage.setItem(LOCALE_KEY",
    name: "mxi.crm.locale",
  },
  {
    file: `${UI}/lib/components/DataTable.svelte`,
    line: "localStorage.setItem(orderKey",
    name: "crm-grid-order-<table>",
  },
  {
    file: `${UI}/lib/components/DataTable.svelte`,
    line: "storageKey={testid ? `crm-grid-${testid}`",
    name: "crm-grid-<table>",
  },
  {
    file: `${UI}/routes/signin/+page.server.ts`,
    line: "cookies.set(SESSION_COOKIE",
    name: "__Host-mxi_session",
  },
  {
    file: `${UI}/routes/tour/+page.server.ts`,
    line: "cookies.set(SESSION_COOKIE",
    name: "__Host-mxi_session",
  },
  {
    file: `${UI}/routes/verify/+page.server.ts`,
    line: "cookies.set(SESSION_COOKIE",
    name: "__Host-mxi_session",
  },
  {
    file: `${SITE}/routes/+layout.svelte`,
    line: 'keptChoice("lily-text-size")',
    name: "lily-text-size",
  },
  {
    file: `${SITE}/routes/+layout.svelte`,
    line: 'keptChoice("lily-time-zone")',
    name: "lily-time-zone",
  },
  {
    file: `${SITE}/routes/+layout.svelte`,
    line: 'cookieName: "lily-theme"',
    name: "lily-theme",
  },
  {
    file: `${SITE}/routes/+layout.svelte`,
    line: 'storageKey: "lily-theme"',
    name: "lily-theme",
  },
];

// The constants and templates the entries above stand for, so a renamed key fails here too.
const DEFINITIONS: { file: string; text: string }[] = [
  { file: `${UI}/lib/theme.ts`, text: 'THEME_COOKIE = "crm-theme"' },
  { file: `${UI}/lib/i18n.svelte.ts`, text: 'LOCALE_KEY = "mxi.crm.locale"' },
  {
    file: `${UI}/lib/components/DataTable.svelte`,
    text: "`crm-grid-order-${testid}`",
  },
  {
    file: `${UI}/lib/server/session.ts`,
    text: 'SESSION_COOKIE = "__Host-mxi_session"',
  },
];

function sources(dir: string): string[] {
  return readdirSync(join(ROOT, dir)).flatMap((entry) => {
    const path = join(dir, entry);
    if (statSync(join(ROOT, path)).isDirectory()) return sources(path);
    return /\.(ts|js|svelte)$/.test(entry) ? [path] : [];
  });
}

/** The names in the first column of the notice's tables. */
function noticeNames(): string[] {
  const text = readFileSync(join(ROOT, "docs/privacy-notice.md"), "utf8");
  return [...text.matchAll(/^\| `([^`]+)` \|/gm)].map((m) => m[1]!);
}

describe("what the privacy notice lists (CRM-R97)", () => {
  it("covers every line in the UI and the site that writes a cookie or a storage item", () => {
    const unlisted: string[] = [];
    for (const file of [...sources(UI), ...sources(SITE)]) {
      readFileSync(join(ROOT, file), "utf8")
        .split("\n")
        .forEach((text, i) => {
          if (!WRITE.test(text) || text.trim().startsWith("//")) return;
          if (
            !WRITES.some(
              (w) =>
                w.file === relative(ROOT, join(ROOT, file)) &&
                text.includes(w.line),
            )
          )
            unlisted.push(`${file}:${i + 1}: ${text.trim()}`);
        });
    }
    expect(
      unlisted,
      "add each to WRITES and to docs/privacy-notice.md",
    ).toEqual([]);
  });

  it("has every write's line still in its file, and the keys they stand for", () => {
    for (const w of WRITES)
      expect(
        readFileSync(join(ROOT, w.file), "utf8"),
        `${w.file}: ${w.line}`,
      ).toContain(w.line);
    for (const d of DEFINITIONS)
      expect(
        readFileSync(join(ROOT, d.file), "utf8"),
        `${d.file}: ${d.text}`,
      ).toContain(d.text);
  });

  it("names exactly what is written", () => {
    const written = [
      ...new Set(WRITES.flatMap((w) => (w.name ? [w.name] : []))),
    ].sort();
    expect([...new Set(noticeNames())].sort()).toEqual(written);
  });
});
