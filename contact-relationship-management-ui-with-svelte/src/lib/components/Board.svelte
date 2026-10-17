<!--
  Board — the app's one kanban board, a thin wrapper over the Lily
  `@lilydesignsystem/svelte-kanban-board`. Cards move by pointer drag or by the
  keyboard-reachable "Move to column" menu. `onMove` may be async: the card
  shows in its new column at once and goes back, with the reason announced and
  shown, when the promise rejects. Every user-facing string comes from the
  i18n catalog (`kanban.*`), because Lily ships none.

  Props:
    - label            accessible name for the board
    - columns          { id, title, flag?, accent? }[]
    - cards            { id, columnId, title, subtitle?, badges?, flag?, href? }[]
    - onMove           (cardId, toColumnId) => Promise — saves the move
    - allowedColumns   (card) => column ids the card may move to (a workflow)
    - columnFooter     snippet under each column (for example a total)
    - testid           `data-testid` on the board
-->
<script lang="ts">
  import KanbanBoard from "@lilydesignsystem/svelte-kanban-board";
  import type {
    KanbanCard,
    KanbanColumn,
    KanbanColumnContext,
  } from "@lilydesignsystem/svelte-kanban-board";
  import type { Snippet } from "svelte";
  import { t, type StringKey } from "#lib/i18n.svelte.js";

  let {
    label,
    columns,
    cards,
    onMove,
    allowedColumns,
    columnFooter,
    testid,
  }: {
    label: string;
    columns: KanbanColumn[];
    cards: KanbanCard[];
    onMove?: (cardId: string, toColumnId: string) => Promise<unknown>;
    allowedColumns?: (card: KanbanCard) => string[];
    columnFooter?: Snippet<[KanbanColumnContext]>;
    testid?: string;
  } = $props();

  let search = $state("");
  let collapsedColumns = $state<string[]>([]);
  let moveError = $state<string | null>(null);

  const fill = (key: StringKey, values: Record<string, string | number>) =>
    Object.entries(values).reduce(
      (text, [name, value]) => text.replaceAll(`{${name}}`, String(value)),
      t(key),
    );

  const labels = $derived({
    cardCount: (count: number) =>
      count === 1 ? t("kanban.cardsOne") : fill("kanban.cards", { count }),
    moveButton: (card: KanbanCard) => fill("kanban.move", { title: card.title }),
    moveMenuLabel: t("kanban.menu"),
    moveAnnouncement: (title: string, column: string) =>
      fill("kanban.moved", { title, column }),
    movePending: (title: string, column: string) =>
      fill("kanban.pending", { title, column }),
    moveFailed: (title: string, column: string) =>
      fill("kanban.undone", { title, column }),
    moveBlocked: (title: string, column: string) =>
      fill("kanban.blocked", { title, column }),
    emptyColumn: () => t("kanban.emptyColumn"),
    search: t("kanban.search"),
    collapseColumn: (column: KanbanColumn, collapsed: boolean) =>
      fill(collapsed ? "kanban.expand" : "kanban.collapse", {
        column: column.title,
      }),
    export: t("kanban.export"),
    exportCsv: "CSV",
    exportExcel: "Excel",
    exportAnnouncement: (_format: unknown, count: number, filename: string) =>
      fill("kanban.exported", { count, filename }),
    exportHeaders: {
      title: t("kanban.title.col"),
      column: t("kanban.column.col"),
    },
  });

  function handleMove(cardId: string, toColumnId: string) {
    moveError = null;
    return onMove?.(cardId, toColumnId);
  }

  function handleError(_id: string, _to: string, cause: unknown) {
    moveError = cause instanceof Error ? cause.message : t("kanban.failed");
  }
</script>

{#if moveError}
  <p class="error" role="alert" data-testid="action-error">{moveError}</p>
{/if}
<KanbanBoard
  {label}
  {columns}
  {cards}
  {labels}
  {allowedColumns}
  bind:search
  bind:collapsedColumns
  onMove={handleMove}
  onMoveError={handleError}
  {columnFooter}
  data-testid={testid}
/>
