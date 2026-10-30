<!--
  DataTable — the app's one data table, a thin wrapper over the Lily
  `@lilydesignsystem/svelte-data-grid` (sort, filter, paging, APG grid keyboard
  support). Every user-facing string comes from the i18n catalog (`grid.*`),
  because Lily ships none.

  Cells are text unless a column says otherwise:
    - kind "chip"   the value as a chip (status, priority, consent)
    - kind "flag"   the value in bold alert colour (a breach)
    - kind "number" a typed number: right-aligned, formatted for the UI
                    language, sorted numerically (an empty value shows blank)
    - kind "date"   a typed date (an ISO day), shown in the UI language
    - kind "money"  right-aligned (pass `format` to show the amount)
    - total         "sum" or "avg": a footer cell totals the rows shown
    - href(row)     the value as a link
  The first column is the row header and stays in place (pinned) when the
  table scrolls sideways. Columns can be dragged, or moved from the keyboard,
  into a new order, which is remembered per table.

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
  import { packLabels } from "#lib/lily-labels.js";
  import DataGrid from "@lilydesignsystem/svelte-data-grid";
  import type {
    DataGridCellContext,
    DataGridColumn,
    DataGridRow,
    DataGridSortDirection,
  } from "@lilydesignsystem/svelte-data-grid";
  import { i18n, t, type StringKey } from "#lib/i18n.svelte.js";
  import { formatNumber } from "#lib/locale-format.js";

  // `kind` is one of "chip" | "flag" | "number" | "money" (a plain string so
  // pages can declare their columns without widening trouble).
  type Column = DataGridColumn & {
    kind?: string;
    href?: (row: DataGridRow) => string;
    total?: string;
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

  // The column order is remembered per table (never the rows).
  const orderKey = $derived(testid ? `crm-grid-order-${testid}` : null);
  let columnOrder = $state<string[]>([]);
  $effect(() => {
    if (!orderKey) return;
    try {
      const saved = JSON.parse(localStorage.getItem(orderKey) ?? "[]");
      if (Array.isArray(saved)) columnOrder = saved.map(String);
    } catch {
      // Storage can be unavailable; the table still works.
    }
  });
  function rememberOrder(order: string[]) {
    columnOrder = order;
    if (!orderKey) return;
    try {
      localStorage.setItem(orderKey, JSON.stringify(order));
    } catch {
      // Not remembered; nothing else depends on it.
    }
  }

  const fill = (key: StringKey, values: Record<string, string | number>) =>
    Object.entries(values).reduce(
      (text, [name, value]) => text.replaceAll(`{${name}}`, String(value)),
      t(key),
    );

  const showFilter = $derived(filter ?? rows.length >= 8);
  const size = $derived(pageSize ?? (rows.length > 25 ? 25 : undefined));

  // The pack supplies the generic wording; what follows is the app's own and wins.
  const labels = $derived({
    ...packLabels("data-grid", i18n.locale),
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
    columnMoveAnnouncement: (header: string, position: number, count: number) =>
      fill("grid.columnMoved", { header, position, count }),
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

  // A typed number reads its value as a number; an empty one is blank, not 0.
  const numeric = (c: Column) => (row: DataGridRow) => {
    const raw = c.accessor ? c.accessor(row) : row[c.id];
    if (raw === "" || raw === null || raw === undefined) return null;
    const n = Number(raw);
    return Number.isNaN(n) ? raw : n;
  };

  const cols = $derived(
    columns.map((c, index) => ({
      sortable: true,
      compare,
      rowHeader: index === 0,
      movable: true,
      pinned: index === 0 ? "start" : undefined,
      align: c.kind === "number" || c.kind === "money" ? "end" : undefined,
      ...(c.kind === "number" && !c.format
        ? { type: "number", accessor: numeric(c), compare: undefined }
        : {}),
      ...(c.kind === "date"
        ? { type: "date", dateFormat: { dateStyle: "medium", timeZone: "UTC" } }
        : {}),
      ...c,
      cell: c.kind === "chip" || c.kind === "flag" || c.href ? richCell : undefined,
    })) as DataGridColumn[],
  );

  const hasTotals = $derived(columns.some((c) => c.total));
  function totalOf(c: Column, shown: DataGridRow[]): string {
    const values = shown
      .map((row) => (c.accessor ? c.accessor(row) : row[c.id]))
      .filter((v) => v !== "" && v !== null && v !== undefined)
      .map(Number)
      .filter((n) => !Number.isNaN(n));
    if (values.length === 0) return "";
    const sum = values.reduce((a, b) => a + b, 0);
    const value = c.total === "avg" ? sum / values.length : sum;
    return `${t(c.total === "avg" ? "grid.average" : "grid.total")}: ${formatNumber(value, { maximumFractionDigits: 1 })}`;
  }

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

{#snippet totalsRow(ctx: { columns: DataGridColumn[]; rows: DataGridRow[] })}
  {#if hasTotals}
    <tr class="grid-totals">
      {#each ctx.columns as column, index (column.id)}
        {@const source = columns.find((c) => c.id === column.id)}
        {#if index === 0}
          <th scope="row">{t("grid.shown")}</th>
        {:else}
          <td data-align={source?.kind === "number" ? "end" : undefined}
            >{source?.total && source ? totalOf(source, ctx.rows) : ""}</td
          >
        {/if}
      {/each}
    </tr>
  {/if}
{/snippet}

<DataGrid
  {label}
  columns={cols}
  {rows}
  {rowId}
  pageSize={size}
  {labels}
  exportFilename={testid}
  locale={i18n.locale}
  {columnOrder}
  onColumnOrderChange={rememberOrder}
  footer={hasTotals ? totalsRow : undefined}
  storageKey={testid ? `crm-grid-${testid}` : undefined}
  empty={empty ? emptyRow : undefined}
  onRowActivate={onselect ? activate : undefined}
  rowClass={(row) => (onselect && rowId(row, 0) === activeId ? "is-selected" : "")}
  rowAttrs={() => (onselect ? { "data-activatable": "" } : {})}
  class={`app-grid ${className}`.trim()}
  data-testid={testid}
/>
