// The accessibility gate covers every page (CRM-R77): the table in `tests/e2e/a11y-routes.ts`
// lists exactly the pages under `src/routes`, a dynamic page has a real sample address, and a page
// that is not scanned says why.

import { readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import { ROUTES } from "../e2e/a11y-routes";

/** The pages in the file tree, as routes (`/accounts/[pid]`). */
function pages(dir: string, prefix = ""): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return pages(path, `${prefix}/${name}`);
    return name === "+page.svelte" ? [prefix || "/"] : [];
  });
}

describe("the accessibility route table", () => {
  const found = pages(join(process.cwd(), "src/routes")).sort();

  it("lists every page, and only pages", () => {
    const listed = ROUTES.map((r) => r.route).sort();
    expect(
      found.filter((r) => !listed.includes(r)),
      "pages the axe gate does not know about",
    ).toEqual([]);
    expect(
      listed.filter((r) => !found.includes(r)),
      "entries for pages that do not exist",
    ).toEqual([]);
    expect(new Set(listed).size).toBe(listed.length);
  });

  it("gives each page an address that fits its route", () => {
    for (const { route, path } of ROUTES) {
      const shape = new RegExp(
        "^" +
          route
            .replace(/\[locale=locale\]/g, "@LOCALE@")
            .replace(/\[[^\]]+\]/g, "@ID@")
            .replace(/\//g, "\\/")
            .replace(/@LOCALE@/g, "[a-z]{2}-[a-z0-9]+")
            .replace(/@ID@/g, "[^/]+") +
          "$",
      );
      expect(path, route).toMatch(shape);
      expect(path, route).not.toContain("[");
    }
  });

  it("says why when a page is not scanned", () => {
    for (const entry of ROUTES.filter((r) => r.excluded !== undefined)) {
      expect(entry.excluded?.trim().length, entry.route).toBeGreaterThan(20);
    }
  });
});
