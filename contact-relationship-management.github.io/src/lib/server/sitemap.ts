import { SITE_URL } from "#lib/site.js";

/** A sitemap of site routes. Native-script slugs are percent-encoded, as the protocol asks. */
export function sitemap(routes: readonly string[]): Response {
  const urls = routes.map((route) => `  <url><loc>${SITE_URL}${encodeURI(route)}</loc></url>`).join("\n");
  const xml = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls}
</urlset>
`;
  return new Response(xml, { headers: { "content-type": "application/xml" } });
}
