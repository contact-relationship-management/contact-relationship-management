<script lang="ts">
  import { page } from "$app/state";
  import { href } from "#lib/href.js";
  import { t } from "#lib/i18n.js";
  import { search, type Entry } from "#lib/search.js";
  import { isRtl, langTag } from "#lib/site.js";

  // The query is the bare query string the header's SearchPicker writes (`/search/?consent`). It is read in an
  // effect, not during rendering: a prerendered page has no query string, and SvelteKit refuses to read one.
  let query = $state("");
  let entries = $state<Entry[] | null>(null);
  let failed = $state(false);

  $effect(() => {
    try {
      query = decodeURIComponent(page.url.search.slice(1));
    } catch {
      query = page.url.search.slice(1);
    }
  });

  // Fetched on the first search only: the index is the text of every page.
  $effect(() => {
    if (!query || entries || failed) return;
    fetch(href("/search-index.json"))
      .then((r) => {
        if (!r.ok) throw new Error(String(r.status));
        return r.json();
      })
      .then((data: Entry[]) => (entries = data))
      .catch(() => (failed = true));
  });

  const hits = $derived(entries ? search(entries, query) : []);
</script>

<svelte:head><title>{t(null, "search")} · Contact Relationship Management</title></svelte:head>

<h1>{t(null, "search")}{query ? `: ${query}` : ""}</h1>

{#if failed}
  <p role="alert">{t(null, "search.failed")}</p>
{:else if !query}
  <p>{t(null, "search.prompt")}</p>
{:else if entries}
  <p role="status">
    {hits.length === 0 ? t(null, "search.none") : t(null, "search.count").replace("{n}", String(hits.length))}
  </p>
  <ol class="results">
    {#each hits as hit (hit.entry.route)}
      {@const locale = hit.entry.locale}
      <li lang={locale ? langTag(locale) : "en"} dir={locale && isRtl(locale) ? "rtl" : "ltr"}>
        <a href={href(hit.entry.route)}>{hit.entry.title}</a>
        {#if locale}<span class="locale">{locale}</span>{/if}
        <p>{hit.snippet}</p>
      </li>
    {/each}
  </ol>
{/if}

<style>
  .results {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .results li {
    margin-block: 1.25rem;
  }
  .results a {
    font-weight: 600;
    text-decoration: underline;
  }
  .results p {
    margin-block: 0.25rem 0;
    overflow-wrap: anywhere;
  }
  .locale {
    color: var(--mxi-color-muted);
    font-size: 0.875rem;
    margin-inline-start: 0.5rem;
  }
</style>
