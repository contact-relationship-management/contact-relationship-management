import { describe, expect, it } from "vitest";
import { fold, matches, words } from "#lib/search.js";

describe("site-wide search", () => {
  it("folds case and accents", () => {
    expect(fold("Zoë ÉCOLE")).toBe("zoe ecole");
  });

  it("splits a query into folded words and drops blanks", () => {
    expect(words("  Ada   LOVELACE ")).toEqual(["ada", "lovelace"]);
    expect(words("   ")).toEqual([]);
  });

  it("needs every word, in any order, across fields", () => {
    expect(matches(["Ada Lovelace", "Engineer"], "engineer ada")).toBe(true);
    expect(matches(["Ada Lovelace", "Engineer"], "ada grace")).toBe(false);
  });

  it("matches without accents or case", () => {
    expect(matches(["Zoë Brown"], "ZOE")).toBe(true);
  });

  it("ignores missing fields, and a blank query matches nothing", () => {
    expect(matches([null, undefined, 42, "open"], "42 open")).toBe(true);
    expect(matches(["anything"], "")).toBe(false);
  });
});
