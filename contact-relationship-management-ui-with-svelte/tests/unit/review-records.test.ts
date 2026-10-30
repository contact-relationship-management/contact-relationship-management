import { existsSync, readFileSync, readdirSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { LOCALES } from "#lib/i18n.svelte.js";

// One review record per locale (CRM-T175), and none of them claims a review that has not happened.
describe("the per-locale review records", () => {
  const dir = "spec/reviews";

  it("has one record for each locale, and no record without a locale", () => {
    const files = readdirSync(dir).filter((f) => f.endsWith(".md"));
    expect(files.map((f) => f.replace(/\.md$/, "")).sort()).toEqual(
      [...LOCALES].sort(),
    );
  });

  it("says each locale is not reviewed and names no reviewer, as the register does", () => {
    const register = readFileSync("spec/translation-register.md", "utf8");
    for (const locale of LOCALES) {
      const path = `${dir}/${locale}.md`;
      expect(existsSync(path), path).toBe(true);
      const record = readFileSync(path, "utf8");
      expect(record, locale).toContain(`# Review record: \`${locale}\``);
      expect(record, locale).toContain("**not reviewed**");
      expect(record, locale).toMatch(/\| Reviewer \| none \|/);
      expect(record, locale).toMatch(/\| Reviewed on \| never \|/);
      // The register agrees: no locale is `reviewed` while its record says otherwise.
      const row = register
        .split("\n")
        .find((line) => line.startsWith(`| \`${locale}\` |`));
      expect(row, locale).toBeDefined();
      expect(row, locale).not.toContain("`reviewed`");
    }
  });

  it("records the two regional codes as a limit with a reason", () => {
    for (const locale of ["de-de", "zh-cn"]) {
      const record = readFileSync(`${dir}/${locale}.md`, "utf8");
      expect(record, locale).toContain(
        "Regional code, kept as a recorded limit",
      );
    }
  });
});
