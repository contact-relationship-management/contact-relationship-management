// Route matchers (SvelteKit 3: one module, listed in `vite.config.ts`).
//
// SvelteKit loads this file in plain Node at build time, so it cannot import
// the i18n module (a rune file that needs the app runtime). The locale list is
// therefore repeated here, and `tests/unit/locale-route.test.ts` fails if it
// ever differs from `LOCALES` in `src/lib/i18n.svelte.ts`.
import { defineParams } from "@sveltejs/kit/params";

/** The UI's languages: the `[locale=locale]` values. */
export const ROUTE_LOCALES: readonly string[] = [
  "ar-001",
  "cy-001",
  "de-de",
  "en-001",
  "es-001",
  "fr-001",
  "hi-001",
  "zh-cn",
];

/** The locale named by a path segment, or `undefined` if it is not one. */
export function localeOf(segment: string): string | undefined {
  return ROUTE_LOCALES.includes(segment) ? segment : undefined;
}

export const params = defineParams({ locale: localeOf });
