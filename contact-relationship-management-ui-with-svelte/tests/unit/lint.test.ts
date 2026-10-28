// @vitest-environment node
// ESLint and Prettier as tests (CRM-R80): the project's own lint rules do what they say, and the
// whole project passes both tools, so `pnpm test` fails on a lint or formatting fault as well as on
// a wrong answer. `pnpm lint` runs the same two tools from the command line (and CI runs it).

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { ESLint } from "eslint";
import { createRequire } from "node:module";
import { describe, expect, it } from "vitest";

const root = process.cwd();
// Prettier is loaded by Node's own resolver: Vite would give the browser build, which cannot read
// files or the project's `.prettierignore`.
const prettier: typeof import("prettier") = createRequire(
  join(root, "package.json"),
)("prettier");
// Linting the whole project takes a few seconds, and more on a busy machine.
const SLOW = 120_000;

const eslint = new ESLint({ cwd: root });

/** The messages ESLint gives `code` as if it lived at `filePath`. */
async function lint(code: string, filePath: string) {
  const [result] = await eslint.lintText(code, {
    filePath: join(root, filePath),
  });
  return result!.messages.map((m) => `${m.ruleId}: ${m.message}`);
}

describe("the rule that no page shows the service's error text", () => {
  const page = "src/routes/zz-sample/+page.svelte";

  it("refuses an error's own message in a page", async () => {
    const found = await lint(
      `<script lang="ts">\n  let note = $state("");\n  try { await fetch("/x"); } catch (cause) { note = (cause as Error).message; }\n</script>\n<p>{note}</p>\n`,
      page,
    );
    expect(
      found.filter((m) => m.startsWith("no-restricted-properties")),
    ).toHaveLength(1);
  });

  it("refuses printing an error as a string", async () => {
    const found = await lint(
      `<script lang="ts">\n  let note = $state("");\n  try { await fetch("/x"); } catch (error) { note = String(error); }\n</script>\n<p>{note}</p>\n`,
      page,
    );
    expect(
      found.filter((m) => m.startsWith("no-restricted-syntax")),
    ).toHaveLength(1);
  });

  it("refuses it in a component and in a page's own script, too", async () => {
    const code = `export const text = (e: Error) => e.message;\n`;
    expect(
      (await lint(code, "src/routes/zz-sample/helper.ts")).join(),
    ).toContain("no-restricted-properties");
    expect(
      (
        await lint(
          `<script lang="ts">\n  let { e }: { e: Error } = $props();\n</script>\n<p>{e.message}</p>\n`,
          "src/lib/components/Zz.svelte",
        )
      ).join(),
    ).toContain("no-restricted-properties");
  });

  it("leaves the one place that may read an error, and ordinary code, alone", async () => {
    const code = `export const text = (e: Error) => e.message;\n`;
    expect(await lint(code, "src/lib/errors.ts")).toEqual([]);
    expect(
      await lint(
        `export const total = (rows: number[]) => rows.length;\n`,
        "src/routes/zz-sample/helper.ts",
      ),
    ).toEqual([]);
  });
});

/** Every file under `dir` whose name ends with one of `endings`. */
function files(dir: string, endings: string[]): string[] {
  return readdirSync(dir).flatMap((name) => {
    if (name === "node_modules" || name.startsWith(".")) return [];
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return files(path, endings);
    return endings.some((e) => name.endsWith(e)) ? [path] : [];
  });
}

describe("the project passes ESLint", () => {
  it(
    "with no error and no warning anywhere",
    async () => {
      const results = await eslint.lintFiles(["."]);
      const faults = results.flatMap((r) =>
        r.messages.map(
          (m) =>
            `${relative(root, r.filePath)}:${m.line} ${m.ruleId}: ${m.message}`,
        ),
      );
      expect(faults).toEqual([]);
    },
    SLOW,
  );
});

describe("the project passes Prettier", () => {
  it(
    "every file it can format is formatted",
    async () => {
      const targets = [
        ...files(join(root, "src"), [".ts", ".js", ".css", ".json", ".svelte"]),
        ...files(join(root, "tests"), [".ts", ".js"]),
        ...files(join(root, "scripts"), [".mjs", ".js", ".ts"]),
        ...readdirSync(root)
          .filter((name) => /^[^.].*\.(ts|js)$/.test(name))
          .map((name) => join(root, name)),
      ];
      const unformatted: string[] = [];
      let checked = 0;
      for (const path of targets) {
        const info = await prettier.getFileInfo(path, {
          ignorePath: join(root, ".prettierignore"),
        });
        // `.svelte` has no parser here, and an ignored file is none of Prettier's business.
        if (info.ignored || info.inferredParser === null) continue;
        const options = (await prettier.resolveConfig(path)) ?? {};
        checked += 1;
        if (
          !(await prettier.check(readFileSync(path, "utf8"), {
            ...options,
            filepath: path,
          }))
        )
          unformatted.push(relative(root, path));
      }
      expect(checked, "Prettier looked at nothing").toBeGreaterThan(50);
      expect(unformatted).toEqual([]);
    },
    SLOW,
  );
});
