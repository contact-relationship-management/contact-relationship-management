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
    - onPosition       (cardId, position) => Promise — stores the card's place
    - fields           typed values shown under a card; an editable one edits
                       in place (click it, or F2) and calls `onEdit`
    - lanes            swimlanes; a card's `laneId` names its lane. Lanes only
                       group: dropping a card into another lane is refused
    - undo             Ctrl/Cmd+Z takes back a move or an edit (Ctrl+Shift+Z redoes)
    - several cards    Ctrl/Cmd+click (or `x`) selects; moving one moves all
    - allowedColumns   (card) => column ids the card may move to (a workflow)
    - columnFooter     snippet under each column (for example a total)
    - testid           `data-testid` on the board
-->
<script lang="ts">
  import { packLabels } from "#lib/lily-labels.js";
  import KanbanBoard from "@lilydesignsystem/svelte-kanban-board";
  import type {
    KanbanCard,
    KanbanColumn,
    KanbanColumnContext,
    KanbanField,
    KanbanFieldEdit,
    KanbanLane,
  } from "@lilydesignsystem/svelte-kanban-board";
  import type { Snippet } from "svelte";
  import { i18n, t, type StringKey } from "#lib/i18n.svelte.js";

  let {
    label,
    columns,
    cards,
    onMove,
    onPosition,
    onEdit,
    fields,
    lanes,
    allowedColumns,
    columnFooter,
    testid,
  }: {
    label: string;
    columns: KanbanColumn[];
    cards: KanbanCard[];
    onMove?: (cardId: string, toColumnId: string) => Promise<unknown>;
    onPosition?: (cardId: string, position: number) => Promise<unknown>;
    onEdit?: (edit: KanbanFieldEdit) => Promise<unknown>;
    fields?: KanbanField[];
    lanes?: KanbanLane[];
    allowedColumns?: (card: KanbanCard) => string[];
    columnFooter?: Snippet<[KanbanColumnContext]>;
    testid?: string;
  } = $props();

  let search = $state("");
  let selected = $state<string[]>([]);
  let collapsedLanes = $state<string[]>([]);
  let collapsedColumns = $state<string[]>([]);
  let moveError = $state<string | null>(null);

  const fill = (key: StringKey, values: Record<string, string | number>) =>
    Object.entries(values).reduce(
      (text, [name, value]) => text.replaceAll(`{${name}}`, String(value)),
      t(key),
    );

  // The pack supplies the generic wording; what follows is the app's own and wins.
  const labels = $derived({
    ...packLabels("kanban-board", i18n.locale),
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
    laneCount: (_lane: KanbanLane, count: number) =>
      count === 1 ? t("kanban.cardsOne") : fill("kanban.cards", { count }),
    collapseLane: (lane: KanbanLane, collapsed: boolean) =>
      fill(collapsed ? "kanban.expand" : "kanban.collapse", { column: lane.title }),
    moveManyAnnouncement: (count: number, column: string) =>
      fill("kanban.movedMany", { count, column }),
    editAnnouncement: (title: string, field: string, text: string) =>
      fill("kanban.edited", { title, field, text }),
    editInvalid: (title: string, field: string) =>
      fill("kanban.editInvalid", { title, field }),
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

  // A drop is a move (a different column), a place (where in the column),
  // or both. A drop into another lane is refused: lanes only group.
  async function handleMove(
    cardId: string,
    toColumnId: string,
    toIndex?: number,
    toLaneId?: string,
  ) {
    moveError = null;
    const card = cards.find((c) => c.id === cardId);
    const lane = card?.laneId ?? lanes?.[0]?.id;
    if (toLaneId !== undefined && lane !== undefined && toLaneId !== lane) {
      throw new Error(t("kanban.laneLocked"));
    }
    if (card && card.columnId !== toColumnId) await onMove?.(cardId, toColumnId);
    if (toIndex !== undefined) await onPosition?.(cardId, toIndex);
  }

  async function handleReorder(cardId: string, toIndex: number) {
    moveError = null;
    try {
      await onPosition?.(cardId, toIndex);
    } catch (cause) {
      handleError(cardId, "", cause);
    }
  }

  async function handleFieldEdit(edit: KanbanFieldEdit) {
    moveError = null;
    try {
      await onEdit?.(edit);
    } catch (cause) {
      moveError = cause instanceof Error ? cause.message : t("kanban.editFailed");
    }
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
  bind:collapsedLanes
  bind:selected
  {fields}
  {lanes}
  selectable
  undoable
  onMove={handleMove}
  onReorder={onPosition ? handleReorder : undefined}
  onFieldEdit={onEdit ? handleFieldEdit : undefined}
  dragPreviewText={(dragged) =>
    dragged.length === 1 ? dragged[0]!.title : fill("kanban.cards", { count: dragged.length })}
  onMoveError={handleError}
  {columnFooter}
  data-testid={testid}
/>
