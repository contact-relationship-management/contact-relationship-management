<script lang="ts">
  import DataTable from "#lib/components/DataTable.svelte";
  import { articleStatus, listArticles } from "#lib/api/crm.js";
  import { t } from "#lib/i18n.svelte.js";
  import type { Article } from "#lib/api/crm.js";

  let articles = $state<Article[] | null>(null);
  let query = $state("");
  let error = $state<string | null>(null);

  async function load() {
    try {
      articles = await listArticles(query || undefined);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  $effect(() => {
    void load();
  });

  async function publish(article: Article) {
    await articleStatus(article.pid, "published");
    await load();
  }

  let selectedPid = $state<string | null>(null);
  const selected = $derived(
    (articles ?? []).find((a) => a.pid === selectedPid) ?? null,
  );

  const columns = $derived([
    { id: "title", header: t("common.name") },
    { id: "status", kind: "chip", header: t("common.status") },
    {
      id: "version",
      kind: "number",
      header: t("article.version"),
      format: (v: unknown) => `v${v}`,
    },
  ]);
  const rows = $derived(
    (articles ?? []).map((a) => ({
      id: a.pid,
      title: a.title,
      status: a.status,
      version: a.version,
    })),
  );
</script>

<h1>{t("nav.articles")}</h1>

<div class="panel">
  <label>
    {t("article.search")}
    <input bind:value={query} onchange={() => void load()} />
  </label>
</div>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if articles === null}
  <p>{t("common.loading")}</p>
{:else}
  <DataTable
    label={t("nav.articles")}
    {columns}
    {rows}
    onselect={(pid) => (selectedPid = pid)}
    testid="articles"
  />
  {#if selected}
    <div class="panel" data-testid="article-actions">
      <strong>{selected.title}</strong>
      <span class="chip">{selected.status}</span>
      {#if selected.status === "draft"}
        <button onclick={() => void publish(selected)}>{t("article.publish")}</button>
      {/if}
    </div>
  {/if}
{/if}
