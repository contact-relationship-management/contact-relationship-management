import { error } from "@sveltejs/kit";
import type { EntryGenerator, PageServerLoad } from "./$types";
import { render } from "#lib/server/content.js";
import { href } from "#lib/href.js";
import { PRIVACY_LOCALES, hasPrivacyPage } from "#lib/site.js";

// The privacy notice in one locale (locales/<locale>/privacy/index.md; English source docs/privacy-notice.md).
export const entries: EntryGenerator = () => PRIVACY_LOCALES.map((locale) => ({ locale }));

export const load: PageServerLoad = ({ params }) => {
  if (!hasPrivacyPage(params.locale)) error(404, "No privacy notice in this locale");
  const path = `locales/${params.locale}/privacy/index.md`;
  const { title, html } = render(path, href);
  return { title, html, path, locale: params.locale };
};
