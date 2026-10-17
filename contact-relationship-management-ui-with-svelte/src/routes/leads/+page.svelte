<script lang="ts">
  import { getLead, listLeads } from "#lib/api/crm.js";
  import DataTable from "#lib/components/DataTable.svelte";
  import { t } from "#lib/i18n.svelte.js";
  import type { Lead, ScoreBreakdown } from "#lib/api/crm.js";

  let leads = $state<Lead[] | null>(null);
  let selectedPid = $state<string | null>(null);
  let breakdown = $state<{ pid: string; score: ScoreBreakdown } | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        leads = await listLeads();
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });

  const columns = $derived([
    { id: "score", kind: "number", total: "avg", header: t("lead.score") },
    { id: "display_name", header: t("common.name") },
    { id: "source", header: t("lead.source") },
    { id: "status", kind: "chip", header: t("common.status") },
  ]);

  const rows = $derived(
    (leads ?? []).map((l) => ({
      id: l.pid,
      score: l.score,
      display_name: l.display_name,
      source: l.source,
      status: l.status,
    })),
  );


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
<p><a class="button" href="/leads/board">{t("common.board")}</a></p>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if leads === null}
  <p>{t("common.loading")}</p>
{:else}
  <DataTable
    label={t("nav.leads")}
    {columns}
    {rows}
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

