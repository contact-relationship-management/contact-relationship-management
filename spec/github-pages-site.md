# GitHub Pages site

*Added 2026-10-30 (CRM-T177). Published 2026-11-04 (CRM-T178) at <https://contact-relationship-management.github.io/>.*

The public documentation site, `contact-relationship-management.github.io/` in this repository. It follows the family
pattern written down in the workforce planning repository (`workforce-planning-management/spec/monorepo-github-pages/`)
and built first in the project portfolio repository (its `spec/github-pages-site.md`, task GP-1): a SvelteKit and Lily
Design System subproject named `<repo>.github.io/` inside the monorepo, exported with `git subtree` to a read-only
sibling repository, published by `make github-pages`.

## What it is

- **A welcome, then the documents.** The site is the project's front door: it says what the software offers, how to
  try it and how it compares, in plain and friendly words, and it stays honest that this is demo software
  (CRM-T193).
- **Static.** SvelteKit 3 with `@sveltejs/adapter-static`; every page is prerendered (`prerender = true`,
  `trailingSlash = "always"`, so `/install/` is `install/index.html`): 157 pages. No server, no API call, no form.
- **Svelte 5 runes** and the full Lily `PickerBar`, as the CRM UI uses it, at the same pinned versions: link (every
  page and the repository), search, theme (all 57 Lily themes, following the system preference first), locale, time
  zone, text size (seven steps) and share (the UI's five destinations and copy link). The time-zone picker records
  the choice on `<html data-time-zone>`, and nothing on this site shows a time, so it has no visible effect here.
- **Search.** `/search-index.json` is written at build time with the text of every page, fetched on the first search,
  and searched in the browser; `/search/?words` is the page the search button opens. It is in English, whichever page
  the search began on.
- **Pages.** The landing page `/` and `/<locale>/` (`locales/<locale>/index.md`: the hero is what comes before the
  first `##`, the rest are the sections, and a list renders as cards); `docs/tutorial-first-tour.md` (`/tutorial/`),
  `COMPARISONS.md` (`/compare/`), every `docs/compare/<product>.md` (`/compare/<product>/`), `INSTALL.md`
  (`/install/`), `README.md` (`/overview/`), `docs/tutorial-family-bridges.md` (`/family-bridges/`), `spec/index.md`
  (`/spec/`), the locale status `spec/locale/index.md` (`/locales/`), and every role brief
  `locales/<locale>/for/<slug>/index.md` at `/<locale>/for/<slug>/`, with a list per locale at `/<locale>/for/`
  (11 locales, 11 roles each).
- **Sitemaps.** `/sitemap.xml` for the whole site, and `/<locale>/sitemap.xml` for each locale. The locale rules name
  `/locales/<locale>/sitemap.xml`; on this site a locale's pages are at `/<locale>/`, so its sitemap is there.
- **Comparison pages.** Each says first where the other product is stronger, states that this project is demo software,
  makes no claim about the other product beyond a linked public page, and gives the date those pages were checked.
- **Links.** A relative link to a document the site shows goes to its page (with its `#fragment`; headings get
  GitHub's anchors); a relative link to any other repository path goes to that path on GitHub (`blob`, or `tree` for a
  directory); a link that climbs out of the repository (the sibling repositories) goes to the repository's home. The
  prerender fails on a link to a missing page or a missing anchor.
- **Raw HTML** in the Markdown is escaped and shown as text; the HTML on a page comes only from the Markdown renderer.

## Content copy

The exported repository has no parent directory, so the site cannot read `../README.md` when it builds. The site's
`scripts/sync-content.mjs` copies the documents into `src/content/docs.json` and the brief index (locale, slug and
`.locale-peer-id` of each brief) into `src/content/briefs.json`, both committed. `--check` fails when either differs
from the repository; CI runs it, and `bin/make-github-pages` runs it before pushing. The document list is in that
script and in `src/lib/site.ts`.

## Locales and direction

- **Briefs across scripts.** The Arabic, Chinese, Hindi and Welsh briefs have slugs in their own script, so a role is
  matched across locales by its peer id, not its slug. The language button opens the same role's brief in the chosen
  locale, and each brief lists the others with `<link rel="alternate" hreflang>`.
- **`lang` and `dir`.** `<html lang>` is the page's locale in BCP 47 form (`en-gb-oxendict` is `en-GB-oxendict`,
  `ar-001` stays `ar-001`) on a brief or a locale's list, and `en` elsewhere; `dir` is `rtl` for a right-to-left
  language, which `ar-001` is. Both are written into the prerendered HTML by `hooks.server.ts` and kept in step on
  client-side navigation. Every layout rule uses logical properties; code and paths stay left to right.
