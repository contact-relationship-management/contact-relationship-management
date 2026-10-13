<!--
  DataTable — the app's one data table, a thin wrapper over the Lily
  `@lilydesignsystem/svelte-data-grid` (sort, filter, row selection, paging,
  full APG grid keyboard support). Cells are plain text: use `format` to turn a
  value into display text. Every user-facing string comes from the i18n
  catalog (`grid.*`), because Lily ships none.

  Props:
    - label     accessible name for the grid
    - columns   { id, header, format?, accessor?, sortable?, width? }[]
    - rows      row objects (keyed by column id)
    - rowId     stable id per row (default: the row's `id`, else its index)
    - onselect  when set, rows are selectable one at a time and choosing one
                calls this with its id (used to open a detail or act on it)
    - filter    show the filter box (default: on when there are 8+ rows)
    - pageSize  rows per page (default: all rows)
    - testid    `data-testid` on the grid root
    - empty     text shown instead of the grid when there are no rows
-->
<script lang="ts">
  import DataGrid from "@lilydesignsystem/svelte-data-grid";
  import type {
    DataGridColumn,
    DataGridRow,
    DataGridSortDirection,
  } from "@lilydesignsystem/svelte-data-grid";
  import { t, type StringKey } from "#lib/i18n.svelte.js";

  let {
    label,
    columns,
    rows,
    rowId = (row: DataGridRow, index: number) =>
      String((row as { id?: unknown }).id ?? index),
    onselect,
    filter,
    pageSize,
    testid,
    empty,
    class: className = "",
  }: {
    label: string;
    columns: DataGridColumn[];
    rows: DataGridRow[];
    rowId?: (row: DataGridRow, index: number) => string;
    onselect?: (id: string) => void;
    filter?: boolean;
    pageSize?: number;
    testid?: string;
    empty?: string;
    class?: string;
  } = $props();

  let selected = $state<string[]>([]);

  const fill = (key: StringKey, values: Record<string, string | number>) =>
    Object.entries(values).reduce(
      (text, [name, value]) => text.replaceAll(`{${name}}`, String(value)),
      t(key),
    );

  const showFilter = $derived(filter ?? rows.length >= 8);

  const labels = $derived({
    search: showFilter ? t("grid.search") : undefined,
    previousPage: t("grid.previous"),
    nextPage: t("grid.next"),
    pageStatus: (page: number, pages: number, count: number) =>
      fill("grid.page", { page, pages, rows: count }),
    sortAnnouncement: (header: string, direction: DataGridSortDirection) =>
      fill("grid.sorted", {
        header,
        direction: t(
          direction === "ascending"
            ? "grid.ascending"
            : direction === "descending"
              ? "grid.descending"
              : "grid.unsorted",
        ),
      }),
    filterAnnouncement: (matches: number, total: number) =>
      fill("grid.filtered", { matches, total }),
    selectionColumn: t("grid.select"),
    selectRow: (row: string) => fill("grid.selectRow", { row }),
  });

  // Sorting is on by default: an interactive table that cannot be sorted is
  // the exception, not the rule.
  const cols = $derived(
    columns.map((c) => ({ sortable: true, ...c })) as DataGridColumn[],
  );

  function chosen(ids: string[]) {
    const id = ids[0];
    if (id !== undefined) onselect?.(id);
  }
</script>

{#if rows.length === 0 && empty}
  <p class="muted" data-testid={testid}>{empty}</p>
{:else}
<DataGrid
  {label}
  columns={cols}
  {rows}
  {rowId}
  {pageSize}
  {labels}
  selectionMode={onselect ? "single" : "none"}
  bind:selected
  onSelectionChange={chosen}
  class={`app-grid ${className}`.trim()}
  data-testid={testid}
/>
{/if}
