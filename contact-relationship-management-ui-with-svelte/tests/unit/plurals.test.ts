// Plural rules (CRM-T173; spec/worldwide.md code row 36, CRM-R95, CRM-D19): a string that fills a
// count is a plural group chosen with `Intl.PluralRules` in the page's locale, and every locale's
// file holds exactly the CLDR categories its language uses.
//
// Ported from the workforce planning management UI's `tests/unit/plurals.test.ts` (WPM-T374), whose
// helpers the CRM's share in behaviour; the cases are WPM's, in the CRM's eight locales.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { beforeAll, describe, expect, it } from "vitest";

import {
  LOCALES,
  PLURAL_CATEGORIES,
  PLURAL_KEYS,
  fill,
  loadAllLocales,
  pluralCategories,
  pluralCategory,
  translatePlural,
  type Locale,
} from "../../src/lib/i18n.svelte";

const KEY = "kanban.cards" as const;
let tables: Record<Locale, Record<string, string>>;

beforeAll(async () => {
  tables = await loadAllLocales();
});

describe("plural selection", () => {
  it("English has two forms: one and other", () => {
    expect(pluralCategory(1, "en-001")).toBe("one");
    expect(pluralCategory(0, "en-001")).toBe("other");
    expect(pluralCategory(2, "en-001")).toBe("other");
    expect(translatePlural(KEY, 1, "en-001")).toBe("{count} card");
    expect(translatePlural(KEY, 5, "en-001")).toBe("{count} cards");
  });

  it("Arabic has six: zero, one, two, few, many, other", () => {
    expect(pluralCategories("ar-001")).toEqual([...PLURAL_CATEGORIES]);
    const forms = [0, 1, 2, 3, 11, 100].map((n) => pluralCategory(n, "ar-001"));
    expect(forms).toEqual(["zero", "one", "two", "few", "many", "other"]);
    const texts = [0, 1, 2, 3, 11, 100].map((n) =>
      translatePlural(KEY, n, "ar-001"),
    );
    expect(new Set(texts).size).toBe(6);
  });

  it("Welsh has six, like Arabic but with its own rules", () => {
    expect([0, 1, 2, 3, 6, 4].map((n) => pluralCategory(n, "cy-001"))).toEqual([
      "zero",
      "one",
      "two",
      "few",
      "many",
      "other",
    ]);
    expect(translatePlural(KEY, 2, "cy-001")).toBe("{count} gerdyn");
    expect(translatePlural(KEY, 3, "cy-001")).toBe("{count} cherdyn");
  });

  it("Spanish and French have a many form for a million", () => {
    for (const locale of ["es-001", "fr-001"] as const) {
      expect(pluralCategories(locale)).toEqual(["one", "many", "other"]);
      expect(pluralCategory(1000000, locale)).toBe("many");
    }
    expect(translatePlural(KEY, 1000000, "fr-001")).toBe("{count} de cartes");
    // French takes the singular for 0 and 1; Spanish only for 1.
    expect(translatePlural(KEY, 0, "fr-001")).toBe("{count} carte");
    expect(translatePlural(KEY, 0, "es-001")).toBe("{count} tarjetas");
  });

  it("Hindi takes its one form for 0 and 1; German for 1 only", () => {
    expect(pluralCategory(0, "hi-001")).toBe("one");
    expect(pluralCategory(0, "de-de")).toBe("other");
    expect(translatePlural("grid.exported", 1, "de-de")).toBe(
      "{count} Zeile nach {filename} exportiert",
    );
  });

  it("Chinese has one form for every count", () => {
    expect(pluralCategories("zh-cn")).toEqual(["other"]);
    expect(translatePlural(KEY, 1, "zh-cn")).toBe(
      translatePlural(KEY, 7, "zh-cn"),
    );
    expect(translatePlural(KEY, 1, "zh-cn")).toBe("{count} 张卡片");
  });

  it("fills a count in the page's number format", () => {
    expect(fill("{count} Karten", { count: 1234 }, "de-de")).toBe(
      "1.234 Karten",
    );
    expect(fill("{count} cards", { count: 1234 }, "en-001")).toBe(
      "1,234 cards",
    );
    expect(fill("{name}", { name: "x" }, "en-001")).toBe("x");
    expect(fill("{other}", {}, "en-001")).toBe("{other}");
  });

  it("no string writes (s), and no count is chosen by `=== 1` in src/", () => {
    for (const locale of LOCALES)
      for (const [key, text] of Object.entries(tables[locale]))
        expect(text, `${locale} ${key}`).not.toMatch(/\w\(s\)/);
    const here = join(__dirname, "..", "..");
    const walk = (dir: string): string[] =>
      readdirSync(dir).flatMap((name) => {
        const path = join(dir, name);
        return statSync(path).isDirectory()
          ? walk(path)
          : /\.(svelte|ts)$/.test(path)
            ? [path]
            : [];
      });
    const offenders = walk(join(here, "src"))
      .filter((f) => !/server[/\\]demo\.ts$/.test(f))
      .flatMap((f) =>
        readFileSync(f, "utf8")
          .split("\n")
          .filter((line) =>
            /\b(?:count|length|total)\s*===\s*1\s*\?\s*t\(/.test(line),
          )
          .map((line) => `${relative(here, f)}: ${line.trim()}`),
      );
    expect(offenders).toEqual([]);
  });
});

describe("every plural group has the forms its locale needs", () => {
  it("finds the plural groups", () => {
    expect(PLURAL_KEYS.length).toBeGreaterThanOrEqual(10);
    expect(PLURAL_KEYS).toContain("kanban.cards");
  });

  for (const locale of LOCALES) {
    it(`${locale} holds exactly ${pluralCategories(locale).join(", ")}`, () => {
      const table = tables[locale];
      const need = pluralCategories(locale);
      const problems: string[] = [];
      for (const base of PLURAL_KEYS) {
        const have = PLURAL_CATEGORIES.filter(
          (c) => table[`${base}.${c}`] !== undefined,
        );
        if (have.join() !== need.join())
          problems.push(
            `${base}: has ${have.join(",")}, needs ${need.join(",")}`,
          );
        for (const c of have)
          if (!table[`${base}.${c}`]?.trim())
            problems.push(`${base}.${c} is empty`);
      }
      expect(problems, problems.join("\n")).toEqual([]);
    });
  }
});
