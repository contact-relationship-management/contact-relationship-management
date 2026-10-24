<!--
  `/<locale>/kanban` — the signed-in user's deal board as a Lily kanban board
  (`@lilydesignsystem/svelte-kanban-board`): one column per pipeline stage, one
  card per deal. A card moves by pointer drag or by the keyboard-reachable
  "Move to column" menu; either way the deal's stage is saved through the same
  API call the deal board uses, and a refused move is undone and explained.

  The locale in the address is authoritative: opening `/de-de/kanban` shows the
  board in German (and remembers it as the UI language).
-->
<script lang="ts">
  import { resource } from "#lib/resource.svelte.js";
  import Board from "#lib/components/Board.svelte";
  import type {
    KanbanCard,
    KanbanColumn,
  } from "@lilydesignsystem/svelte-kanban-board";
  import { page } from "$app/state";
  import { listDeals, listPipelines, moveDeal, money, placeCard } from "#lib/api/crm.js";
  import type { Deal, Stage } from "#lib/api/crm.js";
  import { i18n, t } from "#lib/i18n.svelte.js";

  const board = resource(async () => {
    const first = (await listPipelines())[0];
    if (!first) return { pipeline: null, stages: [] as Stage[], deals: [] as Deal[] };
    return {
      pipeline: first.pipeline,
      stages: [...first.stages].sort((a, b) => a.position - b.position),
      deals: await listDeals(first.pipeline.pid),
    };
  });
  const pipeline = $derived(board.value?.pipeline ?? null);
  const stages = $derived(board.value?.stages ?? []);
  const deals = $derived(board.value?.deals ?? null);
  const error = $derived(board.error);

  // The address names the language.
  $effect(() => {
    const locale = page.params.locale;
    if (locale && locale !== i18n.locale) i18n.set(locale);
  });

  const columns = $derived<KanbanColumn[]>(
    stages.map((s) => ({ id: s.pid, title: `${s.name} ${s.probability_percent}%` })),
  );
  const cards = $derived<KanbanCard[]>(
    (deals ?? []).map((d) => ({
      id: d.pid,
      columnId: d.stage_pid,
      title: d.name,
      subtitle: money(d.amount_minor, d.currency, i18n.locale),
    })),
  );

  async function onPosition(pid: string, position: number) {
    await placeCard("deals", pid, position);
    await board.reload();
  }

  async function onMove(pid: string, stagePid: string) {
    await moveDeal(pid, stagePid);
    await board.reload();
  }
</script>

<svelte:head><title>{t("kanban.title")} — {t("brand.name")}</title></svelte:head>

<h1>{t("kanban.title")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if deals === null}
  <p>{t("common.loading")}</p>
{:else if stages.length === 0}
  <p class="muted" data-testid="kanban-empty">{t("kanban.empty")}</p>
{:else}
  {#if pipeline}<p class="muted">{pipeline.name}</p>{/if}
  <Board
    label={t("kanban.title")}
    {columns}
    {cards}
    {onMove}
    {onPosition}
    testid="kanban-board"
  />
{/if}
