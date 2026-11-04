<script lang="ts">
  import "../app.css";
  import type { Snippet } from "svelte";
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import PickerBar from "@lilydesignsystem/svelte-picker-bar";
  import type { LinkItem } from "@lilydesignsystem/svelte-link-picker";
  import type { ShareTarget } from "@lilydesignsystem/svelte-share-picker";
  import { href } from "#lib/href.js";
  import { t, languageOf } from "#lib/i18n.js";
  import {
    BRIEF_LOCALES,
    DEFAULT_BRIEF_LOCALE,
    DOC_PAGES,
    LOCALE_LABELS,
    REPO_URL,
    briefAt,
    hasHome,
    isBriefLocale,
    isRtl,
    langTag,
    peerIn,
  } from "#lib/site.js";

  let { children }: { children: Snippet } = $props();

  // The role-brief locale of this page (null on the English documents).
  const locale = $derived(page.data.locale ?? null);
  const role = $derived(typeof page.params.role === "string" ? page.params.role : null);
  // The site's own words follow the page (as the CRM UI's do): the brief's language when the site has a table for
  // it, English otherwise. `lang` names what the words are in, `dir` follows the page.
  const chromeLang = $derived(languageOf(locale) === "en" ? "en" : langTag(locale ?? "en"));
  const dir = $derived(locale && isRtl(locale) ? "rtl" : "ltr");

  // Keep <html lang dir> in step on client-side navigation; the prerendered HTML already carries them
  // (hooks.server.ts).
  const htmlLang = $derived(locale ? langTag(locale) : "en");
  $effect(() => {
    document.documentElement.lang = htmlLang;
    document.documentElement.dir = dir;
  });

  // The locale picker is on every page. On a brief it shows the brief's locale and on a document, which is English,
  // the default brief locale.
  const pickedLocale = $derived(locale ?? DEFAULT_BRIEF_LOCALE);

  // The locale switcher opens the same brief (or the same list) in the chosen locale, and a document opens that
  // locale's list. The picker also reports the locale it starts on, which is `pickedLocale`: nothing to do then, or
  // every document would redirect. Slugs are in each locale's own script, so the same role is found by its peer id.
  function switchLocale(code: string) {
    if (!isBriefLocale(code) || code === pickedLocale) return;
    const here = locale && role ? briefAt(locale, role) : undefined;
    const there = here ? peerIn(code, here.peer) : undefined;
    if (there) void goto(href(`/${code}/for/${there.slug}/`));
    else if (page.url.pathname.endsWith("/for/") || !hasHome(code)) void goto(href(`/${code}/for/`));
    else void goto(href(`/${code}/`));
  }

  // The wordmark and Home stay in the page's language: its locale's home page when it has one.
  const homeRoute = $derived(locale && hasHome(locale) ? `/${locale}/` : "/");

  const nav = $derived([
    { label: t(locale, "nav.home"), route: homeRoute },
    ...DOC_PAGES.filter((d) => d.header).map((d) => ({ label: t(locale, `nav.${d.nav}`), route: `/${d.slug}/` })),
    { label: t(locale, "nav.briefs"), route: `/${locale ?? DEFAULT_BRIEF_LOCALE}/for/` },
  ]);
  // The link picker has the rest of the documents too.
  const allPages = $derived([
    ...nav,
    ...DOC_PAGES.filter((d) => !d.header).map((d) => ({ label: t(locale, `nav.${d.nav}`), route: `/${d.slug}/` })),
  ]);
  function current(route: string): boolean {
    const here = page.url.pathname;
    const target = href(route);
    if (route.endsWith("/for/")) return /\/for\//.test(here);
    return here === target || here === new URL(target, page.url.href).pathname;
  }

  // The link picker offers every page and the repository.
  const pageLinks: LinkItem[] = $derived([
    ...allPages.map((item) => ({ href: href(item.route), label: item.label, current: current(item.route) })),
    { href: REPO_URL, label: t(locale, "repository"), newTab: true },
  ]);

  const pageTitle = $derived(page.data.title ?? t(locale, "brand"));

  // Lily ships no third-party URLs: each `href` builder is the site's, the same destinations as the CRM UI.
  const shareTargets: ShareTarget[] = $derived([
    {
      id: "email",
      label: t(locale, "share.email"),
      href: (url, title) => `mailto:?subject=${encodeURIComponent(title)}&body=${encodeURIComponent(url)}`,
      newTab: false,
    },
    {
      id: "linkedin",
      label: "LinkedIn",
      href: (url) => `https://www.linkedin.com/sharing/share-offsite/?url=${encodeURIComponent(url)}`,
    },
    {
      id: "reddit",
      label: "Reddit",
      href: (url, title) =>
        `https://www.reddit.com/submit?url=${encodeURIComponent(url)}&title=${encodeURIComponent(title)}`,
    },
    {
      id: "bluesky",
      label: "Bluesky",
      href: (url, title) => `https://bsky.app/intent/compose?text=${encodeURIComponent(`${title} ${url}`)}`,
    },
    {
      id: "mastodon",
      label: "Mastodon",
      href: (url, title) =>
        `https://mastodonshare.com/?text=${encodeURIComponent(title)}&url=${encodeURIComponent(url)}`,
    },
  ]);

  const themesUrl = $derived(`${href("/")}assets/themes/`.replace(/\/{2,}assets/, "/assets"));
