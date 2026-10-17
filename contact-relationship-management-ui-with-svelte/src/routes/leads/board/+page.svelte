<!--
  Lead triage board (`/leads/board`): the lead lifecycle as SVAR
  Kanban columns; drag = the status transition (the service's
  lifecycle machine refuses illegal moves with 422 and the reload
  restores the stored truth). Cards carry the rule-derived score.
-->
<script lang="ts">
  import Board from "#lib/components/Board.svelte";
  import type { KanbanCard } from "@lilydesignsystem/svelte-kanban-board";
  import { leadStatus, listLeads, type Lead } from "#lib/api/crm.js";
  import { t } from "#lib/i18n.svelte.js";

  let leads = $state<Lead[] | null>(null);
  let error = $state<string | null>(null);

  async function load() {
    try {
      leads = await listLeads();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }
  $effect(() => {
    void load();
  });

  const columns = [
    { id: "new", title: "New" },
    { id: "contacted", title: "Contacted" },
    { id: "qualified", title: "Qualified" },
    { id: "converted", title: "Converted" },
    { id: "disqualified", title: "Disqualified" },
  ];

  // The service's lead lifecycle (CRM-R3): the board offers only legal moves.
  const NEXT: Record<string, string[]> = {
    new: ["contacted", "disqualified"],
    contacted: ["qualified", "disqualified"],
    qualified: ["converted", "disqualified"],
    converted: [],
    disqualified: [],
  };

  const cards = $derived<KanbanCard[]>(
    (leads ?? []).map((lead) => ({
      id: lead.pid,
      columnId: lead.status,
      title: lead.display_name,
      subtitle: `${lead.source} · score ${lead.score}`,
      locked: (NEXT[lead.status] ?? []).length === 0,
    })),
  );
  const allowedColumns = (card: KanbanCard) => NEXT[card.columnId] ?? [];

  // Show the move at once; the board undoes it if the service refuses.
  async function onMove(id: string, status: string) {
    await leadStatus(id, status);
    await load();
  }
</script>

<svelte:head><title>{t("nav.leads")} — CRM</title></svelte:head>

<h1>{t("nav.leads")} {t("common.board")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if leads === null}
  <p>{t("common.loading")}</p>
{:else}
  <Board
    label={`${t("nav.leads")} ${t("common.board")}`}
    {columns}
    {cards}
    {allowedColumns}
    {onMove}
    testid="lead-board"
  />
{/if}
