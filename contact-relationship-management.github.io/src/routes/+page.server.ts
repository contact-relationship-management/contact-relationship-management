import type { PageServerLoad } from "./$types";
import { render } from "#lib/server/content.js";
import { href } from "#lib/href.js";

// The landing page, in English; `/en-001/` is the same page at its locale address.
export const load: PageServerLoad = () => {
  const path = "locales/en-001/index.md";
  const { title, html } = render(path, href, "hero");
  return { title, hero: html, body: render(path, href, "body").html };
};
