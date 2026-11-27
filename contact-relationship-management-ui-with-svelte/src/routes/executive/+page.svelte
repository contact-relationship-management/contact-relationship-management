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
    slaAttainment,
    staleDeals,
    type Finding,
    type StaleDeal,
  } from "#lib/api/crm.js";
  import { money } from "#lib/money.js";
  import { i18n, t, tf } from "#lib/i18n.svelte.js";
  import { formatDateRange, formatDateTime, formatPercent } from "#lib/locale-format.js";

  type Pack = Awaited<ReturnType<typeof executivePack>>;
  type Trends = Awaited<ReturnType<typeof forecastTrends>>;
  type Attainment = Awaited<ReturnType<typeof slaAttainment>>;

  let pack = $state<Pack | null>(null);
  let stale = $state<StaleDeal[]>([]);
  let staleDerivation = $state("");
  let findings = $state<Finding[]>([]);
  let trends = $state<Trends | null>(null);
  let attainment = $state<Attainment | null>(null);
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
        attainment = await slaAttainment();
      } catch (cause) {
        error = errorMessage(cause);
      }
    })();
  });

  const staleColumns = $derived([
    { id: "deal", header: t("col.deal") },
    { id: "stage", header: t("deal.stage") },
    { id: "days", kind: "number", total: "avg", header: t("col.daysInStage") },
    { id: "amount", kind: "money", header: t("common.amount"), accessor: (r: Record<string, unknown>) => r.amount_minor, format: (_v: unknown, r: Record<string, unknown>) => String(r.amount) },
  ]);
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
  const hygieneColumns = $derived([
    { id: "rule", header: t("col.rule") },
    { id: "detail", header: t("col.detail") },
  ]);
  const hygieneRows = $derived(
    findings.map((f, index) => ({ id: String(index), rule: f.rule, detail: f.detail })),
  );
  const attainmentColumns = $derived([
    { id: "ticket", header: t("col.ticket") },
    { id: "measure", header: t("col.measure") },
    { id: "due", header: t("col.dueAt") },
    { id: "outcome", kind: "chip", header: t("col.outcome") },
  ]);
  const attainmentRows = $derived(
    (attainment?.deadlines ?? []).map((d) => ({
      id: `${d.pid}-${d.measure}`,
      ticket: d.title,
      measure: t(d.measure === "first_response" ? "exec.measure.firstResponse" : "exec.measure.resolution"),
      due: formatDateTime(d.due_at),
      outcome: t(
        d.outcome === "met"
          ? "exec.outcome.met"
          : d.outcome === "missed"
            ? "exec.outcome.missed"
            : "exec.outcome.undecided",
      ),
    })),
  );
  const trendColumns = $derived([
    { id: "taken", kind: "date", header: t("col.taken") },
    { id: "totals", header: t("col.totals") },
  ]);
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
  <p class="muted" data-testid="exec-window">
    {tf("exec.window", { range: formatDateRange(pack.window.from, pack.window.to) })} · {pack.note}
  </p>
  <section class="tiles" data-testid="exec-tiles">
    <div class="tile"><strong>{pack.deals_won}</strong><span>{t("exec.won")}</span></div>
    <div class="tile"><strong>{pack.deals_lost}</strong><span>{t("exec.lost")}</span></div>
    <div class="tile"><strong>{pack.new_leads}</strong><span>{t("exec.newLeads")}</span></div>
    <div class="tile"><strong>{pack.tickets_opened}</strong><span>{t("exec.ticketsOpened")}</span></div>
    <div class="tile"><strong>{pack.tickets_resolved}</strong><span>{t("exec.resolved")}</span></div>
    <div class="tile"><strong>{pack.consent_withdrawals}</strong><span>{t("exec.consentWithdrawals")}</span></div>
  </section>
  <p data-testid="exec-won-value">
    {t("exec.wonValue")}
    {#each Object.entries(pack.won_value_by_currency_minor) as [currency, minor] (currency)}
      <strong>{money(minor, currency, i18n.locale)}</strong>&nbsp;<wbr />
    {:else}
      <span class="muted">{t("exec.noneInWindow")}</span>
    {/each}
  </p>
  {#if Object.keys(pack.lost_reasons).length > 0}
    <p data-testid="exec-lost-reasons">
      {t("exec.lostReasons")}
      {#each Object.entries(pack.lost_reasons) as [reason, count] (reason)}
        {reason} ({count})&nbsp;
      {/each}
    </p>
  {/if}
{/if}

{#if attainment}
  <h2>{t("exec.attainment")}</h2>
  <p class="muted">{attainment.derivation}</p>
  <section class="tiles" data-testid="exec-attainment">
    {#each [["exec.firstResponseMet", attainment.first_response], ["exec.resolutionMet", attainment.resolution]] as const as [label, m] (label)}
      <div class="tile">
        <strong>{m.attained.value === null ? t("exec.noneDecided") : formatPercent(m.attained.value)}</strong>
        <span>{t(label)}</span>
        <span class="muted small">
          {tf("exec.attainedOf", { met: m.met, decided: m.attained.denominator, undecided: m.undecided })}
        </span>
      </div>
    {/each}
  </section>
  <DataTable
    label={t("exec.attainment")}
    columns={attainmentColumns}
    rows={attainmentRows}
    testid="exec-deadlines"
    empty={t("exec.noDeadlines")}
  />
{/if}

{#if stale.length > 0}
  <h2>{t("exec.stale")}</h2>
  <p class="muted">{staleDerivation}</p>
  <DataTable label={t("exec.stale")} columns={staleColumns} rows={staleRows} testid="exec-stale" />
{/if}

<h2>{t("exec.hygiene")}</h2>
<DataTable
  label={t("exec.hygiene")}
  columns={hygieneColumns}
  rows={hygieneRows}
  testid="exec-hygiene"
  empty={t("exec.noFindings")}
/>

{#if trends}
  <h2>{t("exec.trend")}</h2>
  <p class="muted">{trends.note}</p>
  <DataTable
    label={t("exec.trend")}
    columns={trendColumns}
    rows={trendRows}
    testid="exec-trends"
    empty={t("exec.noSnapshots")}
  />
{/if}
