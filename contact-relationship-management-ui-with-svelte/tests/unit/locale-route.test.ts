import { describe, expect, it } from "vitest";
import { ROUTE_LOCALES, localeOf, splitLocale } from "../../src/params";

const match = (segment: string) => localeOf(segment) !== undefined;
import { LOCALES } from "../../src/lib/i18n.svelte";

describe("the /<locale>/ route matcher", () => {
  it("lists exactly the UI's languages (params.ts cannot import them)", () => {
    expect([...ROUTE_LOCALES]).toEqual([...LOCALES]);
  });

  it("accepts every language the UI is translated into", () => {
    for (const locale of LOCALES) expect(match(locale), locale).toBe(true);
  });

  it("refuses anything else", () => {
    for (const other of ["xx", "en", "en-GB", "EN-001", "signin", "kanban", ""]) {
      expect(match(other), other).toBe(false);
    }
  });
});

describe("a language in the address", () => {
  it("splits the prefix from the path", () => {
    expect(splitLocale("/de-de/contacts")).toEqual({ locale: "de-de", path: "/contacts" });
    expect(splitLocale("/de-de/contacts/con-1")).toEqual({ locale: "de-de", path: "/contacts/con-1" });
    expect(splitLocale("/zh-cn")).toEqual({ locale: "zh-cn", path: "/" });
  });

  it("leaves an unprefixed path alone", () => {
    expect(splitLocale("/contacts")).toEqual({ path: "/contacts" });
    expect(splitLocale("/")).toEqual({ path: "/" });
    expect(splitLocale("/xx/contacts")).toEqual({ path: "/xx/contacts" });
  });
});
