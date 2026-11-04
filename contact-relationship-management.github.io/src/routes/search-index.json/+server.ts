import { json } from "@sveltejs/kit";
import { BRIEFS, COMPARE_PAGES, DOC_PAGES, LOCALE_HOMES } from "#lib/site.js";
import { render } from "#lib/server/content.js";
import { href } from "#lib/href.js";
import type { Entry } from "#lib/search.js";

// The search index, written to a file at build time: the plain text of every document and brief.
export const prerender = true;

function plain(html: string): string {
  return html
    .replace(/<[^>]*>/g, " ")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&amp;/g, "&")
    .replace(/\s+/g, " ")
    .trim();
}

function entry(path: string, route: string, locale: string | null): Entry {
  const { title, html } = render(path, href);
  return { route, title, locale, text: plain(html) };
}

export function GET(): Response {
  return json([
    ...DOC_PAGES.map((d) => entry(d.path, `/${d.slug}/`, null)),
    ...COMPARE_PAGES.map((c) => entry(`docs/compare/${c}.md`, `/compare/${c}/`, null)),
    ...LOCALE_HOMES.map((l) => entry(`locales/${l}/index.md`, `/${l}/`, l)),
    ...BRIEFS.map((b) =>
      entry(`locales/${b.locale}/for/${b.slug}/index.md`, `/${b.locale}/for/${b.slug}/`, b.locale),
    ),
  ]);
}
