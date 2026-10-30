import type { EntryGenerator, PageServerLoad } from "./$types";
import { BRIEF_LOCALES, briefsOf } from "#lib/site.js";
import { briefSummary } from "#lib/server/content.js";

export const entries: EntryGenerator = () => BRIEF_LOCALES.map((locale) => ({ locale }));

export const load: PageServerLoad = ({ params }) => {
  const locale = params.locale;
  const briefs = briefsOf(locale).map((b) => ({
    slug: b.slug,
    ...briefSummary(`locales/${locale}/for/${b.slug}/index.md`),
  }));
  return { title: "Role briefs", locale, briefs };
};
