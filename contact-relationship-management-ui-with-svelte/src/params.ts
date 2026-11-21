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
  "bn-001",
  "cy-001",
  "da-001",
  "de-de",
  "en-001",
  "es-001",
  "fr-001",
  "hi-001",
  "id-001",
  "it-001",
  "ja-001",
  "ko-001",
  "nl-001",
  "pt-001",
  "ru-001",
  "sv-001",
  "sw-001",
  "ur-001",
  "vi-001",
  "zh-cn",
];

/** The locale named by a path segment, or `undefined` if it is not one. */
export function localeOf(segment: string): string | undefined {
  return ROUTE_LOCALES.includes(segment) ? segment : undefined;
}

/**
 * Split a path into its language prefix and the rest: `/de-de/contacts` is
 * `{ locale: "de-de", path: "/contacts" }`, `/de-de` is `{ …, path: "/" }`, and
 * a path with no prefix keeps `locale` undefined.
 */
export function splitLocale(pathname: string): {
  locale?: string;
  path: string;
} {
  const [, first = "", ...rest] = pathname.split("/");
  const locale = localeOf(first);
  if (locale === undefined) return { path: pathname };
  return { locale, path: `/${rest.join("/")}` };
}

export const params = defineParams({ locale: localeOf });
