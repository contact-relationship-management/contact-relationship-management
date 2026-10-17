<!--
  DataTable — the app's one data table, a thin wrapper over the Lily
  `@lilydesignsystem/svelte-data-grid` (sort, filter, paging, APG grid keyboard
  support). Every user-facing string comes from the i18n catalog (`grid.*`),
  because Lily ships none.

  Cells are text unless a column says otherwise:
    - kind "chip"   the value as a chip (status, priority, consent)
    - kind "flag"   the value in bold alert colour (a breach)
    - kind "number" right-aligned (counts, scores, days)
    - kind "money"  right-aligned (pass `format` to show the amount)
    - href(row)     the value as a link
  The first column is the row header. Text sorts with the UI language's
  collation (numbers numerically, "10" after "9").

  Props:
    - label     accessible name for the grid
    - columns   { id, header, format?, accessor?, kind?, href?, width?, … }[]
    - rows      row objects (keyed by column id)
    - rowId     stable id per row (default: the row's `id`, else its index)
    - onselect  when set, clicking a row (or Enter on it) calls this with the
                row's id, and the row is marked selected
    - filter    show the filter box (default: on when there are 8+ rows)
    - pageSize  rows per page (default: all rows, or 25 when there are more)
    - testid    `data-testid` on the grid root; also keys the remembered
                sort and column widths
    - (export)  CSV and Excel buttons, named after `testid`
    - empty     text shown in the grid when there are no rows
-->
<script lang="ts">
  import DataGrid from "@lilydesignsystem/svelte-data-grid";
  import type {
    DataGridCellContext,
    DataGridColumn,
    DataGridRow,
    DataGridSortDirection,
  } from "@lilydesignsystem/svelte-data-grid";
  import { i18n, t, type StringKey } from "#lib/i18n.svelte.js";

  // `kind` is one of "chip" | "flag" | "number" | "money" (a plain string so
  // pages can declare their columns without widening trouble).
  type Column = DataGridColumn & {
    kind?: string;
    href?: (row: DataGridRow) => string;
  };

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
    columns: Column[];
    rows: DataGridRow[];
    rowId?: (row: DataGridRow, index: number) => string;
    onselect?: (id: string) => void;
    filter?: boolean;
    pageSize?: number;
    testid?: string;
    empty?: string;
    class?: string;
  } = $props();

  let activeId = $state<string | null>(null);

  const fill = (key: StringKey, values: Record<string, string | number>) =>
    Object.entries(values).reduce(
      (text, [name, value]) => text.replaceAll(`{${name}}`, String(value)),
      t(key),
    );

  const showFilter = $derived(filter ?? rows.length >= 8);
  const size = $derived(pageSize ?? (rows.length > 25 ? 25 : undefined));

  const labels = $derived({
    search: showFilter ? t("grid.search") : undefined,
    export: t("grid.export"),
    exportCsv: "CSV",
    exportExcel: "Excel",
    exportAnnouncement: (_format: unknown, count: number, filename: string) =>
      fill("grid.exported", { count, filename }),
    previousPage: t("grid.previous"),
    nextPage: t("grid.next"),
    pageStatus: (page: number, pages: number, count: number) =>
      fill("grid.page", { page, pages, rows: count }),
    pageAnnouncement: (page: number, pages: number) =>
      fill("grid.pageOf", { page, pages }),
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
  });

  // Numbers sort numerically; text sorts with the UI language's collation, with
  // digit runs compared as numbers.
  const collator = $derived(
    new Intl.Collator(i18n.locale, { numeric: true, sensitivity: "base" }),
  );
  const compare = (a: unknown, b: unknown) =>
    typeof a === "number" && typeof b === "number"
      ? a - b
      : collator.compare(String(a), String(b));

  const cols = $derived(
    columns.map((c, index) => ({
      sortable: true,
      compare,
      rowHeader: index === 0,
      align: c.kind === "number" || c.kind === "money" ? "end" : undefined,
      ...c,
      cell: c.kind === "chip" || c.kind === "flag" || c.href ? richCell : undefined,
    })) as DataGridColumn[],
  );

  function activate(_row: DataGridRow, id: string) {
    activeId = id;
    onselect?.(id);
  }
</script>

{#snippet richCell(ctx: DataGridCellContext)}
  {@const column = ctx.column as Column}
  {#if column.href}
    <a href={column.href(ctx.row)}>{ctx.formatted}</a>
  {:else if column.kind === "chip" && ctx.formatted}
    <span class="chip">{ctx.formatted}</span>
  {:else if column.kind === "flag" && ctx.formatted}
    <strong class="flag">{ctx.formatted}</strong>
  {:else}
    {ctx.formatted}
  {/if}
{/snippet}

{#snippet emptyRow()}{empty}{/snippet}

<DataGrid
  {label}
  columns={cols}
  {rows}
  {rowId}
  pageSize={size}
  {labels}
  exportFilename={testid}
  storageKey={testid ? `crm-grid-${testid}` : undefined}
  empty={empty ? emptyRow : undefined}
  onRowActivate={onselect ? activate : undefined}
  rowClass={(row) => (onselect && rowId(row, 0) === activeId ? "is-selected" : "")}
  rowAttrs={() => (onselect ? { "data-activatable": "" } : {})}
  class={`app-grid ${className}`.trim()}
  data-testid={testid}
/>
