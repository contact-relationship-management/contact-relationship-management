# GitHub Pages site

*Added 2026-10-30 (CRM-T177). Publishing is CRM-T178, the owner's, open.*

The public documentation site, `contact-relationship-management.github.io/` in this repository. It follows the family
pattern written down in the workforce planning repository (`workforce-planning-management/spec/monorepo-github-pages/`)
and built first in the project portfolio repository (its `spec/github-pages-site.md`, task GP-1): a SvelteKit and Lily
Design System subproject named `<repo>.github.io/` inside the monorepo, exported with `git subtree` to a read-only
sibling repository, published by `make github-pages`.

## What it is

- **Static.** SvelteKit 3 with `@sveltejs/adapter-static`; every page is prerendered (`prerender = true`,
  `trailingSlash = "always"`, so `/install/` is `install/index.html`): 137 pages. No server, no API call, no form.
- **Svelte 5 runes** and Lily widgets as the CRM UI uses them, at the same pinned versions: `ThemePicker` (themes
  `light` and `dark`, following the system preference), `TextSizePicker` (the same seven steps and `data-text-size`
  rules) and, on the role-brief pages, `LocalePicker`.
- **Pages.** `README.md` (`/`), `INSTALL.md` (`/install/`), `docs/tutorial-family-bridges.md` (`/family-bridges/`),
  `spec/index.md` (`/spec/`), the locale status `spec/locale/index.md` (`/locales/`), and every role brief
  `locales/<locale>/for/<slug>/index.md` at `/<locale>/for/<slug>/`, with a list per locale at `/<locale>/for/`
  (11 locales, 11 roles each).
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
- **The site's own words** follow the CRM UI's locale approach (`src/lib/i18n.svelte.ts` there): one table of strings
  per language in `src/lib/i18n.ts`, English the source, every table the same keys (the type enforces it), and a
  locale read by its language, so `cy-gb` and `cy-001` both show Welsh and `zh-001` and `zh-cn` both Chinese. The
  tables are English, Arabic, Welsh, Hindi and Chinese, the languages of the briefs; the four that are not English are
  machine drafts. A page that is not a brief (the README and the other documents) is English, because its document is.

## Colour and accessibility

The colour tokens are the family documentation site's (`--mxi-color-*` over the Lily `--color-*` tokens, as the CRM
UI maps its own), with the contrast-fixed link and fill colours the project portfolio site uses. Links in running text
are underlined. A table or code block that scrolls sideways is focusable. The target is **no axe violation of any
impact** for the WCAG 2.0, 2.1 and 2.2 A and AA tags, under the light and the dark system preference.

## Content security policy

GitHub Pages sends no custom headers, so the policy is SvelteKit's `<meta http-equiv>` with hashes of its own inline
scripts (`mode: "hash"`): `default-src 'self'`, `script-src 'self'` plus hashes, `style-src 'self'`, and a
`style-src-attr` hash for SvelteKit's announcer only. Computed styles use Svelte's `style:` directive, never a
`style="…"` attribute. `frame-ancestors` cannot be set from a meta tag and is not set.

## Tests

`tests/e2e/site.spec.ts` (Playwright with `@axe-core/playwright`, against `vite preview` of the build): every page
answers 200 (137); axe under light and dark on every document, every locale's list and one brief per locale, with no
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
The remote and the repository are created by the owner; the steps are in the root [README](../README.md#github-pages-site)
and the site's [README](../contact-relationship-management.github.io/README.md#publishing).

## Not built

Not published (CRM-T178); the deploy workflow has never run (its actions are pinned by commit SHA, not watched by
Dependabot); GitHub Pages serving the native-script paths is unverified (only `vite preview` was run); the documents
other than the briefs are English only, and the site's words in four languages are unreviewed drafts; only the `light`
and `dark` themes; axe runs on 11 of the 121 briefs; the first paint (or a visit with JavaScript off) uses the light
fallback tokens whatever the system preference.
