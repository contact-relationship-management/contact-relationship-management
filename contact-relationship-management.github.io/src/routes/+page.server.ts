import type { PageServerLoad } from "./$types";
import { render } from "#lib/server/content.js";
import { href } from "#lib/href.js";

export const load: PageServerLoad = () => {
  const { title, html } = render("README.md", href);
  return { title, html, path: "README.md" };
};
