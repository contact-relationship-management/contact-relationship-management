import { error } from "@sveltejs/kit";
import type { EntryGenerator, PageServerLoad } from "./$types";
import { renderMarkdown, source } from "#lib/server/content.js";
import { href } from "#lib/href.js";
import { PRODUCTION_LOCALES, hasProductionPage } from "#lib/site.js";
import { readiness, type Readiness } from "#lib/server/readiness.js";

export const entries: EntryGenerator = () => PRODUCTION_LOCALES.map((locale) => ({ locale }));

export const load: PageServerLoad = ({ params }) => {
  if (!hasProductionPage(params.locale)) error(404, "No path-to-production page in this locale");
  const path = `locales/${params.locale}/path-to-production/index.md`;
  const { before, items, after } = readiness(path, source(path));
  const head = renderMarkdown(path, before, href);
  const list: Readiness[] = items.map((i) => ({
    name: i.name,
    percent: i.percent,
    html: renderMarkdown(path, i.text, href).html,
  }));
  return {
    title: head.title,
    before: head.html,
    items: list,
    after: renderMarkdown(path, after, href).html,
    locale: params.locale,
  };
};
