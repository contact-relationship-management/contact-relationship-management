import { error } from "@sveltejs/kit";
import type { EntryGenerator, PageServerLoad } from "./$types";
import { BRIEFS, BRIEF_LOCALES, briefAt, peerIn } from "#lib/site.js";
import { render } from "#lib/server/content.js";
import { href } from "#lib/href.js";

export const entries: EntryGenerator = () => BRIEFS.map((b) => ({ locale: b.locale, role: b.slug }));

export const load: PageServerLoad = ({ params }) => {
  const brief = briefAt(params.locale, params.role);
  // The matchers accept any locale and any locale's slug; only a pair that exists is a page.
  if (!brief) error(404, "Not found");
  const path = `locales/${brief.locale}/for/${brief.slug}/index.md`;
  const { title, html } = render(path, href);
  // The same role in every locale, for <link rel="alternate">.
  const alternates = BRIEF_LOCALES.flatMap((l) => {
    const peer = peerIn(l, brief.peer);
    return peer ? [{ locale: l, slug: peer.slug }] : [];
  });
  return { title, html, path, locale: brief.locale, role: brief.slug, alternates };
};
