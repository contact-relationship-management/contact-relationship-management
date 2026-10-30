<script lang="ts">
  import { resource } from "#lib/resource.svelte.js";
  import DataTable from "#lib/components/DataTable.svelte";
  import { articleStatus, kbDeflection, listArticles } from "#lib/api/crm.js";
  import { t } from "#lib/i18n.svelte.js";
  import { formatPercent } from "#lib/locale-format.js";
  import type { Article } from "#lib/api/crm.js";

  let query = $state("");
  // The search runs when the box is committed, not on every keystroke.
  let applied = $state("");
  const list = resource(() => listArticles(applied || undefined));
  const articles = $derived(list.value);
  const error = $derived(list.error);

  async function publish(article: Article) {
    await articleStatus(article.pid, "published");
    await list.reload();
  }

  // What the public help centre has done lately (CRM-R57); a reader without the right gets nothing.
  const deflection = resource(() => kbDeflection(30));
  const stats = $derived(deflection.value);
  const percent = (r: { value: number | null }) =>
    formatPercent(r.value);

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
    <input bind:value={query} onchange={() => (applied = query)} />
  </label>
</div>

{#if stats}
  <div class="panel" data-testid="kb-deflection">
    <strong>{t("kb.title")}</strong>
    <span class="muted small">({stats.days} {t("kb.days")})</span>
    <ul>
      <li>{t("kb.views")}: {stats.views}</li>
      <li>
        {t("kb.searches")}: {stats.searches}
        ({t("kb.noResult")}: {stats.searches_without_result})
      </li>
      <li>{t("kb.helped")}: {stats.helpful_yes} · {t("kb.notHelped")}: {stats.helpful_no}</li>
      <li>{t("kb.tickets")}: {stats.tickets_opened}</li>
      <li>
        {t("kb.deflection")}: <strong>{percent(stats.deflection)}</strong>
        <span class="muted small">
          ({stats.deflection.numerator} / {stats.deflection.denominator})
        </span>
      </li>
    </ul>
    <p class="muted small">{stats.note}</p>
  </div>
{/if}

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
