<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import { resource } from "#lib/resource.svelte.js";
  import CampaignTimeline from "#lib/components/CampaignTimeline.svelte";
  import DataTable from "#lib/components/DataTable.svelte";
  import {
    campaignFunnel,
    campaignStatus,
    createCampaign,
    listCampaigns,
    listSegments,
    listTemplates,
    money,
    runCampaign,
  } from "#lib/api/crm.js";
  import { ratioPercent } from "#lib/format.js";
  import { i18n, t } from "#lib/i18n.svelte.js";
  import type { Campaign, Ratio, ReportRateUsed, ReportUnconverted } from "#lib/api/crm.js";

  const list = resource(listCampaigns);
  const campaigns = $derived(list.value);
  const error = $derived(list.error);
  let funnel = $state<{
    pid: string;
    leads: number;
    won_revenue_minor: number | null;
    roi: Ratio | null;
    rate_date: string | null;
    rates: ReportRateUsed[];
    unconverted: ReportUnconverted[];
  } | null>(null);
  let convert = $state(false);
  let actionError = $state<string | null>(null);

  async function run(campaign: Campaign) {
    actionError = null;
    try {
      if (campaign.status === "draft") {
        await campaignStatus(campaign.pid, "scheduled");
      }
      await runCampaign(campaign.pid);
      await list.reload();
    } catch (cause) {
      actionError = errorMessage(cause);
    }
  }

  async function showFunnel(campaign: Campaign) {
    const data = await campaignFunnel(campaign.pid, { convert });
    funnel = {
      pid: campaign.pid,
      leads: data.leads,
      won_revenue_minor: data.won_revenue_minor,
      roi: data.roi,
      rate_date: data.rate_date,
      rates: data.rates,
      unconverted: data.unconverted,
    };
  }

  function roiLabel(roi: Ratio | null): string {
    return ratioPercent(roi);
  }

  const segments = resource(listSegments);
  const templates = resource(listTemplates);
  let creating = $state(false);
  let name = $state("");
  let segmentPid = $state("");
  let templatePid = $state("");
  let subject = $state("");
  let body = $state("");

  async function create() {
    actionError = null;
    try {
      await createCampaign({
        name,
        ...(segmentPid ? { segment_pid: segmentPid } : {}),
        ...(templatePid
          ? { template_pid: templatePid }
          : { subject: subject || undefined, body: body || undefined }),
      });
      creating = false;
      name = segmentPid = templatePid = subject = body = "";
      await list.reload();
    } catch (cause) {
      actionError = errorMessage(cause);
    }
  }

  let selectedPid = $state<string | null>(null);
  const selected = $derived(
    (campaigns ?? []).find((c) => c.pid === selectedPid) ?? null,
  );

  const columns = $derived([
    { id: "name", header: t("common.name") },
    { id: "status", kind: "chip", header: t("common.status") },
    { id: "recipients", header: t("campaign.recipients") },
  ]);
  const rows = $derived(
    (campaigns ?? []).map((c) => ({
      id: c.pid,
      name: c.name,
      status: c.status,
      recipients: `${c.recipients} → ${c.delivered} → ${c.opened} → ${c.clicked}`,
    })),
  );
</script>

<h1>{t("nav.campaigns")}</h1>
<p>
  <button type="button" data-testid="campaign-new" onclick={() => (creating = !creating)}>
    {t("campaign.new")}
  </button>
  <a class="button" href="/templates">{t("tpl.title")}</a>
</p>
{#if creating}
  <form
    class="panel"
    data-testid="campaign-form"
    onsubmit={(event) => {
      event.preventDefault();
      void create();
    }}
  >
    <label>{t("common.name")} <input data-testid="campaign-name" bind:value={name} required /></label>
    <label>
      {t("campaign.segment")}
      <select data-testid="campaign-segment" bind:value={segmentPid}>
        <option value="">{t("campaign.noSegment")}</option>
        {#each segments.value ?? [] as segment (segment.pid)}
          <option value={segment.pid}>{segment.name}</option>
        {/each}
      </select>
    </label>
    <label>
      {t("campaign.template")}
      <select data-testid="campaign-template" bind:value={templatePid}>
        <option value="">{t("campaign.ownMessage")}</option>
        {#each templates.value ?? [] as template (template.pid)}
          <option value={template.pid}>{template.name}</option>
        {/each}
      </select>
    </label>
    {#if !templatePid}
      <label>{t("tpl.subject")} <input data-testid="campaign-subject" bind:value={subject} /></label>
      <label>
        {t("tpl.body")}
        <textarea data-testid="campaign-body" rows="5" bind:value={body}></textarea>
      </label>
    {/if}
    <button type="submit" data-testid="campaign-create">{t("campaign.create")}</button>
  </form>
{/if}

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if campaigns === null}
  <p>{t("common.loading")}</p>
{:else}
  {#if actionError}
    <p class="error" data-testid="action-error">{actionError}</p>
  {/if}
  <DataTable
    label={t("nav.campaigns")}
    {columns}
    {rows}
    onselect={(pid) => {
      selectedPid = pid;
      funnel = null;
    }}
    testid="campaigns"
  />
  {#if selected}
    <div class="panel" data-testid="campaign-actions">
      <strong>{selected.name}</strong>
      {#if selected.status === "draft" || selected.status === "scheduled"}
        <button onclick={() => void run(selected)}>{t("campaign.run")}</button>
      {/if}
      <button onclick={() => void showFunnel(selected)}>{t("campaign.funnel")}</button>
      <label>
        <input
          type="checkbox"
          bind:checked={convert}
          onchange={() => void showFunnel(selected)}
          data-testid="funnel-convert"
        />
        {t("campaign.convert")}
      </label>
      {#if funnel?.pid === selected.pid}
        <p data-testid="funnel">
          {t("nav.leads")}: {funnel.leads} ·
          {money(funnel.won_revenue_minor, selected.currency, i18n.locale)} ·
          {t("campaign.roi")}: <strong>{roiLabel(funnel.roi)}</strong>
          {#if funnel.roi}
            <span class="muted">
              ({funnel.roi.numerator} / {funnel.roi.denominator})
            </span>
          {/if}
        </p>
        {#if funnel.rates.length > 0}
          <p class="muted small" data-testid="funnel-rates">
            {t("rpt.ratesUsed")} ({funnel.rate_date}):
            {#each funnel.rates as used (used.from)}
              <span class="chip">
                1 {used.from} = {used.rate}
                {used.to}, {used.effective_on}, {used.source}
                {#if used.inverted}({t("rpt.inverted")}){/if}
              </span>
            {/each}
          </p>
        {/if}
        {#if funnel.unconverted.length > 0}
          <p data-testid="funnel-unconverted">
            {t("rpt.unconverted")}:
            {#each funnel.unconverted as left (left.currency)}
              <span class="chip">
                {left.currency}: {money(left.amount_minor, left.currency, i18n.locale)}
              </span>
            {/each}
            <span class="muted small">{t("rpt.unconvertedHint")}</span>
          </p>
        {/if}
      {/if}
    </div>
  {/if}
  <CampaignTimeline campaigns={campaigns ?? []} />
{/if}
