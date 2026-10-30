// CRM-T167 and CRM-T168: one money formatter and parser. No currency is built in, the minor unit is
// the currency's own (JPY 0, EUR 2, KWD 3, CLF 4) from the same ISO 4217 table as the service, and an
// amount is read in the reader's own number format. The cases follow the sibling apps'
// tests/unit/money.test.ts (project portfolio management and workforce planning management, WPM-T373),
// so the three stay alike, in the CRM's locale codes.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  MINOR_UNITS,
  currencyCode,
  exponentOf,
  formMinor,
  majorToMinor,
  minorToInput,
  minorToMajor,
  money,
  parseAmount,
  signedMoney,
  toMinor,
} from "../../src/lib/money";
import { LOCALES } from "../../src/lib/i18n.svelte";
import { PageError } from "../../src/lib/errors";

describe("exponentOf", () => {
  it("is the currency's own minor unit", () => {
    expect(exponentOf("JPY", "en-001")).toBe(0);
    expect(exponentOf("EUR", "en-001")).toBe(2);
    expect(exponentOf("KWD", "en-001")).toBe(3);
    expect(exponentOf("CLF", "en-001")).toBe(4);
  });
  it("is the same in every shipped locale", () => {
    for (const locale of LOCALES) {
      expect(exponentOf("JPY", locale), locale).toBe(0);
      expect(exponentOf("KWD", locale), locale).toBe(3);
      expect(exponentOf("CLF", locale), locale).toBe(4);
    }
  });
  it("follows ISO 4217, not CLDR, where the two differ, as the service does", () => {
    // CLDR (and so Intl) shows the forint and the rupiah with no decimals; ISO 4217 gives them 2,
    // and the Iraqi dinar 3. The service converts by ISO 4217, so the UI types and shows by it too.
    expect(exponentOf("HUF", "en-001")).toBe(2);
    expect(exponentOf("IDR", "en-001")).toBe(2);
    expect(exponentOf("IQD", "en-001")).toBe(3);
    expect(money(150_000, "HUF", "en-001")).toContain("1,500.00");
  });
  it("falls back to Intl, then 2, for a code the table does not hold", () => {
    expect(exponentOf("XAU", "en-001")).toBe(
      new Intl.NumberFormat("en-001", {
        style: "currency",
        currency: "XAU",
      }).resolvedOptions().maximumFractionDigits,
    );
    expect(exponentOf("not a code", "en-001")).toBe(2);
  });
});

describe("the ISO 4217 table", () => {
  it("is exactly the service's (src/rules/currency.rs), so both scale by the same exponent", () => {
    const rust = readFileSync(
      join(
        process.cwd(),
        "../contact-relationship-management-service-with-rust/src/rules/currency.rs",
      ),
      "utf8",
    );
    const table = rust.slice(
      rust.indexOf("pub const MINOR_UNITS"),
      rust.indexOf("];"),
    );
    const service = Object.fromEntries(
      [...table.matchAll(/\("([A-Z]{3})", (\d)\)/g)].map((m) => [
        m[1]!,
        Number(m[2]),
      ]),
    );
    expect(Object.keys(service).length).toBeGreaterThan(150);
    expect(MINOR_UNITS).toEqual(service);
  });
});

describe("money", () => {
  it("scales by the currency's exponent, not by 100", () => {
    expect(money(123_456, "JPY", "en-001")).toBe("JP¥123,456");
    expect(money(1_234_567, "KWD", "en-001")).toContain("1,234.567");
    expect(money(123_450, "EUR", "en-001")).toBe("€1,234.50");
    expect(money(12_345, "CLF", "en-001")).toContain("1.2345");
  });
  it("uses the reader's own separators", () => {
    expect(money(123_450, "EUR", "de-de")).toMatch(/^1\.234,50\s€$/u);
    expect(money(123_450, "EUR", "fr-001")).toMatch(/^1\s234,50\s€$/u);
    expect(money(1500, "JPY", "de-de")).toMatch(/^1\.500\s¥$/u);
    expect(money(1_234_567, "KWD", "de-de")).toMatch(/^1\.234,567\s/u);
  });
  it("renders an unknown amount or a missing currency as an em dash, never zero", () => {
    expect(money(null, "EUR", "en-001")).toBe("—");
    expect(money(undefined, "EUR", "en-001")).toBe("—");
    expect(money(100, null, "en-001")).toBe("—");
    expect(money(100, "", "en-001")).toBe("—");
  });
  it("shows the sign of a change", () => {
    expect(signedMoney(1000, "JPY", "en-001")).toBe("+JP¥1,000");
    expect(signedMoney(-5, "EUR", "en-001")).toBe("-€0.05");
    expect(signedMoney(0, "EUR", "en-001")).toBe("€0.00");
  });
});

describe("parseAmount and toMinor", () => {
  it("reads a decimal comma where the locale uses one", () => {
    expect(parseAmount("1,5", "de-de")).toBe(1.5);
    expect(parseAmount("12,50", "de-de")).toBe(12.5);
    expect(parseAmount("1 234,50", "fr-001")).toBe(1234.5);
    expect(parseAmount("1.234,50", "de-de")).toBe(1234.5);
    expect(parseAmount("1.234,50", "es-001")).toBe(1234.5);
    expect(toMinor("1 234,50", "EUR", "fr-001")).toBe(123_450);
    expect(toMinor("1234,50", "EUR", "de-de")).toBe(123_450);
  });
  it("reads a decimal point where the locale uses one", () => {
    expect(parseAmount("1,234.50", "en-001")).toBe(1234.5);
    expect(parseAmount("1234.5", "en-001")).toBe(1234.5);
    expect(parseAmount("12,34,567.5", "hi-001")).toBe(1_234_567.5);
  });
  it("never turns text into a number", () => {
    expect(parseAmount("", "en-001")).toBeNull();
    expect(parseAmount("abc", "en-001")).toBeNull();
    expect(parseAmount("1.2.3", "en-001")).toBeNull();
    expect(parseAmount("12 EUR", "en-001")).toBeNull();
  });
  it("scales by the exponent without float error", () => {
    expect(toMinor("1500", "JPY", "en-001")).toBe(1500);
    expect(toMinor("1.234", "KWD", "en-001")).toBe(1234);
    expect(toMinor("1.005", "EUR", "en-001")).toBe(101);
    expect(toMinor("0,5", "EUR", "de-de")).toBe(50);
    expect(toMinor("1.2345", "CLF", "en-001")).toBe(12345);
    expect(toMinor("x", "EUR", "en-001")).toBeNull();
  });
});