- **Locale homes.** English (`en-001`, `en-gb`, `en-gb-oxendict`, `en-us`), Welsh (`cy-gb`, `cy-001`) and Mandarin
  (`zh-cn`, `zh-001`) have a home page, written first in that order per
  [the locale rules](locales-for-global-sharing-with-svelte/index.md), with a translated wordmark and a "Translations:"
  sentence of the `*-001` endonyms from `locales.tsv`. Arabic and Hindi have their briefs and no home page, so the
  locale button opens their list of briefs. The rules' longer list of locales is not started.
- **The locale button** is on every page, labelled each locale in its own language. It reports the locale it starts on,
  so a document, which shows the default brief locale, must ignore that report or it would redirect.
- **The site's own words** follow the CRM UI's locale approach (`src/lib/i18n.svelte.ts` there): one table of strings
  per language in `src/lib/i18n.ts`, English the source, every table the same keys (the type enforces it), and a
  locale read by its language, so `cy-gb` and `cy-001` both show Welsh and `zh-001` and `zh-cn` both Chinese. The
  tables are English, Arabic, Welsh, Hindi and Chinese, the languages of the briefs; the four that are not English are
  machine drafts. A page that is not a brief or a locale home (the README and the other documents) is English, because its document is.

## Colour and accessibility

The colour tokens are the family documentation site's (`--mxi-color-*` over the Lily `--color-*` tokens, as the CRM
UI maps its own), with the contrast-fixed link and fill colours the project portfolio site uses. Links in running text
are underlined. A table or code block that scrolls sideways is focusable. The target is **no axe violation of any
impact** for the WCAG 2.0, 2.1 and 2.2 A and AA tags, under the light and the dark system preference.

Every theme the picker offers passes axe on one document (57). That needed the link colour moved to 25% primary and
the muted colour to 92%, and header links in the theme's own text colour. Lily's hover rule never reaches the picker
buttons (it matches `[class$="-button"]`, and they write their class with a trailing space), so the site sets the same
colours itself.

## Content security policy

GitHub Pages sends no custom headers, so the policy is SvelteKit's `<meta http-equiv>` with hashes of its own inline
scripts (`mode: "hash"`): `default-src 'self'`, `script-src 'self'` plus hashes, `style-src 'self'`, and a
`style-src-attr` hash for SvelteKit's announcer only. Computed styles use Svelte's `style:` directive, never a
`style="…"` attribute. `frame-ancestors` cannot be set from a meta tag and is not set.

## Tests

`tests/e2e/site.spec.ts` (Playwright with `@axe-core/playwright`, against `vite preview` of the build): every page
answers 200 (157); each of the seven pickers changes colour on hover and opens under its button without moving
the page, left to right and right to left; search across scripts; share and copy; the time-zone record; the landing
page in three languages; the sitemaps; each comparison page fair and sourced; axe in every theme; axe under light and dark on every document, every locale's list and one brief per locale, with no
CSP report in the console; `lang` and `dir` on a brief and on a document; the two Arabic pages right to left (header
`dir`, the brand at the inline start, no sideways scroll, Arabic navigation, axe clean in both schemes); the site's
words by language for five locales; the language button from English to Arabic to Welsh across scripts; links on and
off the site; no sideways scroll at 390 px; no element with a `style` attribute. `scripts/ci-check.sh site` runs the
content check, `svelte-check`, the build and the suite; the CI job `site` runs that stage.

## Publishing

`make github-pages` runs `bin/make-github-pages`, which runs
`git subtree push --prefix=contact-relationship-management.github.io github-pages main` after checking that the
`github-pages` remote exists, the content copy is current and the directory has no uncommitted change. The exported
repository deploys with its own `.github/workflows/deploy.yml` (GitHub Pages' actions; inert inside the monorepo).
The `github-pages` remote is per clone; the steps are in the root [README](../README.md#github-pages-site) and the site's
[README](../contact-relationship-management.github.io/README.md#publishing).

Published 2026-11-04 (CRM-T178). The exported repository's Deploy to GitHub Pages workflow passed its first run;
all 137 pages answered 200 on the live host, the native-script paths among them; and seven pages (four documents,
an Arabic list and brief, a Chinese brief) loaded in Chromium with no console error, CSP report or failed request, with the expected `lang` and `dir`.

## Not built

The deploy workflow's actions are pinned by commit SHA and not watched by Dependabot; the Playwright and axe suite
runs against `vite preview`, not the live site, and axe has not been run against the live site; there is no hosted
demo, so the tutorial needs Node.js and pnpm; the tutorial was written from the repository and has not been timed;
the comparison pages are English only and will age; the search page is English; the theme and text-size picker
options are English on every page; the documents other than the briefs and the locale homes are English only, and the site's words in four languages are unreviewed drafts; axe runs on 11 of the 121 briefs, and on themes other than `light` and `dark` on one page; the first paint (or a visit with JavaScript off) uses the light
fallback tokens whatever the system preference.
