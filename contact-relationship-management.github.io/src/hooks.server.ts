import type { Handle } from "@sveltejs/kit/hooks";
import { isRtl, langTag, localeOfPath } from "#lib/site.js";

// `lang` and `dir` on <html> come from the page's locale (the first path segment of a role brief), so the
// prerendered HTML is right before any script runs. Every other page is English.
export const handle: Handle = async ({ event, resolve }) => {
  const locale = localeOfPath(event.url.pathname);
  const lang = locale ? langTag(locale) : "en";
  const dir = locale && isRtl(locale) ? "rtl" : "ltr";
  return resolve(event, {
    transformPageChunk: ({ html }) => html.replace("%lang%", lang).replace("%dir%", dir),
  });
};