describe("minorToInput", () => {
  it("is what the same locale reads back", () => {
    for (const [minor, cur, loc] of [
      [150_050, "EUR", "de-de"],
      [1500, "JPY", "en-001"],
      [1_234_567, "KWD", "fr-001"],
      [12_345, "CLF", "en-001"],
      [150_050, "EUR", "es-001"],
      [150_050, "INR", "hi-001"],
    ] as const) {
      const text = minorToInput(minor, cur, loc);
      expect(toMinor(text, cur, loc), `${cur} ${loc} ${text}`).toBe(minor);
    }
    expect(minorToInput(150_050, "EUR", "de-de")).toBe("1500,50");
    expect(minorToInput(1500, "JPY", "en-001")).toBe("1500");
  });
});

describe("minorToMajor and majorToMinor", () => {
  it("move between minor units and a plain number by the exponent, exactly", () => {
    expect(minorToMajor(1500, "JPY", "en-001")).toBe(1500);
    expect(minorToMajor(150_050, "EUR", "en-001")).toBe(1500.5);
    expect(minorToMajor(1_234_567, "KWD", "en-001")).toBe(1234.567);
    expect(majorToMinor(1500, "JPY", "en-001")).toBe(1500);
    expect(majorToMinor(1.005, "EUR", "en-001")).toBe(101);
    expect(majorToMinor(1.2345, "CLF", "en-001")).toBe(12_345);
    expect(majorToMinor(Number.NaN, "EUR", "en-001")).toBeNull();
  });
});

describe("formMinor", () => {
  it("needs a currency and a number in the page's format, and assumes neither", () => {
    expect(formMinor("1234,50", "eur", "de-de")).toEqual({
      minor: 123_450,
      currency: "EUR",
    });
    expect(formMinor("1500", "JPY", "en-001")).toEqual({
      minor: 1500,
      currency: "JPY",
    });
    expect(() => formMinor("10", "", "en-001")).toThrow(PageError);
    expect(() => formMinor("10", undefined, "en-001")).toThrow(PageError);
    expect(() => formMinor("ten", "EUR", "en-001")).toThrow(PageError);
  });
});

describe("currencyCode", () => {
  it("is three letters, upper-cased, and nothing is assumed", () => {
    expect(currencyCode("jpy")).toBe("JPY");
    expect(currencyCode(" EUR ")).toBe("EUR");
    expect(currencyCode("")).toBeNull();
    expect(currencyCode("EU")).toBeNull();
    expect(currencyCode("E1R")).toBeNull();
  });
});

describe("no built-in currency and no fixed 100 in the UI (CRM-T167, CRM-T168)", () => {
  const SRC = join(__dirname, "../../src");
  const files = (dir: string): string[] =>
    readdirSync(dir).flatMap((name) => {
      const p = join(dir, name);
      if (statSync(p).isDirectory()) return files(p);
      return /\.(ts|svelte)$/.test(name) && !name.endsWith(".d.ts") ? [p] : [];
    });
  const sources = files(SRC).map(
    (p) => [p.slice(SRC.length + 1), readFileSync(p, "utf8")] as const,
  );

  it("no form starts with a currency", () => {
    for (const [file, text] of sources) {
      expect(text, file).not.toMatch(
        /\$state(<[^>]*>)?\(\s*["'][A-Z]{3}["']\s*\)/,
      );
    }
    const read = (p: string) => readFileSync(join(SRC, p), "utf8");
    expect(read("routes/quotas/+page.svelte")).toMatch(
      /let currency = \$state\(""\)/,
    );
    expect(read("routes/campaigns/+page.svelte")).toMatch(
      /let currency = \$state\(""\)/,
    );
    expect(read("routes/fx-rates/+page.svelte")).toMatch(
      /let from = \$state\(""\)/,
    );
  });

  it("nothing falls back to a currency", () => {
    // Three capital letters after `??` or `||` (an HTTP method such as "GET" is not a currency).
    const fallback = /(\?\?|\|\|)\s*["']([A-Z]{3})["']/g;
    for (const [file, text] of sources) {
      const codes = [...text.matchAll(fallback)].map((m) => m[2]!);
      expect(
        codes.filter((c) => c in MINOR_UNITS),
        file,
      ).toEqual([]);
    }
  });

  it("no file divides or multiplies money by a fixed 100", () => {
    for (const [file, text] of sources) {
      expect(text, file).not.toMatch(
        /(minor|pounds|pence|cost|amount|revenue)\w*\)? ?[*/%] ?100\b/i,
      );
      expect(text, file).not.toMatch(
        /\b100 ?\* ?(minor|pounds|cost|amount|revenue)/i,
      );
    }
  });

  it("formats money in one place: no hand-made currency formatter", () => {
    for (const [file, text] of sources) {
      if (file === "lib/money.ts") continue;
      expect(text, file).not.toMatch(/style:\s*["']currency["']/);
    }
  });
});
