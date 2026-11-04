import { error } from "@sveltejs/kit";
import type { EntryGenerator, PageServerLoad } from "./$types";
import { LOCALE_HOMES, hasHome } from "#lib/site.js";
import { render } from "#lib/server/content.js";
import { href } from "#lib/href.js";

export const entries: EntryGenerator = () => LOCALE_HOMES.map((locale) => ({ locale }));

export const load: PageServerLoad = ({ params }) => {
  // The matcher accepts every brief locale; only one with a home page is a page.
  if (!hasHome(params.locale)) error(404, "Not found");
  const path = `locales/${params.locale}/index.md`;
  const { title, html } = render(path, href, "hero");
  return { title, hero: html, body: render(path, href, "body").html, locale: params.locale };
};
