import { error } from "@sveltejs/kit";
import type { EntryGenerator, PageServerLoad } from "./$types";
import { render } from "#lib/server/content.js";
import { href } from "#lib/href.js";

// Only en-001 has an evaluate page for now
export const entries: EntryGenerator = () => [
  { locale: "en-001" },
];

export const load: PageServerLoad = ({ params }) => {
  const path = `locales/${params.locale}/evaluate/index.md`;
  const { title, html } = render(path, href, "hero");
  return { title, hero: html, body: render(path, href, "body").html, locale: params.locale };
};
