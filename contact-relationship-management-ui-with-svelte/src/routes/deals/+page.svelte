<script lang="ts">
  import { resource } from "#lib/resource.svelte.js";
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
    moveDeal,
    placeCard,
  } from "#lib/api/crm.js";
  import { minorToInput, minorToMajor, money, toMinor } from "#lib/money.js";
  import { i18n, t } from "#lib/i18n.svelte.js";
  import { ratioFraction, ratioPercent, stageLabel } from "#lib/format.js";
  import type { Deal, Stage } from "#lib/api/crm.js";
  import ViewBar from "#lib/components/ViewBar.svelte";
  import { applyFilter, applyShape } from "#lib/views.js";
  import type { SavedView } from "#lib/api/crm.js";

  /** A currency to show the forecast in too (CRM-R53); empty shows per-currency totals only. */
  let showIn = $state("");
  const converted = resource(async () => {
    const to = showIn.trim().toUpperCase();
    return /^[A-Z]{3}$/.test(to) ? await forecast({ convertTo: to }) : null;
  });

  /** The pipeline the visitor picked; empty means the first. */
  let chosen = $state("");
  const board = resource(
    async () => {
      const want = chosen;
      const pipelines = await listPipelines();
      const active = pipelines.find((p) => p.pipeline.pid === want) ?? pipelines[0];
      const deals = active ? await listDeals(active.pipeline.pid) : [];
      const totals = (await forecast()).totals_minor ?? {};
      const stageFunnel = active ? await funnel(active.pipeline.pid) : null;
      return { pipelines, active, deals, totals, stageFunnel };
    },
    { keepWhileLoading: true },
  );
  const stages = $derived<Stage[]>(board.value?.active?.stages ?? []);
  const deals = $derived<Deal[] | null>(board.value?.deals ?? null);
  const totals = $derived<Record<string, number>>(board.value?.totals ?? {});
  const pipelines = $derived(board.value?.pipelines ?? []);
  const selectedPipeline = $derived(board.value?.active?.pipeline.pid ?? chosen);
  const stageFunnel = $derived(board.value?.stageFunnel ?? null);
  const error = $derived(board.error);

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
        // The amount as the page's own number format types it, in the deal's own minor unit.
        amount:
          deal.amount_minor === null
            ? null
            : minorToInput(deal.amount_minor, deal.currency, i18n.locale),
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

  // The amount shows with its own currency and edits as text in the page's own number format
  // ("1234,50" in German, "1500" yen), so a decimal comma is accepted (CRM-T167). A deal with no
  // currency shows a dash: none is assumed (CRM-T168).
  const minorOf = (value: unknown, currency: unknown) =>
    typeof value === "string" && typeof currency === "string" && currency
      ? toMinor(value, currency, i18n.locale)
      : null;
  const fields = $derived<KanbanField[]>([
    {
      id: "amount",
      header: t("kanban.amount"),
      type: "text",
      editable: true,
      format: (value: unknown, row: Record<string, unknown>) =>
        money(minorOf(value, row.currency), String(row.currency ?? ""), i18n.locale),
      validate: (value: unknown, values: Record<string, unknown>) =>
        minorOf(value, values.currency) === null ? t("money.amountInvalid") : undefined,
    },
    { id: "close", header: t("kanban.closeOn"), type: "date", editable: true },
  ]);

  async function onEdit(edit: KanbanFieldEdit) {
    const currency = openDeals.find((deal) => deal.pid === edit.cardId)?.currency;
    const minor = edit.fieldId === "amount" ? minorOf(edit.value, currency) : null;
    if (minor !== null) {
      await editDeal(edit.cardId, { amount_minor: minor });
    } else if (edit.fieldId === "close" && typeof edit.value === "string") {
      await editDeal(edit.cardId, { expected_close_on: edit.value });
    }
    await board.reload();
  }
  async function onRename(id: string, name: string) {
    await editDeal(id, { name });
    await board.reload();
  }
  const onPosition = async (id: string, position: number) => {
    await placeCard("deals", id, position);
    await board.reload();
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
    await board.reload();
  }

  // The plain list of deals can be viewed through a saved view (CRM-R35). A masked amount is
  // absent, so a view cannot be used to find out what it hides.
  let view = $state<SavedView | null>(null);
  const dealColumns = $derived([
    {
      id: "name",
      header: t("common.name"),
      href: (row: Record<string, unknown>) => `/deals/${row.id}`,
    },
    { id: "amount_minor", kind: "number", header: t("kanban.amount") },
    { id: "currency", header: "Currency" },
    { id: "state", kind: "chip", header: t("common.status") },
    { id: "expected_close_on", kind: "date", header: t("kanban.closeOn") },
  ]);
  const dealRows = $derived(
    applyFilter(deals ?? [], view).map((deal) => ({
      id: deal.pid,
      name: deal.name,
      // Major units of the deal's own currency, so the column sorts as numbers.
      amount_minor:
        deal.amount_minor === null ? "" : minorToMajor(deal.amount_minor, deal.currency, i18n.locale),
      currency: deal.currency,
      state: deal.closed_at === null ? "open" : deal.won ? "won" : "lost",
      expected_close_on: deal.expected_close_on ?? "",
    })),
  );
  const shaped = $derived(applyShape(dealRows, dealColumns, view));
  const headings = $derived(Object.fromEntries(dealColumns.map((c) => [c.id, c.header])));

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
      aria-label={t("rpt.field.pipeline")}
      data-testid="pipeline-select"
      value={selectedPipeline}
      onchange={(event) => {
        chosen = event.currentTarget.value;
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
  <label>
    {t("rpt.convertTo")}
    <input bind:value={showIn} maxlength="3" size="4" data-testid="forecast-convert" />
  </label>
</p>
{#if converted.value?.converted_minor != null}
  <p data-testid="forecast-converted">
    {t("dash.forecast")} ({converted.value.convert_to}):
    <strong>{money(converted.value.converted_minor, converted.value.convert_to!, i18n.locale)}</strong>
    {#each converted.value.rates as used (used.from)}
      <span class="chip muted small">
        1 {used.from} = {used.rate}
        {used.to}, {used.effective_on}, {used.source}
        {#if used.inverted}({t("rpt.inverted")}){/if}
      </span>
    {/each}
    {#each converted.value.unconverted as left (left.currency)}
      <span class="chip">
        {t("rpt.unconverted")}: {left.currency}
        {money(left.amount_minor, left.currency, i18n.locale)}
      </span>
    {/each}
  </p>
{/if}

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

{#if deals !== null && deals.length > 0}
  <h2>{t("nav.deals")}</h2>
  <ViewBar kind="deal" bind:active={view} labels={headings} />
  <DataTable
    label={t("nav.deals")}
    columns={shaped.columns}
    rows={shaped.rows}
    testid="deal-list"
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
