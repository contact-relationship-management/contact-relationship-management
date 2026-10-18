<!--
  Lead triage board (`/leads/board`): the lead lifecycle as Lily
  Kanban columns; drag = the status transition (the service's
  lifecycle machine refuses illegal moves with 422 and the reload
  restores the stored truth). Cards carry the rule-derived score.
-->
<script lang="ts">
  import Board from "#lib/components/Board.svelte";
  import type {
    KanbanCard,
    KanbanLane,
  } from "@lilydesignsystem/svelte-kanban-board";
  import { editLead, leadStatus, listLeads, placeCard, type Lead } from "#lib/api/crm.js";
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
      laneId: lead.source,
      subtitle: `${lead.source} · score ${lead.score}`,
      values: { name: lead.display_name },
      locked: (NEXT[lead.status] ?? []).length === 0,
    })),
  );
  // Swimlanes: one per lead source.
  const lanes = $derived<KanbanLane[]>(
    [...new Set((leads ?? []).map((l) => l.source))]
      .sort()
      .map((id) => ({ id, title: id })),
  );
  async function onRename(id: string, title: string) {
    await editLead(id, { display_name: title });
    await load();
  }
  const onPosition = async (id: string, position: number) => {
    await placeCard("leads", id, position);
    await load();
  };
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
    {lanes}
    {onMove}
    {onPosition}
    {onRename}
    testid="lead-board"
  />
{/if}
