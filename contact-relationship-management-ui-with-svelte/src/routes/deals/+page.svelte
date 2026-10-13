<script lang="ts">
  import DataTable from "#lib/components/DataTable.svelte";
  import { Kanban, Willow, getCardShape } from "@svar-ui/svelte-kanban";
  import type { KanbanInstanceApi } from "@svar-ui/svelte-kanban";
  import { forecast, funnel, listDeals, listPipelines, money, moveDeal } from "#lib/api/crm.js";
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
  let actionError = $state<string | null>(null);

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
    stages.map((stage) => ({
      id: stage.pid,
      label: stageLabel(stage),
      addCard: false,
    })),
  );

  const cards = $derived(
    (deals ?? [])
      .filter((deal) => deal.closed_at === null)
      .map((deal) => ({
        id: deal.pid,
        label: deal.name,
        description: money(deal.amount_minor, deal.currency, i18n.locale),
        stage_pid: deal.stage_pid,
      })),
  );

  // Drag between stage columns = the stage-move API (a lost target
  // carries a reason; the service enforces pipeline membership and
  // terminal immutability). The forecast strip re-reads the derived
  // number after every move — never client math.
  function init(api: KanbanInstanceApi) {
    api.on("move-card", (raw) => {
      const ev = raw as { id: string | number; column?: string | number };
      if (!ev.column) return;
      const target = stages.find((stage) => stage.pid === String(ev.column));
      actionError = null;
      void moveDeal(
        String(ev.id),
        String(ev.column),
        target?.is_lost ? "moved to lost on the board" : undefined,
      )
        .catch((cause) => {
          actionError = cause instanceof Error ? cause.message : String(cause);
        })
        .finally(load);
    });
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
  {#if actionError}
    <p class="error" data-testid="action-error">{actionError}</p>
  {/if}
  <div class="board-wrap" data-testid="deal-board">
    <Willow>
      <Kanban
        {cards}
        {columns}
        columnAccessor="stage_pid"
        card={{ ...getCardShape(), menu: false }}
        {init}
      />
    </Willow>
  </div>
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

<style>
  .board-wrap {
    height: 620px;
    overflow-x: auto;
  }
</style>
