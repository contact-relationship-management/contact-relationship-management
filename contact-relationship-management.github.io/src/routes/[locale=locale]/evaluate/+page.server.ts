import { error } from "@sveltejs/kit";
import type { EntryGenerator, PageServerLoad } from "./$types";
import { render } from "#lib/server/content.js";
import { href } from "#lib/href.js";

import { EVALUATE_LOCALES } from "#lib/site.js";

export const entries: EntryGenerator = () => EVALUATE_LOCALES.map((locale) => ({ locale }));

export const load: PageServerLoad = ({ params }) => {
  if (!EVALUATE_LOCALES.includes(params.locale)) error(404, "Not found");
  const path = `locales/${params.locale}/evaluate/index.md`;
  const { title, html } = render(path, href, "hero");
  return { title, hero: html, body: render(path, href, "body").html, locale: params.locale };
};
