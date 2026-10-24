<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import { resource } from "#lib/resource.svelte.js";
  import CampaignTimeline from "#lib/components/CampaignTimeline.svelte";
  import DataTable from "#lib/components/DataTable.svelte";
  import { campaignFunnel, campaignStatus, listCampaigns, money, runCampaign } from "#lib/api/crm.js";
  import { ratioPercent } from "#lib/format.js";
  import { i18n, t } from "#lib/i18n.svelte.js";
  import type { Campaign, Ratio } from "#lib/api/crm.js";

  const list = resource(listCampaigns);
  const campaigns = $derived(list.value);
  const error = $derived(list.error);
  let funnel = $state<{
    pid: string;
    leads: number;
    won_revenue_minor: number | null;
    roi: Ratio | null;
  } | null>(null);
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
    const data = await campaignFunnel(campaign.pid);
    funnel = {
      pid: campaign.pid,
      leads: data.leads,
      won_revenue_minor: data.won_revenue_minor,
      roi: data.roi,
    };
  }

  function roiLabel(roi: Ratio | null): string {
    return ratioPercent(roi);
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
      {/if}
    </div>
  {/if}
  <CampaignTimeline campaigns={campaigns ?? []} />
{/if}
