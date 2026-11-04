import { BRIEF_LOCALES, COMPARE_PAGES, DOC_PAGES, routesOf } from "#lib/site.js";
import { sitemap } from "#lib/server/sitemap.js";

// The whole site. Written to a file at build time.
export const prerender = true;

export function GET(): Response {
  return sitemap(["/", ...DOC_PAGES.map((d) => `/${d.slug}/`), ...COMPARE_PAGES.map((c) => `/compare/${c}/`), ...BRIEF_LOCALES.flatMap(routesOf)]);
}
