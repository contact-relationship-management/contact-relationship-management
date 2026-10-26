<!--
  Help centre (`/help`): the published knowledge-base articles, open to anyone (CRM-R57). Search
  by words; each result opens the article. Nothing here needs a sign-in, and nothing a visitor
  types is kept: the service counts searches, not what was searched for.
-->
<svelte:head><title>{t("help.title")} · CRM</title></svelte:head>

<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import { kbSearch, type KbListing } from "#lib/api/crm.js";
  import { t } from "#lib/i18n.svelte.js";

  let query = $state("");
  let results = $state<KbListing[] | null>(null);
  let error = $state<string | null>(null);

  async function search(q: string) {
    error = null;
    try {
      results = await kbSearch(q);
    } catch (cause) {
      results = null;
      error = errorMessage(cause);
    }
  }

  $effect(() => {
    void search("");
  });
</script>

<h1>{t("help.title")}</h1>
<p class="muted">{t("help.hint")}</p>

<form
  class="panel"
  data-testid="help-form"
  onsubmit={(e) => {
    e.preventDefault();
    void search(query);
  }}
>
  <label>
    {t("help.search")}
    <input type="search" bind:value={query} maxlength="100" data-testid="help-query" />
  </label>
  <button type="submit" data-testid="help-run">{t("help.run")}</button>
</form>

{#if error}<p class="error" role="alert" data-testid="help-error">{error}</p>{/if}

{#if results}
  {#if results.length === 0}
    <p data-testid="help-none">{t("help.none")}</p>
  {:else}
    <ul class="help-list" data-testid="help-results">
      {#each results as article (article.pid)}
        <li data-testid="help-result">
          <a href={`/help/${article.pid}`}>{article.title}</a>
          <span class="muted small">{article.snippet}</span>
        </li>
      {/each}
    </ul>
  {/if}
{/if}

<style>
  .help-list {
    list-style: none;
    padding: 0;
    display: grid;
    gap: 0.75rem;
  }
  .help-list li {
    display: grid;
    gap: 0.15rem;
  }
</style>
