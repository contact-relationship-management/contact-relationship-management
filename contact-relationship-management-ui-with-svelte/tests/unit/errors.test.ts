// No page shows the service's error text (CRM-R76): a failed call becomes a fixed, translated
// message chosen by the kind of failure, and the service's own words never reach the screen.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import { ApiError } from "../../src/lib/api/client";
import { PageError, errorMessage, fieldErrors } from "../../src/lib/errors";
import { loadAllLocales, translate } from "../../src/lib/i18n.svelte";

const SECRET = "contact c-4471 has an active nurture enrolment (rule R20)";

const kinds: [number, string][] = [
  [400, "error.invalid"],
  [401, "error.unauthorized"],
  [403, "error.forbidden"],
  [404, "error.notFound"],
  [409, "error.conflict"],
  [422, "error.invalid"],
  [429, "error.tooMany"],
  [500, "error.unavailable"],
  [502, "error.unavailable"],
  [503, "error.unavailable"],
  [418, "error.generic"],
];

describe("errorMessage", () => {
  it.each(kinds)(
    "says a fixed message for a %i, never the service's words",
    (status, key) => {
      const cause = new ApiError(status, { description: SECRET });
      expect(cause.message).toContain("nurture"); // the service did say it
      const shown = errorMessage(cause);
      expect(shown).toBe(translate(key as never, "en-001"));
      expect(shown).not.toContain("nurture");
      expect(shown).not.toContain("c-4471");
    },
  );

  it("keeps the service's words out even when the problem carries a title and detail", () => {
    const cause = new ApiError(422, {
      title: "Unprocessable",
      detail: SECRET,
      description: SECRET,
      errors: [{ pointer: "/name", detail: SECRET }],
    });
    expect(errorMessage(cause)).not.toContain("nurture");
  });

  it("uses the page's own message when it gives one, and a PageError as written", () => {
    expect(
      errorMessage(
        new ApiError(500, { description: SECRET }),
        "Could not save",
      ),
    ).toBe("Could not save");
    expect(
      errorMessage(new PageError("Cards cannot move between lanes here")),
    ).toBe("Cards cannot move between lanes here");
  });

  it("calls a failed fetch unreachable and anything else generic, whatever it says", () => {
    expect(errorMessage(new TypeError("Failed to fetch"))).toBe(
      translate("error.unavailable", "en-001"),
    );
    for (const odd of [new Error(SECRET), SECRET, 42, null, undefined]) {
      expect(errorMessage(odd)).toBe(translate("error.generic", "en-001"));
    }
  });

  it("has every message in every locale, none of them empty", async () => {
    const tables = await loadAllLocales();
    for (const [locale, table] of Object.entries(tables) as [
      string,
      Record<string, string>,
    ][]) {
      for (const key of new Set(kinds.map(([, k]) => k)).add(
        "error.fieldInvalid",
      )) {
        expect(table[key]?.trim(), `${locale} ${key}`).toBeTruthy();
      }
    }
  });
});

describe("fieldErrors", () => {
  it("names the member and says nothing the service wrote", () => {
    const shown = fieldErrors({
      body: { errors: [{ pointer: "/display_name", detail: SECRET }] },
    });
    expect(Object.keys(shown)).toEqual(["display_name"]);
    expect(shown.display_name).toBe(translate("error.fieldInvalid", "en-001"));
    expect(JSON.stringify(shown)).not.toContain("nurture");
  });
});

/** Every `.svelte` and `.ts` file under `dir`. */
function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return sources(path);
    return /\.(svelte|ts)$/.test(name) ? [path] : [];
  });
}

describe("the pages", () => {
  // The lint rule (`eslint.config.js`) says the same; this fails even where lint is not run.
  it("never read an error's `.message` or print an error as a string", () => {
    const root = process.cwd();
    const files = [
      ...sources(join(root, "src/routes")),
      ...sources(join(root, "src/lib/components")),
    ];
    const offences: string[] = [];
    for (const file of files) {
      readFileSync(file, "utf8")
        .split("\n")
        .forEach((line, index) => {
          if (
            /\.message\b/.test(line) ||
            /String\((cause|err|error|e)\)/.test(line)
          ) {
            offences.push(
              `${file.slice(root.length + 1)}:${index + 1}: ${line.trim()}`,
            );
          }
        });
    }
    expect(offences).toEqual([]);
  });
});
