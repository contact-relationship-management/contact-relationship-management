// The site's pages and how a repository path maps to one. Safe for the browser: the brief index is small
// (`src/content/briefs.json`: locale, slug and peer id); the document text itself is read only on the server side,
// during prerendering (`$lib/server/content.ts`).
import index from "../content/briefs.json" with { type: "json" };

export const REPO_URL = "https://github.com/contact-relationship-management/contact-relationship-management";

/** A page made from one repository document. `slug` is the route under the site root ("" is the home page). */
export type DocPage = { slug: string; path: string; nav: NavKey };

/** The navigation labels, translated in `$lib/i18n.ts`. */
export type NavKey = "home" | "install" | "bridges" | "spec" | "locales";

export const DOC_PAGES: readonly DocPage[] = [
  { slug: "", path: "README.md", nav: "home" },
  { slug: "install", path: "INSTALL.md", nav: "install" },
  { slug: "family-bridges", path: "docs/tutorial-family-bridges.md", nav: "bridges" },
  { slug: "spec", path: "spec/index.md", nav: "spec" },
  { slug: "locales", path: "spec/locale/index.md", nav: "locales" },
];

/** One role brief: `locales/<locale>/for/<slug>/index.md`; `peer` is the same for one role in every locale. */
export type Brief = { locale: string; slug: string; peer: string };

export const BRIEFS: readonly Brief[] = index.briefs;

/** The locales that have role briefs (`locales/<locale>/for/`), in the repository's spelling, sorted. */
export const BRIEF_LOCALES: readonly string[] = [...new Set(BRIEFS.map((b) => b.locale))].sort();

/** The locale the navigation's "Role briefs" link opens on a page that has none (spec/locale/index.md's source). */
export const DEFAULT_BRIEF_LOCALE = "en-gb-oxendict";

export function isBriefLocale(s: string): boolean {
  return BRIEF_LOCALES.includes(s);
}

/** Whether `s` is a brief's slug in any locale (a route matcher sees one parameter at a time). */
export function isRole(s: string): boolean {
  return BRIEFS.some((b) => b.slug === s);
}

/** The brief at `locale` and `slug`, if there is one. */
export function briefAt(locale: string, slug: string): Brief | undefined {
  return BRIEFS.find((b) => b.locale === locale && b.slug === slug);
}

/** The briefs of one locale, in the repository's order. */
export function briefsOf(locale: string): Brief[] {
  return BRIEFS.filter((b) => b.locale === locale);
}

/** The same role's brief in another locale (matched by peer id, since slugs are in each locale's own script). */
export function peerIn(locale: string, peer: string): Brief | undefined {
  return BRIEFS.find((b) => b.locale === locale && b.peer === peer);
}

/** BCP 47 form for `lang` ("en-gb-oxendict" → "en-GB-oxendict", "zh-001" → "zh-001"). */
export function langTag(locale: string): string {
  const [lang, region, ...rest] = locale.split("-");
  if (!region) return lang ?? locale;
  const r = /^[a-z]{2}$/.test(region) ? region.toUpperCase() : region;
  return [lang, r, ...rest].join("-");
}

// Right-to-left base languages (the same list Lily's LocalePicker uses). `ar-001` has briefs, so its pages are
// right-to-left.
const RTL = new Set(["ar", "arc", "ckb", "dv", "fa", "he", "iw", "ji", "ks", "ku", "mzn", "ps", "sd", "ug", "ur", "yi"]);
export function isRtl(locale: string): boolean {
  return RTL.has(locale.split("-")[0] ?? "");
}

/** The locale named by a site pathname's first segment, if it is one. */
export function localeOfPath(pathname: string, base = ""): string | null {
  const rest = pathname.startsWith(base) ? pathname.slice(base.length) : pathname;
  const first = rest.split("/").filter(Boolean)[0] ?? "";
  return isBriefLocale(first) ? first : null;
}

/** The site route ("/install/", "/cy-gb/for/prif-swyddog-risg/") of a repository path, or null if it has none. */
export function routeOfRepoPath(path: string): string | null {
  const clean = path.replace(/\/+$/, "");
  for (const d of DOC_PAGES) {
    const dir = d.path.replace(/\/?[^/]+$/, "");
    if (clean === d.path || (dir && clean === dir && /\/(index|README)\.md$/.test("/" + d.path)))
      return d.slug ? `/${d.slug}/` : "/";
  }
  const m = /^locales\/([^/]+)\/for(?:\/([^/]+))?(?:\/(?:index|README)\.md)?$/.exec(clean);
  if (m && m[1] && isBriefLocale(m[1])) {
    if (!m[2]) return `/${m[1]}/for/`;
    if (briefAt(m[1], m[2])) return `/${m[1]}/for/${m[2]}/`;
  }
  return null;
}
