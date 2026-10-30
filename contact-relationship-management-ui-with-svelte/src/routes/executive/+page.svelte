<!--
  Sales executive area (`/executive`): the period pack (deals won /
  lost with per-currency won value, lost reasons verbatim, leads,
  tickets, activities, consent withdrawals), stale-deal aging,
  pipeline-hygiene findings, and the stored forecast-trend series.
  All numbers server-derived; currencies never merged.
-->
<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import DataTable from "#lib/components/DataTable.svelte";
  import {
    executivePack,
    forecastTrends,
    pipelineHygiene,
    staleDeals,
    type Finding,
    type StaleDeal,
  } from "#lib/api/crm.js";
  import { money } from "#lib/money.js";
  import { i18n, t } from "#lib/i18n.svelte.js";

  type Pack = Awaited<ReturnType<typeof executivePack>>;
  type Trends = Awaited<ReturnType<typeof forecastTrends>>;

  let pack = $state<Pack | null>(null);
  let stale = $state<StaleDeal[]>([]);
  let staleDerivation = $state("");
  let findings = $state<Finding[]>([]);
  let trends = $state<Trends | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        pack = await executivePack();
        const aging = await staleDeals();
        stale = aging.deals.filter((deal) => deal.stale);
        staleDerivation = aging.derivation;
        findings = (await pipelineHygiene()).findings;
        trends = await forecastTrends();
      } catch (cause) {
        error = errorMessage(cause);
      }
    })();
  });

  const staleColumns = [
    { id: "deal", header: "Deal" },
    { id: "stage", header: "Stage" },
    { id: "days", kind: "number", total: "avg", header: "Days in stage" },
    { id: "amount", kind: "money", header: "Amount", accessor: (r: Record<string, unknown>) => r.amount_minor, format: (_v: unknown, r: Record<string, unknown>) => String(r.amount) },
  ];
  const staleRows = $derived(
    stale.map((d) => ({
      id: d.pid,
      deal: d.name,
      stage: d.stage ?? "—",
      days: d.days_in_stage,
      amount_minor: d.amount_minor,
      amount: money(d.amount_minor, d.currency, i18n.locale),
    })),
  );
  const hygieneColumns = [
    { id: "rule", header: "Rule" },
    { id: "detail", header: "Detail" },
  ];
  const hygieneRows = $derived(
    findings.map((f, index) => ({ id: String(index), rule: f.rule, detail: f.detail })),
  );
  const trendColumns = [
    { id: "taken", kind: "date", header: "Taken" },
    { id: "totals", header: "Totals" },
  ];
  const trendRows = $derived(
    (trends?.series ?? []).map((snap) => ({
      id: snap.taken_on,
      taken: snap.taken_on,
      totals: Object.entries(snap.totals)
        .map(([currency, minor]) => money(minor, currency, i18n.locale))
        .join(" "),
    })),
  );
</script>

<svelte:head><title>{t("nav.executive")} — CRM</title></svelte:head>

<h1>{t("nav.executive")}</h1>
{#if error}<p class="error" data-testid="error">{error}</p>{/if}

{#if pack}
  <p class="muted">window {pack.window.from} → {pack.window.to} · {pack.note}</p>
  <section class="tiles" data-testid="exec-tiles">
    <div class="tile"><strong>{pack.deals_won}</strong><span>won</span></div>
    <div class="tile"><strong>{pack.deals_lost}</strong><span>lost</span></div>
    <div class="tile"><strong>{pack.new_leads}</strong><span>new leads</span></div>
    <div class="tile"><strong>{pack.tickets_opened}</strong><span>tickets opened</span></div>
    <div class="tile"><strong>{pack.tickets_resolved}</strong><span>resolved</span></div>
    <div class="tile"><strong>{pack.consent_withdrawals}</strong><span>consent withdrawals</span></div>
  </section>
  <p data-testid="exec-won-value">
    Won value:
    {#each Object.entries(pack.won_value_by_currency_minor) as [currency, minor] (currency)}
      <strong>{money(minor, currency, i18n.locale)}</strong>&nbsp;
    {:else}
      <span class="muted">none in window</span>
    {/each}
  </p>
  {#if Object.keys(pack.lost_reasons).length > 0}
    <p data-testid="exec-lost-reasons">
      Lost reasons:
      {#each Object.entries(pack.lost_reasons) as [reason, count] (reason)}
        {reason} ({count})&nbsp;
      {/each}
    </p>
  {/if}
{/if}

{#if stale.length > 0}
  <h2>Stale deals</h2>
  <p class="muted">{staleDerivation}</p>
  <DataTable label="Stale deals" columns={staleColumns} rows={staleRows} testid="exec-stale" />
{/if}

<h2>Pipeline hygiene</h2>
<DataTable
  label="Pipeline hygiene"
  columns={hygieneColumns}
  rows={hygieneRows}
  testid="exec-hygiene"
  empty="No findings."
/>

{#if trends}
  <h2>Forecast trend</h2>
  <p class="muted">{trends.note}</p>
  <DataTable
    label="Forecast trend"
    columns={trendColumns}
    rows={trendRows}
    testid="exec-trends"
    empty="No snapshots captured yet."
  />
{/if}
