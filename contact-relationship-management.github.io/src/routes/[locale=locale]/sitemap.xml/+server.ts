import { BRIEF_LOCALES, routesOf } from "#lib/site.js";
import { sitemap } from "#lib/server/sitemap.js";
import type { EntryGenerator, RequestHandler } from "./$types";

// One locale's routes. Written to a file at build time.
export const prerender = true;
export const entries: EntryGenerator = () => BRIEF_LOCALES.map((locale) => ({ locale }));

export const GET: RequestHandler = ({ params }) => sitemap(routesOf(params.locale));
