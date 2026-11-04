import { error } from "@sveltejs/kit";
import type { EntryGenerator, PageServerLoad } from "./$types";
import { COMPARE_PAGES } from "#lib/site.js";
import { render } from "#lib/server/content.js";
import { href } from "#lib/href.js";

export const entries: EntryGenerator = () => COMPARE_PAGES.map((product) => ({ product }));

export const load: PageServerLoad = ({ params }) => {
  if (!COMPARE_PAGES.includes(params.product)) error(404, "Not found");
  const path = `docs/compare/${params.product}.md`;
  const { title, html } = render(path, href);
  return { title, html, path };
};
