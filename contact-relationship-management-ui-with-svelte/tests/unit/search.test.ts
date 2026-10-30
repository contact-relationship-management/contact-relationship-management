import { describe, expect, it } from "vitest";
import { fold, longEnough, matches, words } from "#lib/search.js";

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

describe("search in every script (CRM-T170)", () => {
  it("keeps the marks of scripts other than Latin, Greek and Cyrillic", () => {
    expect(fold("Ἀθῆναι Ёлка")).toBe("αθηναι елка");
    expect(fold("प्रिया")).toBe("प्रिया");
    expect(fold("สมศรี")).toBe("สมศรี");
    expect(fold("Nguyễn")).toBe("nguyen");
  });

  it("finds part of a name written without spaces", () => {
    expect(matches(["王芳"], "芳")).toBe(true);
    expect(matches(["北京王芳科技有限公司"], "科技")).toBe(true);
    expect(matches(["田中 博"], "田中")).toBe(true);
    expect(matches(["สมศรี ใจดี"], "ศรี")).toBe(true);
    expect(matches(["김민준"], "민준")).toBe(true);
    expect(matches(["प्रिया वेंकटरमन"], "वेंकट")).toBe(true);
    // A vowel sign is part of the word: प्र is not पर.
    expect(matches(["परम"], "प्र")).toBe(false);
  });

  it("takes one ideograph as long enough, as the service does, but not one letter", () => {
    expect(longEnough("芳")).toBe(true);
    expect(longEnough(" か ")).toBe(true);
    expect(longEnough("a")).toBe(false);
    expect(longEnough("ส")).toBe(false);
    expect(longEnough("ab")).toBe(true);
    expect(longEnough("")).toBe(false);
  });
});
