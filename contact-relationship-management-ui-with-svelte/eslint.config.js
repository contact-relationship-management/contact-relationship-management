// Lint (CRM-T94): the recommended rules of ESLint, typescript-eslint and eslint-plugin-svelte,
// with nothing switched off but the generated API types, which are not ours to edit.
import js from "@eslint/js";
import svelte from "eslint-plugin-svelte";
import globals from "globals";
import ts from "typescript-eslint";

export default ts.config(
  {
    ignores: [
      ".svelte-kit/",
      "build/",
      "node_modules/",
      "playwright-report/",
      "test-results/",
      "src/lib/api/schema.d.ts",
    ],
  },
  js.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs["flat/recommended"],
  {
    languageOptions: { globals: { ...globals.browser, ...globals.node } },
  },
  {
    // The app is served from the root (no `paths.base`), so its absolute links need no
    // `resolve()`; turn the rule on if a base path is ever set.
    rules: { "svelte/no-navigation-without-resolve": "off" },
  },
  {
    // SvelteKit's own placeholders: `App.Locals` and friends start empty by design.
    files: ["src/app.d.ts"],
    rules: { "@typescript-eslint/no-empty-object-type": "off" },
  },
  {
    files: ["**/*.svelte", "**/*.svelte.ts"],
    languageOptions: { parserOptions: { parser: ts.parser } },
  },
);
