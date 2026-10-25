<script lang="ts">
  import { resource } from "#lib/resource.svelte.js";
  import { getLead, listLeads } from "#lib/api/crm.js";
  import DataTable from "#lib/components/DataTable.svelte";
  import { t } from "#lib/i18n.svelte.js";
  import type { ScoreBreakdown } from "#lib/api/crm.js";
  import ViewBar from "#lib/components/ViewBar.svelte";
  import { applyFilter, applyShape } from "#lib/views.js";
  import type { SavedView } from "#lib/api/crm.js";

  const list = resource(listLeads);
  const leads = $derived(list.value);
  const error = $derived(list.error);
  let selectedPid = $state<string | null>(null);
  let breakdown = $state<{ pid: string; score: ScoreBreakdown } | null>(null);

  const columns = $derived([
    { id: "score", kind: "number", total: "avg", header: t("lead.score") },
    {
      id: "display_name",
      header: t("common.name"),
      href: (row: Record<string, unknown>) => `/leads/${row.id}`,
    },
    { id: "source", header: t("lead.source") },
    { id: "status", kind: "chip", header: t("common.status") },
  ]);

  let view = $state<SavedView | null>(null);
  const rows = $derived(
    applyFilter(leads ?? [], view).map((l) => ({
      id: l.pid,
      score: l.score,
      display_name: l.display_name,
      source: l.source,
      status: l.status,
    })),
  );

  const shaped = $derived(applyShape(rows, columns, view));
  const headings = $derived(Object.fromEntries(columns.map((c) => [c.id, c.header])));

  // Explain the selected lead (defaults to the top-scored lead so the
  // button always has a subject).
  async function explain() {
    const pid = selectedPid ?? leads?.[0]?.pid;
    if (!pid) return;
    const detail = await getLead(pid);
    breakdown = { pid, score: detail.score };
  }
</script>

<h1>{t("nav.leads")}</h1>
<p>
  <a class="button" href="/leads/board">{t("common.board")}</a>
  <a class="button" href="/leads/import">{t("import.title")}</a>
</p>
<ViewBar kind="lead" bind:active={view} labels={headings} />

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if leads === null}
  <p>{t("common.loading")}</p>
{:else}
  <DataTable
    label={t("nav.leads")}
    columns={shaped.columns}
    rows={shaped.rows}
    onselect={(pid) => (selectedPid = pid)}
    testid="lead-queue"
  />
  <p><button onclick={() => void explain()}>{t("lead.breakdown")}</button></p>
  {#if breakdown}
    <div class="panel" data-testid="breakdown">
      <strong>{breakdown.score.score} · {breakdown.score.label}</strong>
      {#each breakdown.score.rules as rule (rule.rule)}
        <span class="chip">{rule.rule}: {rule.points}</span>
      {/each}
    </div>
  {/if}
{/if}

