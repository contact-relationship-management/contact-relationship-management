<script lang="ts">
  import DataTable from "#lib/components/DataTable.svelte";
  import Board from "#lib/components/Board.svelte";
  import type {
    KanbanCard,
    KanbanField,
    KanbanFieldEdit,
    KanbanLane,
  } from "@lilydesignsystem/svelte-kanban-board";
  import {
    editDeal,
    forecast,
    funnel,
    listDeals,
    listPipelines,
    money,
    moveDeal,
    placeCard,
  } from "#lib/api/crm.js";
  import { i18n, t } from "#lib/i18n.svelte.js";
  import { ratioFraction, ratioPercent, stageLabel } from "#lib/format.js";
  import type { Deal, Stage } from "#lib/api/crm.js";

  let stages = $state<Stage[]>([]);
  let deals = $state<Deal[] | null>(null);
  let totals = $state<Record<string, number>>({});
  let pipelines = $state<Awaited<ReturnType<typeof listPipelines>>>([]);
  let selectedPipeline = $state("");
  let stageFunnel = $state<Awaited<ReturnType<typeof funnel>> | null>(null);
  let error = $state<string | null>(null);

  async function load() {
    try {
      pipelines = await listPipelines();
      const active =
        pipelines.find((p) => p.pipeline.pid === selectedPipeline) ?? pipelines[0];
      selectedPipeline = active?.pipeline.pid ?? "";
      stages = active?.stages ?? [];
      deals = active ? await listDeals(active.pipeline.pid) : [];
      totals = (await forecast()).totals_minor ?? {};
      stageFunnel = active ? await funnel(active.pipeline.pid) : null;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  $effect(() => {
    void load();
  });

  // Columns are the pipeline's stage rows — data, not a token table
  // (CRM-D3); terminal stages are drop targets that close the deal.
  const columns = $derived(
    stages.map((stage) => ({ id: stage.pid, title: stageLabel(stage) })),
  );

  const openDeals = $derived((deals ?? []).filter((deal) => deal.closed_at === null));
  const UNASSIGNED = "__unassigned";
  const cards = $derived<KanbanCard[]>(
    openDeals.map((deal) => ({
      id: deal.pid,
      columnId: deal.stage_pid,
      laneId: deal.owner_ref ?? UNASSIGNED,
      title: deal.name,
      values: {
        amount: deal.amount_minor === null ? null : deal.amount_minor / 100,
        close: deal.expected_close_on ?? null,
        currency: deal.currency,
      },
    })),
  );

  // Swimlanes: one per owner (a deal's `owner_ref`).
  const lanes = $derived<KanbanLane[]>(
    [...new Set(openDeals.map((d) => d.owner_ref ?? UNASSIGNED))]
      .sort()
      .map((id) => ({ id, title: id === UNASSIGNED ? t("kanban.unassigned") : id })),
  );

  // The amount shows with its own currency and edits in major units.
  const fields = $derived<KanbanField[]>([
    {
      id: "amount",
      header: t("kanban.amount"),
      type: "number",
      editable: true,
      format: (value: unknown, row: Record<string, unknown>) =>
        value === null || value === undefined
          ? "—"
          : money(Math.round(Number(value) * 100), String(row.currency ?? "GBP"), i18n.locale),
    },
    { id: "close", header: t("kanban.closeOn"), type: "date", editable: true },
  ]);

  async function onEdit(edit: KanbanFieldEdit) {
    if (edit.fieldId === "amount" && typeof edit.value === "number") {
      await editDeal(edit.cardId, { amount_minor: Math.round(edit.value * 100) });
    } else if (edit.fieldId === "close" && typeof edit.value === "string") {
      await editDeal(edit.cardId, { expected_close_on: edit.value });
    }
    await load();
  }
  async function onRename(id: string, name: string) {
    await editDeal(id, { name });
    await load();
  }
  const onPosition = async (id: string, position: number) => {
    await placeCard("deals", id, position);
    await load();
  };

  // Drag between stage columns = the stage-move API (a lost target
  // carries a reason; the service enforces pipeline membership and
  // terminal immutability). The forecast strip re-reads the derived
  // number after every move — never client math. A refusal puts the card back.
  async function onMove(id: string, stagePid: string) {
    const target = stages.find((stage) => stage.pid === stagePid);
    await moveDeal(
      id,
      stagePid,
      target?.is_lost ? "moved to lost on the board" : undefined,
    );
    await load();
  }

  const funnelColumns = [
    { id: "stage", header: "Stage" },
    { id: "entered", kind: "number", header: "Entered" },
    { id: "conversion", header: "Conversion" },
  ];
  const funnelRows = $derived(
    (stageFunnel?.stages ?? []).map((row) => ({
      id: row.stage,
      stage: `${row.stage}${row.is_won ? " ✓" : row.is_lost ? " ✗" : ""}`,
      entered: row.entered,
      conversion: row.conversion_from_previous
        ? `${ratioPercent(row.conversion_from_previous)} (${ratioFraction(row.conversion_from_previous)})`
        : "—",
    })),
  );
</script>

<h1>{t("deal.board")}</h1>

<p>
  <label>
    <select
      data-testid="pipeline-select"
      value={selectedPipeline}
      onchange={(event) => {
        selectedPipeline = event.currentTarget.value;
        void load();
      }}
    >
      {#each pipelines as entry (entry.pipeline.pid)}
        <option value={entry.pipeline.pid}>{entry.pipeline.name}</option>
      {/each}
    </select>
  </label>
</p>

<p data-testid="forecast">
  {t("dash.forecast")}:
  {#each Object.entries(totals) as [currency, minor] (currency)}
    <strong>{money(minor, currency, i18n.locale)}</strong>&nbsp;
  {:else}
    <span class="muted">{t("dash.noData")}</span>
  {/each}
</p>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if deals === null}
  <p>{t("common.loading")}</p>
{:else}
  <Board
    label={t("deal.board")}
    {columns}
    {cards}
    {lanes}
    {fields}
    {onMove}
    {onPosition}
    {onEdit}
    {onRename}
    testid="deal-board"
  />
{/if}

{#if stageFunnel}
  <h2>Funnel — {stageFunnel.pipeline.name}</h2>
  <p class="muted">{stageFunnel.derivation}</p>
  <DataTable
    label="Stage funnel"
    columns={funnelColumns}
    rows={funnelRows}
    testid="deal-funnel"
  />
{/if}