</script>

<a class="skip" href="#main" lang={chromeLang}>{t(locale, "skip")}</a>
<header class="site-header" lang={chromeLang} {dir}>
  <a class="brand" href={href(homeRoute)}>{t(locale, "brand")}</a>
  <nav aria-label={t(locale, "site")}>
    <ul>
      {#each nav as item (item.route)}
        <li>
          <a href={href(item.route)} aria-current={current(item.route) ? "page" : undefined}>{item.label}</a>
        </li>
      {/each}
    </ul>
  </nav>
  <div class="pickers">
    <PickerBar
      labels={{
        link: t(locale, "pages"),
        search: t(locale, "search"),
        searchInput: t(locale, "search.input"),
        searchSubmit: t(locale, "search.submit"),
        theme: t(locale, "theme"),
        locale: t(locale, role ? "briefLanguage" : "language"),
        timeZone: t(locale, "timeZone"),
        textSize: t(locale, "textSize"),
        share: t(locale, "share"),
      }}
      links={pageLinks}
      linkProps={{ navigate: (to: string) => void goto(to) }}
      searchProps={{
        action: href("/search/"),
        placeholder: t(locale, "search.placeholder"),
        navigate: (to: string) => void goto(to),
      }}
      {themesUrl}
      themeProps={{
        storageKey: "lily-theme",
        // Without a default no theme stylesheet loads until the reader picks one.
        detectFromSystem: true,
        defaultValue: "light",
      }}
      locales={[...BRIEF_LOCALES]}
      localeProps={{ value: pickedLocale, localeLabels: LOCALE_LABELS, applyDir: false, onChange: switchLocale }}
      timeZoneProps={{ locale: htmlLang, storageKey: "lily-time-zone" }}
      textSizeProps={{ storageKey: "lily-text-size" }}
      {shareTargets}
      shareProps={{
        title: pageTitle,
        copyLabel: t(locale, "share.copy"),
        copiedLabel: t(locale, "share.copied"),
        copyFailedLabel: t(locale, "share.copyFailed"),
      }}
    />
  </div>
</header>

<main id="main" tabindex="-1">{@render children()}</main>

<footer class="site-footer" lang={chromeLang} {dir}>
  <p>
    {t(locale, "footer.before")} <a href={REPO_URL}>{t(locale, "footer.link")}</a>.
    {t(locale, "footer.after")}
  </p>
</footer>

<style>
  /* The wrapper div in app.html: a class, not an inline style, for the CSP. */
  :global(.app-root) {
    display: contents;
  }
  .skip {
    position: absolute;
    inset-inline-start: 0.5rem;
    inset-block-start: -3rem;
    padding: 0.5rem 0.75rem;
    background: var(--mxi-color-primary);
    color: var(--mxi-color-primary-fg);
    border-radius: var(--mxi-radius);
    z-index: 50;
  }
  .skip:focus {
    inset-block-start: 0.5rem;
  }
  .site-header {
    position: relative;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem 1.25rem;
    padding: 0.75rem 1.5rem;
    border-block-end: 1px solid var(--mxi-color-border);
    background: var(--mxi-color-surface);
  }
  .brand {
    font-weight: 700;
    color: var(--mxi-color-fg);
  }
  nav ul {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem 1rem;
    list-style: none;
    margin: 0;
    padding: 0;
  }
  /* The theme's own text on its own surface, not the link colour: in ten of Lily's light themes the mixed link colour
     falls under 4.5:1 on the header's surface. Position in the header says these are links. */
  nav a {
    display: inline-block;
    min-height: 24px;
    padding-block: 0.125rem;
    color: var(--mxi-color-fg);
  }
  nav a[aria-current="page"] {
    font-weight: 700;
    text-decoration: underline;
  }
  /* Lily PickerBar: the theme stylesheet styles the buttons and places each popup (CSS anchor positioning); as in the
     UI, this only lines the buttons up and sizes the popups. */
  .pickers {
    margin-inline-start: auto;
  }
  .pickers :global(.picker-bar) {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: 0.375rem;
  }
  .pickers :global(.link-picker),
  .pickers :global(.search-picker),
  .pickers :global(.theme-picker),
  .pickers :global(.locale-picker),
  .pickers :global(.time-zone-picker),
  .pickers :global(.text-size-picker),
  .pickers :global(.share-picker) {
    display: inline-flex;
    align-items: center;
  }
  .pickers :global(.link-picker-button),
  .pickers :global(.search-picker-button),
  .pickers :global(.theme-picker-button),
  .pickers :global(.locale-picker-button),
  .pickers :global(.time-zone-picker-button),
  .pickers :global(.text-size-picker-button),
  .pickers :global(.share-picker-button) {
    box-sizing: border-box;
    width: 2.5rem;
    height: 2.5rem;
    margin: 0;
  }
  .pickers :global(.link-picker-list),
  .pickers :global(.theme-picker-list),
  .pickers :global(.locale-picker-list),
  .pickers :global(.time-zone-picker-list),
  .pickers :global(.text-size-picker-list),
  .pickers :global(.share-picker-list) {
    z-index: 40;
    box-sizing: border-box;
    min-width: 0;
    width: max-content;
    max-width: calc(100vw - 2rem);
  }
  .pickers :global(.theme-picker-option),
  .pickers :global(.locale-picker-option),
  .pickers :global(.time-zone-picker-option),
  .pickers :global(.text-size-picker-option) {
    white-space: nowrap;
  }
  /* Lily's own hover rule matches `[class$="-button"]`, and the pickers write their class with a trailing space, so it
     never applies. The same colours, here. */
  .pickers :global(.link-picker-button:hover:not(:disabled)),
  .pickers :global(.search-picker-button:hover:not(:disabled)),
  .pickers :global(.theme-picker-button:hover:not(:disabled)),
  .pickers :global(.locale-picker-button:hover:not(:disabled)),
  .pickers :global(.time-zone-picker-button:hover:not(:disabled)),
  .pickers :global(.text-size-picker-button:hover:not(:disabled)),
  .pickers :global(.share-picker-button:hover:not(:disabled)) {
    background-color: var(--color-secondary);
    color: var(--color-secondary-content);
  }
  /* "Copied": a live region inside the share picker's root, lifted out of flow as in the UI so it does not push the
     button out of line. */
  .pickers :global(.share-picker-status) {
    position: absolute;
    inset-block-start: 100%;
    inset-inline-end: 1.5rem;
    z-index: 40;
    margin: 0.25rem 0 0;
    padding: 0.25rem 0.625rem;
    border-radius: var(--mxi-radius);
    background: var(--mxi-color-surface);
    border: 1px solid var(--mxi-color-border);
    font-size: 0.8125rem;
  }
  .pickers :global(.share-picker-status:empty) {
    display: none;
  }
  @media (max-width: 40rem) {
  .pickers :global(.theme-picker-option),
  .pickers :global(.locale-picker-option),
  .pickers :global(.time-zone-picker-option),
  .pickers :global(.text-size-picker-option) {
      white-space: normal;
    }
  }
  main {
    max-width: 60rem;
    margin-inline: auto;
    padding: 1.5rem;
  }
  main:focus {
    outline: none;
  }
  .site-footer {
    max-width: 60rem;
    margin-inline: auto;
    padding: 1rem 1.5rem 2rem;
    color: var(--mxi-color-muted);
    font-size: 0.875rem;
    border-block-start: 1px solid var(--mxi-color-border);
  }
</style>
