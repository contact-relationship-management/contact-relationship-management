<script lang="ts">
  import { t } from "#lib/i18n.js";
  import { REPO_URL } from "#lib/site.js";

  // `locale` is the page's brief locale (none on the English documents); the source line is in its language.
  let { html, path, locale = null }: { html: string; path: string; locale?: string | null } = $props();
</script>

<!-- The HTML is rendered at build time from the repository's own Markdown, with raw HTML escaped
     ($lib/server/content.ts), so nothing a reader supplies reaches it. -->
<article class="doc">
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html html}
</article>
<p class="source">
  {t(locale, "source")}
  <a href={`${REPO_URL}/blob/main/${path.split("/").map(encodeURIComponent).join("/")}`}><code dir="ltr">{path}</code></a>
</p>

<style>
  .source {
    margin-block-start: 2.5rem;
    padding-block-start: 0.75rem;
    border-block-start: 1px solid var(--mxi-color-border);
    color: var(--mxi-color-muted);
    font-size: 0.875rem;
  }
</style>
