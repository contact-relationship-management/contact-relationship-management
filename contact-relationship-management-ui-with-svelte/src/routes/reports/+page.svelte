<!--
  Reports (`/reports`): count records, or add up deal amounts, grouped by one field (CRM-R48).
  Everything is chosen from the service's closed lists and computed there; a sum is always shown
  per currency. A report can be saved, and shared like a saved view.
-->
<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import { resource } from "#lib/resource.svelte.js";
  import {
    deleteReport,
    listReports,
    runReport,
    runSavedReport,
    saveReport,
    type ReportResult,
    type ReportSpec,
  } from "#lib/api/crm.js";
  import { i18n, t } from "#lib/i18n.svelte.js";

  // What the service lets each kind be grouped and filtered by.
  const FIELDS: Record<ReportSpec["kind"], string[]> = {
    contact: ["status", "marketing_consent", "account_tier", "owner", "created_month", "tag"],
    account: ["tier", "industry", "owner", "created_month", "tag"],
    lead: ["source", "status", "owner", "created_month", "tag"],
    deal: ["stage", "state", "currency", "pipeline", "owner", "created_month", "closed_month", "tag"],
    ticket: ["status", "priority", "channel", "assignee", "opened_month"],
  };
  const KIND_LABEL = {
    contact: "nav.contacts",
    account: "nav.accounts",
    lead: "nav.leads",
    deal: "nav.deals",
    ticket: "nav.tickets",
  } as const;
  const field = (name: string) => t(`rpt.field.${name}` as never);

  let kind = $state<ReportSpec["kind"]>("lead");
  let groupBy = $state("source");
  let measure = $state<ReportSpec["measure"]>("count");
  let filterField = $state("");
  let filterValues = $state("");
  let from = $state("");
  let to = $state("");
  let name = $state("");
  let shared = $state(false);
  let result = $state<ReportResult | null>(null);
  let error = $state<string | null>(null);

  const saved = resource(listReports);
  const reports = $derived(saved.value ?? []);

  // A kind change picks a grouping that kind has, and drops what it cannot do.
  $effect(() => {
    if (!FIELDS[kind].includes(groupBy)) groupBy = FIELDS[kind][0]!;
    if (kind !== "deal") measure = "count";
    if (filterField && !FIELDS[kind].includes(filterField)) filterField = "";
  });

  function spec(): ReportSpec {
    const values = filterValues
      .split(",")
      .map((v) => v.trim())
      .filter(Boolean);
    return {
      kind,
      group_by: groupBy,
      measure,
      filter: filterField && values.length ? { [filterField]: values } : {},
      from: from || null,
      to: to || null,
    };
  }

  function load(s: ReportSpec) {
    kind = s.kind;
    groupBy = s.group_by;
    measure = s.measure;
    const [first] = Object.entries(s.filter);
    filterField = first?.[0] ?? "";
    filterValues = first?.[1].join(", ") ?? "";
    from = s.from ?? "";
    to = s.to ?? "";
  }

  async function run() {
    error = null;
    try {
      result = await runReport(spec());
    } catch (cause) {
      result = null;
      error = errorMessage(cause);
    }
  }

  async function open(pid: string, s: ReportSpec) {
    error = null;
    load(s);
    try {
      result = await runSavedReport(pid);
    } catch (cause) {
      result = null;
      error = errorMessage(cause);
    }
  }

  async function save() {
    error = null;
    try {
      await saveReport({ name, spec: spec(), shared });
      name = "";
      await saved.reload();
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  async function remove(pid: string) {
    error = null;
    try {
      await deleteReport(pid);
      await saved.reload();
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  // The bars are scaled within one currency, so one never dwarfs another.
  const widest = $derived((currency: string | null) =>
    Math.max(1, ...(result?.rows ?? []).filter((r) => r.currency === currency).map((r) => r.value)),
  );
  const amount = (minor: number, currency: string | null) =>
    currency
      ? new Intl.NumberFormat(i18n.locale, { style: "currency", currency }).format(minor / 100)
      : String(minor);
</script>

<h1>{t("rpt.title")}</h1>
<p class="muted">{t("rpt.hint")}</p>

<form
  class="panel"
  data-testid="report-form"
  onsubmit={(e) => {
    e.preventDefault();
    void run();
  }}
>
  <label>
    {t("rpt.kind")}
    <select bind:value={kind} data-testid="report-kind">
      {#each Object.entries(KIND_LABEL) as [value, key] (value)}
        <option {value}>{t(key)}</option>
      {/each}
    </select>
  </label>
  <label>
    {t("rpt.groupBy")}
    <select bind:value={groupBy} data-testid="report-group">
      {#each FIELDS[kind] as option (option)}<option value={option}>{field(option)}</option>{/each}
    </select>
  </label>
  {#if kind === "deal"}
    <label>
      {t("rpt.measure")}
      <select bind:value={measure} data-testid="report-measure">
        <option value="count">{t("rpt.measure.count")}</option>
        <option value="sum_amount">{t("rpt.measure.sum_amount")}</option>
      </select>
    </label>
  {/if}
  <label>
    {t("rpt.filterField")}
    <select bind:value={filterField} data-testid="report-filter-field">
      <option value=""></option>
      {#each FIELDS[kind].filter((f) => !f.endsWith("_month")) as option (option)}
        <option value={option}>{field(option)}</option>
      {/each}
    </select>
  </label>
  {#if filterField}
    <label>
      {t("rpt.filterValues")}
      <input bind:value={filterValues} data-testid="report-filter-values" />
    </label>
  {/if}
  <label>{t("rpt.from")} <input type="date" bind:value={from} /></label>
  <label>{t("rpt.to")} <input type="date" bind:value={to} /></label>
  <button type="submit" data-testid="report-run">{t("rpt.run")}</button>
</form>

{#if error}<p class="error" role="alert" data-testid="report-error">{error}</p>{/if}

{#if result}
  <div class="panel" data-testid="report-result">
    <p class="muted">
      {t("rpt.asOf")}: {new Date(result.as_of).toLocaleString(i18n.locale)}
    </p>
    {#if result.rows.length === 0}
      <p data-testid="report-none">{t("rpt.none")}</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th>{t("rpt.group")}</th>
            {#if result.spec.measure === "sum_amount"}<th>{t("rpt.currency")}</th>{/if}
            <th>{t("rpt.value")}</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each result.rows as row, i (i)}
            <tr data-testid="report-row">
              <td>{row.group}</td>
              {#if result.spec.measure === "sum_amount"}<td>{row.currency}</td>{/if}
              <td>
                {result.spec.measure === "sum_amount" ? amount(row.value, row.currency) : row.value}
              </td>
              <td aria-hidden="true">
                <span class="bar" style:width={`${(row.value / widest(row.currency)) * 8}rem`}></span>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
    {#if result.truncated}<p class="muted">{t("rpt.truncated")}</p>{/if}
    <p class="muted small">{result.note}</p>
  </div>

  <form
    class="panel"
    data-testid="report-save-form"
    onsubmit={(e) => {
      e.preventDefault();
      void save();
    }}
  >
    <label>{t("rpt.name")} <input bind:value={name} required data-testid="report-name" /></label>
    <label><input type="checkbox" bind:checked={shared} /> {t("rpt.shared")}</label>
    <button type="submit" data-testid="report-save">{t("rpt.save")}</button>
  </form>
{/if}

<h2>{t("rpt.saved")}</h2>
{#if reports.length === 0}
  <p class="muted">{t("rpt.noSaved")}</p>
{:else}
  <ul data-testid="report-list">
    {#each reports as report (report.pid)}
      <li>
        <button type="button" onclick={() => void open(report.pid, report.spec)}>{report.name}</button>
        {#if report.shared}<span class="chip">{t("rpt.shared")}</span>{/if}
        {#if report.mine}
          <button type="button" onclick={() => void remove(report.pid)}>{t("rpt.delete")}</button>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  .bar {
    display: inline-block;
    height: 0.75rem;
    background: var(--accent);
    border-radius: 2px;
    vertical-align: middle;
  }
</style>
