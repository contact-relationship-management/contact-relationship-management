<script lang="ts">
  import { href } from "#lib/href.js";
  import { t } from "#lib/i18n.js";
  import { ENDONYMS, REPO_URL, TRANSLATIONS, isRtl, langTag } from "#lib/site.js";

  // A home page: `locales/<locale>/index.md`, cut at its first `##`. The hero has the headline, subheadline and
  // paragraph, the calls to action, and the sentence that names each translation in its own language; the sections
  // are styled from their Markdown (a list is a row of cards, a numbered list a row of steps). `locale` is null on
  // `/`, which is English.
  let { hero, body, locale = null }: { hero: string; body: string; locale?: string | null } = $props();
</script>

<!-- Rendered at build time from the repository's own Markdown, with raw HTML escaped. -->
<section class="landing-hero">
  <div class="doc">
    <!-- eslint-disable-next-line svelte/no-at-html-tags -->
    {@html hero}
  </div>
  <p class="calls">
    <a class="call primary" href={href("/tutorial/")}>{t(locale, "cta.tutorial")}</a>
    <a class="call" href={REPO_URL}>{t(locale, "cta.source")}</a>
  </p>
  <p class="translations">
    {t(locale, "translations")}
    {#each TRANSLATIONS as code, i (code)}
      {i > 0 ? ", " : ""}<a href={href(`/${code}/`)} lang={langTag(code)} dir={isRtl(code) ? "rtl" : "ltr"}
        >{ENDONYMS[code.split("-")[0] ?? ""]}</a
      >
    {/each}
  </p>
</section>
<div class="doc landing">
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html body}
</div>

<style>
  .landing-hero {
    padding: 2.5rem 2rem;
    margin-block-end: 2.5rem;
    border-radius: calc(var(--mxi-radius) * 2);
    background: var(--mxi-color-surface);
    border: 1px solid var(--mxi-color-border);
  }
  .landing-hero :global(h1) {
    font-size: clamp(2rem, 6vw, 3rem);
    margin-block-end: 0.5rem;
  }
  .landing-hero :global(h1 + p) {
    font-size: 1.375rem;
    font-weight: 600;
    line-height: 1.3;
    margin-block-start: 0;
  }
  .calls {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    margin-block: 1.5rem;
  }
  .call {
    display: inline-block;
    padding: 0.625rem 1.25rem;
    border-radius: var(--mxi-radius);
    border: 1px solid var(--mxi-color-fg);
    color: var(--mxi-color-fg);
    font-weight: 600;
    text-decoration: none;
  }
  .call:hover {
    text-decoration: underline;
  }
  .call.primary {
    background: var(--mxi-color-primary);
    border-color: var(--mxi-color-primary);
    color: var(--mxi-color-primary-fg);
  }
  .translations {
    margin-block-end: 0;
  }
  .landing :global(h2) {
    font-size: 1.625rem;
    margin-block-start: 2.5rem;
  }
  .landing :global(ul) {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(16rem, 100%), 1fr));
    gap: 1rem;
    list-style: none;
    padding: 0;
  }
  .landing :global(ul li) {
    padding: 1rem 1.25rem;
    border-radius: var(--mxi-radius);
    background: var(--mxi-color-surface);
    border: 1px solid var(--mxi-color-border);
  }
  .landing :global(li strong) {
    display: block;
    font-size: 1.0625rem;
    margin-block-end: 0.25rem;
  }
  .landing :global(ol) {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(12rem, 100%), 1fr));
    gap: 1rem;
    padding: 0;
    list-style-position: inside;
  }
  .landing :global(ol li::marker) {
    font-weight: 700;
  }
  .landing :global(ol li strong) {
    display: inline;
  }
</style>
