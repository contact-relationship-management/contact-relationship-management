<script lang="ts">
  import "../app.css";
  import type { Snippet } from "svelte";
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import ThemePicker from "@lilydesignsystem/svelte-theme-picker";
  import LocalePicker from "@lilydesignsystem/svelte-locale-picker";
  import TextSizePicker from "@lilydesignsystem/svelte-text-size-picker";
  import { href } from "#lib/href.js";
  import { t, languageOf } from "#lib/i18n.js";
  import {
    BRIEF_LOCALES,
    DEFAULT_BRIEF_LOCALE,
    DOC_PAGES,
    REPO_URL,
    briefAt,
    isBriefLocale,
    isRtl,
    langTag,
    peerIn,
  } from "#lib/site.js";

  // Lily's seven-step scale, largest first, as the UI's PickerBar uses it (app.css has the rules).
  const TEXT_SIZES = ["largest", "larger", "large", "normal", "small", "smaller", "smallest"];

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

  // The locale switcher opens the same brief (or the same list) in the chosen locale. The picker also reports the
  // locale it starts on, which is this page's own: nothing to do then. Slugs are in each locale's own script, so
  // the same role is found by its peer id.
  function switchLocale(code: string) {
    if (!isBriefLocale(code) || code === locale) return;
    const here = locale && role ? briefAt(locale, role) : undefined;
    const there = here ? peerIn(code, here.peer) : undefined;
    void goto(href(there ? `/${code}/for/${there.slug}/` : `/${code}/for/`));
  }

  const nav = $derived([
    ...DOC_PAGES.map((d) => ({ label: t(locale, `nav.${d.nav}`), route: d.slug ? `/${d.slug}/` : "/" })),
    { label: t(locale, "nav.briefs"), route: `/${locale ?? DEFAULT_BRIEF_LOCALE}/for/` },
  ]);
  function current(route: string): boolean {
    const here = page.url.pathname;
    const target = href(route);
    if (route.endsWith("/for/")) return /\/for\//.test(here);
    return here === target || here === new URL(target, page.url.href).pathname;
  }

  const themesUrl = $derived(`${href("/")}assets/themes/`.replace(/\/{2,}assets/, "/assets"));
</script>

<a class="skip" href="#main" lang={chromeLang}>{t(locale, "skip")}</a>
<header class="site-header" lang={chromeLang} {dir}>
  <a class="brand" href={href("/")} lang="en">Contact Relationship Management</a>
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
    <ThemePicker
      label={t(locale, "theme")}
      {themesUrl}
      themes={["light", "dark"]}
      storageKey="lily-theme"
      detectFromSystem
      defaultValue="light"
    />
    {#if locale}
      <LocalePicker
        label={t(locale, "briefLanguage")}
        locales={[...BRIEF_LOCALES]}
        value={locale}
        applyDir={false}
        onChange={switchLocale}
      />
    {/if}
    <TextSizePicker label={t(locale, "textSize")} sizes={TEXT_SIZES} defaultValue="normal" storageKey="lily-text-size" />
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
  nav a {
    display: inline-block;
    min-height: 24px;
    padding-block: 0.125rem;
  }
  nav a[aria-current="page"] {
    font-weight: 700;
    text-decoration: underline;
  }
  /* Lily pickers: the theme stylesheet styles the buttons and places each popup (CSS anchor positioning); as in the
     UI, this only lines the buttons up and sizes the popups. */
  .pickers {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    margin-inline-start: auto;
  }
  .pickers :global(.theme-picker),
  .pickers :global(.locale-picker),
  .pickers :global(.text-size-picker) {
    display: inline-flex;
    align-items: center;
  }
  .pickers :global(.theme-picker-button),
  .pickers :global(.locale-picker-button),
  .pickers :global(.text-size-picker-button) {
    box-sizing: border-box;
    width: 2.5rem;
    height: 2.5rem;
    margin: 0;
  }
  .pickers :global(.theme-picker-list),
  .pickers :global(.locale-picker-list),
  .pickers :global(.text-size-picker-list) {
    z-index: 40;
    box-sizing: border-box;
    min-width: 0;
    width: max-content;
    max-width: calc(100vw - 2rem);
  }
  .pickers :global(.theme-picker-option),
  .pickers :global(.locale-picker-option),
  .pickers :global(.text-size-picker-option) {
    white-space: nowrap;
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
