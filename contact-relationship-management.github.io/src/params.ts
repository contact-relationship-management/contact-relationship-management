import { defineParams } from "@sveltejs/kit/params";
import { DOC_PAGES, isBriefLocale, isRole } from "#lib/site.ts";

// Route parameters: [slug=doc] is one of the documents (not the home page), [locale=locale] a role-brief locale and
// [role=role] a role brief's slug in any locale (the page checks the pair).
export const params = defineParams({
  doc: (p: string) => (DOC_PAGES.some((d) => d.slug !== "" && d.slug === p) ? p : undefined),
  locale: (p: string) => (isBriefLocale(p) ? p : undefined),
  role: (p: string) => (isRole(p) ? p : undefined),
});
