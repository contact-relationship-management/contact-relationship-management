# contact-relationship-management.github.io

The Contact Relationship Management documentation site for GitHub Pages: a static SvelteKit build (`adapter-static`,
Svelte 5 runes, Lily Design System widgets) of the repository's own documents.

This directory lives inside the monorepo, `contact-relationship-management/contact-relationship-management`, and is
**exported** to its own repository, `contact-relationship-management/contact-relationship-management.github.io`, with
`git subtree` (the family pattern: the workforce planning repository's `spec/monorepo-github-pages/`). Change the site
here; never work in the exported repository. The specification is
[`../spec/github-pages-site.md`](../spec/github-pages-site.md).

**Status: published** at <https://contact-relationship-management.github.io/> (2026-11-04, CRM-T178). The steps to publish a change are below.

## What it shows

| Route | Document |
| --- | --- |
| `/`, `/<locale>/` | the landing page, [`locales/<locale>/index.md`](../locales/) in English, Arabic, Welsh, Hindi and Mandarin |
| `/tutorial/` | [`docs/tutorial-first-tour.md`](../docs/tutorial-first-tour.md) |
| `/compare/` | [`COMPARISONS.md`](../COMPARISONS.md) |
| `/compare/<product>/` | [`docs/compare/<product>.md`](../docs/compare/) |
| `/overview/` | [`README.md`](../README.md) |
| `/search/` | search in the browser, from `/search-index.json` |
| `/install/` | [`INSTALL.md`](../INSTALL.md) |
| `/family-bridges/` | [`docs/tutorial-family-bridges.md`](../docs/tutorial-family-bridges.md) |
| `/spec/` | [`spec/index.md`](../spec/index.md) |
| `/locales/` | [`spec/locale/index.md`](../spec/locale/index.md), the locale status |
| `/<locale>/for/` and `/<locale>/for/<slug>/` | the role briefs, [`locales/<locale>/for/<slug>/index.md`](../locales/) (11 locales, 11 roles) |

A link between two of these documents stays on the site; a link to any other file in the repository goes to that
file on GitHub. Raw HTML in the Markdown is shown as text. A brief's slug is in its locale's own script (Arabic,
Chinese, Hindi, Welsh), so the same role is matched across locales by its `.locale-peer-id`: on a role brief the
header's language button (Lily `LocalePicker`) opens the same brief in another locale. `<html lang>` is the page's
locale and `dir` is `rtl` on the Arabic (`ar-001`) pages. The site's own words (navigation, headings, footer) follow the
page's language the way the CRM UI's strings do, chosen by language (`cy-gb` and `cy-001` read Welsh); they exist in
English, Arabic, Welsh, Hindi and Chinese, and every language but English is a machine draft. The header is the full Lily `PickerBar`: pages, search, theme (all 57, starting from the system's light or dark
preference), locale, time zone, text size and share.

## Content

The site does not read `..` when it builds: the exported repository has no parent. `scripts/sync-content.mjs` copies
the documents into `src/content/docs.json` and the brief index (locale, slug, peer id) into `src/content/briefs.json`;
both are committed. After editing any of the documents above, run in this directory:

```sh
pnpm run sync          # rewrite src/content/
pnpm run sync:check    # CI runs this: fails if the copy is stale
```

## Build and test

Node 26 and pnpm.

```sh
pnpm install
pnpm run check                                   # svelte-check
pnpm run build                                   # prerenders every page into build/
PW_PORT=4189 CI=true pnpm exec playwright test   # every page answers; axe in light and dark; rtl; locale switch
```

From the monorepo root, `scripts/ci-check.sh site` runs all of it, as the CI job `site` does. `BASE_PATH` (empty by
default) serves the site from a sub-path if it is ever published as a project site rather than the organization site.

## Publishing

The repository exists, its Pages source is GitHub Actions, and `.github/workflows/deploy.yml` in this directory is its
root workflow. To publish a change:

1. Commit it, then from the monorepo root run:

   ```sh
   make publish-github-pages
   ```

   which runs `bin/publish-github-pages`: it checks that the remote exists, that `src/content/` is current and that this
   directory has no uncommitted change, then runs
   `git subtree push --prefix=contact-relationship-management.github.io github-pages main`.
2. Watch the Deploy to GitHub Pages run in the exported repository.

A green `make publish-github-pages` means the code was pushed, not that the site was deployed; only that run says so. On
2026-11-04 three attempts at one commit timed out in `actions/deploy-pages` with GitHub reporting no incident. The
cause was not established, and the live site kept the previous deployment.

The remote lives in each clone, so a new clone adds it once:

```sh
git remote add github-pages git@github.com:contact-relationship-management/contact-relationship-management.github.io.git
```

## Not done

- Every action in the deploy workflow is pinned to a commit SHA with its tag in a comment, as the monorepo's own
  workflows are, but Dependabot does not watch it (the `github-actions` entry covers the root `.github` only).
- The Playwright and axe suite runs against `vite preview`, not the live site. The live site had every page requested
  and seven pages loaded in Chromium; axe has not been run against it.
- The documents other than the briefs are English only. The site's own words in Arabic, Welsh, Hindi and Chinese
  have not been read by a native speaker, and there are none for a language without briefs.
- Axe runs on one brief per locale (11 of 121), and on themes other than `light` and `dark` on one page.
- The time-zone picker has no effect here: nothing on the site shows a time.
- There is no hosted demo. The tutorial was run once from a fresh clone (macOS, 2026-11-04); no test keeps it so. The comparison pages are English, and their sources were checked on 2026-11-04.
- Before the theme stylesheet loads (the first paint, or with JavaScript off) the page uses the light fallbacks of the
  tokens, whatever the system preference.
- GitHub Pages cannot send HTTP headers, so the content security policy is a `<meta http-equiv>` tag, which cannot
  carry `frame-ancestors`.
