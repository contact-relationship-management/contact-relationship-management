// The translation-review register (`spec/translation-register.md`) and the locales it describes:
// every locale file has a row and the counts are true; a locale marked reviewed names who and when;
// every locale has exactly English's keys outside the plural groups (a plural group holds its own
// language's forms, CRM-T173, checked by `plurals.test.ts`); and no string changes the
// {placeholders} English has.

import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import {
  DEFAULT_LOCALE,
  LOCALES,
  STRING_KEYS,
  isPluralForm,
  loadAllLocales,
} from "../../src/lib/i18n.svelte";

/** English's text for `key`, or for a plural form English lacks (`zero`, `few`…), its `other`. */
const englishFor = (en: Record<string, string>, key: string) =>
  en[key] ?? en[`${key.slice(0, key.lastIndexOf("."))}.other`];

// Vitest runs from the project folder.
const root = process.cwd();

interface Row {
  locale: string;
  status: string;
  reviewer: string;
  reviewedOn: string;
  strings: number;
  identical: number;
}

/** The register's table: the rows whose first cell is a backticked locale code. */
function register(): Row[] {
  const text = readFileSync(join(root, "spec/translation-register.md"), "utf8");
  return text
    .split("\n")
    .filter((line) => /^\| `[a-z]{2}-[a-z0-9]+` \|/.test(line))
    .map((line) => {
      const cells = line.split("|").map((c) => c.trim());
      const unquote = (c: string) => c.replace(/`/g, "");
      return {
        locale: unquote(cells[1]!),
        status: unquote(cells[3]!),
        reviewer: cells[5]!,
        reviewedOn: cells[6]!,
        strings: Number(cells[7]),
        identical: Number(cells[8]),
      };
    });
}

const placeholders = (text: string) =>
  [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]!).sort();

describe("the translation-review register", () => {
  it("has a row for every locale file and none for a file that is not there", () => {
    const files = readdirSync(join(root, "src/lib/locales"))
      .filter((name) => name.endsWith(".ts"))
      .map((name) => name.slice(0, -3))
      .sort();
    expect(register().map((r) => r.locale)).toEqual(files);
    expect(files).toEqual([...LOCALES].sort());
  });

  it("states a status that means something, and a reviewed one says who and when", () => {
    for (const row of register()) {
      expect(["source", "draft", "reviewed"], row.locale).toContain(row.status);
      expect(row.status === "source", row.locale).toBe(
        row.locale === DEFAULT_LOCALE,
      );
      if (row.status === "reviewed") {
        expect(row.reviewer, `${row.locale} has no reviewer`).not.toMatch(
          /^—?$/,
        );
        expect(row.reviewedOn, `${row.locale} has no date`).toMatch(
          /^\d{4}-\d{2}-\d{2}$/,
        );
      }
    }
  });

  it("counts the strings and the ones identical to English as they are", async () => {
    const tables = await loadAllLocales();
    const en = tables[DEFAULT_LOCALE];
    for (const row of register()) {
      const table = tables[row.locale as keyof typeof tables];
      const keys = Object.keys(table);
      const identical =
        row.locale === DEFAULT_LOCALE
          ? 0
          : keys.filter((key) => table[key] === en[key as keyof typeof en])
              .length;
      expect(row.strings, `${row.locale} strings`).toBe(keys.length);
      expect(row.identical, `${row.locale} identical to English`).toBe(
        identical,
      );
    }
  });
});

describe("the locales", () => {
  it("have exactly English's keys outside the plural groups, each with a value", async () => {
    const tables = await loadAllLocales();
    for (const locale of LOCALES) {
      const table = tables[locale] as Record<string, string>;
      const keys = Object.keys(table);
      expect(
        STRING_KEYS.filter((k) => !isPluralForm(k) && !keys.includes(k)),
        `${locale} lacks`,
      ).toEqual([]);
      expect(
        keys.filter(
          (k) => !isPluralForm(k) && !(STRING_KEYS as string[]).includes(k),
        ),
        `${locale} has keys English does not`,
      ).toEqual([]);
      expect(
        keys.filter((k) => !table[k]?.trim()),
        `${locale} has empty strings`,
      ).toEqual([]);
    }
  });

  it("keep English's {placeholders} in every string, no more and no fewer", async () => {
    const tables = await loadAllLocales();
    const en = tables[DEFAULT_LOCALE] as Record<string, string>;
    const drift: string[] = [];
    for (const locale of LOCALES) {
      const table = tables[locale] as Record<string, string>;
      for (const key of Object.keys(table)) {
        const want = placeholders(englishFor(en, key)!);
        const got = placeholders(table[key]!);
        if (want.join() !== got.join())
          drift.push(
            `${locale} ${key}: {${got.join(",")}} for {${want.join(",")}}`,
          );
      }
    }
    expect(drift).toEqual([]);
  });
});
