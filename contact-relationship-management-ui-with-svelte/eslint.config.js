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
  {
    // No page shows the service's error text (CRM-R76). A failed call is turned into words by
    // `errorMessage` in `src/lib/errors.ts`, the one place that may read an error: it chooses a
    // fixed, translated message by the kind of failure. Pages and components must not reach
    // past it for the error's own `message`, or print an error as a string.
    files: ["src/routes/**/*.{ts,svelte}", "src/lib/components/**/*.svelte"],
    rules: {
      "no-restricted-properties": [
        "error",
        {
          property: "message",
          message:
            "Do not show an error's own text: use errorMessage() from #lib/errors.js, or throw a PageError.",
        },
      ],
      "no-restricted-syntax": [
        "error",
        {
          selector:
            "CallExpression[callee.name='String'][arguments.0.name=/^(cause|err|error|e)$/]",
          message:
            "Do not print an error as a string: use errorMessage() from #lib/errors.js.",
        },
      ],
    },
  },
);
