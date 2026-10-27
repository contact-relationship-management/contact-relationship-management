<!--
  An account's customer lifetime value (CRM-R13, CRM-R60): what has been won per currency, the
  margin on it, and a discounted look-ahead from a retention the reader supplies. Every figure
  comes from the service; amounts of different currencies are never added.
-->
<script lang="ts">
  import { accountClv, money, type Clv } from "#lib/api/crm.js";
  import { errorMessage } from "#lib/errors.js";
  import { i18n, t } from "#lib/i18n.svelte.js";

  let { pid }: { pid: string } = $props();

  let margin = $state("");
  let retention = $state("");
  let years = $state("3");
  let discount = $state("");
  let result = $state<Clv | null>(null);
  let error = $state<string | null>(null);

  /** A percentage typed as `12.5` is 1250 basis points. */
  const bps = (percent: string) => Math.round(Number(percent) * 100);

  async function run() {
    error = null;
    try {
      result = await accountClv(pid, {
        marginBps: margin.trim() ? bps(margin) : undefined,
        retentionBps: retention.trim() ? bps(retention) : undefined,
        years: retention.trim() ? Number(years) : undefined,
        discountBps: discount.trim() ? bps(discount) : undefined,
      });
    } catch (cause) {
      error = errorMessage(cause);
    }
  }
  $effect(() => {
    void run();
  });
</script>

{#snippet lines(label: string, amounts: Record<string, number>, testid: string)}
  <p>
    {label}:
    {#each Object.entries(amounts) as [currency, minor] (currency)}
      <span class="chip" data-testid={testid}>{money(minor, currency, i18n.locale)}</span>
    {/each}
  </p>
{/snippet}

<section class="panel" data-testid="clv-panel">
  <h2>{t("clv.title")}</h2>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      void run();
    }}
  >
    <label>{t("clv.margin")} <input bind:value={margin} inputmode="decimal" size="5" data-testid="clv-margin" /></label>
    <label>{t("clv.retention")} <input bind:value={retention} inputmode="decimal" size="5" data-testid="clv-retention" /></label>
    <label>{t("clv.years")} <input bind:value={years} inputmode="numeric" size="3" data-testid="clv-years" /></label>
    <label>{t("clv.discount")} <input bind:value={discount} inputmode="decimal" size="5" data-testid="clv-discount" /></label>
    <button type="submit" data-testid="clv-run">{t("clv.run")}</button>
  </form>
  {#if error}
    <p class="error" data-testid="clv-error">{error}</p>
  {:else if result}
    {#if result.clv_minor === null}
      <p class="muted" data-testid="clv-masked">{t("clv.masked")}</p>
    {:else}
      {@render lines(t("clv.won"), result.clv_minor, "clv-won")}
      {#if result.margin_minor}{@render lines(t("clv.marginResult"), result.margin_minor, "clv-margin-result")}{/if}
      {#if result.projection}
        {@render lines(t("clv.base"), result.projection.annual_base_minor, "clv-base")}
        {@render lines(t("clv.projected"), result.projection.projected_minor, "clv-projected")}
        {@render lines(t("clv.total"), result.projection.total_minor, "clv-total")}
      {/if}
    {/if}
    <p class="muted small">{new Date(result.as_of).toLocaleString(i18n.locale)}</p>
  {/if}
</section>
