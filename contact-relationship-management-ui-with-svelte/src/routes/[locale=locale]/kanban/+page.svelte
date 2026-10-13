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
  import KanbanBoard from "@lilydesignsystem/svelte-kanban-board";
  import type {
    KanbanCard,
    KanbanColumn,
  } from "@lilydesignsystem/svelte-kanban-board";
  import { page } from "$app/state";
  import { listDeals, listPipelines, moveDeal, money } from "#lib/api/crm.js";
  import type { Deal, Stage } from "#lib/api/crm.js";
  import { i18n, t } from "#lib/i18n.svelte.js";

  let pipeline = $state<{ pid: string; name: string } | null>(null);
  let stages = $state<Stage[]>([]);
  let deals = $state<Deal[] | null>(null);
  let error = $state<string | null>(null);
  let moveError = $state<string | null>(null);

  // The address names the language.
  $effect(() => {
    const locale = page.params.locale;
    if (locale && locale !== i18n.locale) i18n.set(locale);
  });

  async function load() {
    try {
      const pipelines = await listPipelines();
      const first = pipelines[0];
      if (!first) {
        deals = [];
        return;
      }
      pipeline = first.pipeline;
      stages = [...first.stages].sort((a, b) => a.position - b.position);
      deals = await listDeals(first.pipeline.pid);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  $effect(() => {
    void load();
  });

  const columns = $derived<KanbanColumn[]>(
    stages.map((s) => ({ id: s.pid, title: `${s.name} ${s.probability_percent}%` })),
  );
  const cards = $derived<KanbanCard[]>(
    (deals ?? []).map((d) => ({
      id: d.pid,
      columnId: d.stage_pid,
      title: `${d.name} · ${money(d.amount_minor, d.currency, i18n.locale)}`,
    })),
  );

  const fill = (text: string, values: Record<string, string | number>) =>
    Object.entries(values).reduce(
      (out, [name, value]) => out.replaceAll(`{${name}}`, String(value)),
      text,
    );

  const labels = $derived({
    cardCount: (count: number) =>
      count === 1 ? t("kanban.cardsOne") : fill(t("kanban.cards"), { count }),
    moveButton: (card: KanbanCard) => fill(t("kanban.move"), { title: card.title }),
    moveMenuLabel: t("kanban.menu"),
    moveAnnouncement: (title: string, column: string) =>
      fill(t("kanban.moved"), { title, column }),
  });

  async function onMove(pid: string, stagePid: string) {
    moveError = null;
    const before = deals;
    // Show the move at once; undo it if the service refuses.
    deals = (deals ?? []).map((d) =>
      d.pid === pid ? { ...d, stage_pid: stagePid } : d,
    );
    try {
      await moveDeal(pid, stagePid);
    } catch {
      deals = before;
      moveError = t("kanban.failed");
      await load();
    }
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
  {#if moveError}
    <p class="error" role="alert" data-testid="move-error">{moveError}</p>
  {/if}
  <KanbanBoard
    label={t("kanban.title")}
    {columns}
    {cards}
    {labels}
    {onMove}
    data-testid="kanban-board"
  />
{/if}
