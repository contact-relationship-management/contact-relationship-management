<!--
  Exchange rates (`/fx-rates`): the rates a report uses to show deal sums in one currency
  (CRM-R50). They are entered by hand, one per pair and day, each with its source; nothing is
  fetched. Setting a pair and a day that exist replaces the rate.
-->
<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import { resource } from "#lib/resource.svelte.js";
  import { deleteFxRate, listFxRates, setFxRate } from "#lib/api/crm.js";
  import { t } from "#lib/i18n.svelte.js";

  let from = $state("USD");
  let to = $state("GBP");
  let rate = $state("");
  let effectiveOn = $state(new Date().toISOString().slice(0, 10));
  let source = $state("");
  let error = $state<string | null>(null);

  const rates = resource(listFxRates);
  const list = $derived(rates.value ?? []);

  async function save() {
    error = null;
    try {
      await setFxRate({
        from: from.trim().toUpperCase(),
        to: to.trim().toUpperCase(),
        rate: rate.trim(),
        effective_on: effectiveOn,
        source,
      });
      rate = "";
      await rates.reload();
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  async function remove(pid: string) {
    error = null;
    try {
      await deleteFxRate(pid);
      await rates.reload();
    } catch (cause) {
      error = errorMessage(cause);
    }
  }
</script>

<h1>{t("fx.title")}</h1>
<p class="muted">{t("fx.hint")}</p>

<form
  class="panel"
  data-testid="fx-form"
  onsubmit={(e) => {
    e.preventDefault();
    void save();
  }}
>
  <label>
    {t("fx.from")}
    <input bind:value={from} required maxlength="3" size="4" data-testid="fx-from" />
  </label>
  <label>
    {t("fx.to")}
    <input bind:value={to} required maxlength="3" size="4" data-testid="fx-to" />
  </label>
  <label>
    {t("fx.rate")}
    <input bind:value={rate} required inputmode="decimal" data-testid="fx-rate" />
  </label>
  <label>
    {t("fx.effectiveOn")}
    <input type="date" bind:value={effectiveOn} required data-testid="fx-date" />
  </label>
  <label>
    {t("fx.source")}
    <input bind:value={source} required maxlength="80" data-testid="fx-source" />
  </label>
  <button type="submit" data-testid="fx-set">{t("fx.set")}</button>
</form>

{#if error}<p class="error" role="alert" data-testid="fx-error">{error}</p>{/if}

{#if list.length === 0}
  <p class="muted" data-testid="fx-none">{t("fx.none")}</p>
{:else}
  <table data-testid="fx-list">
    <thead>
      <tr>
        <th>{t("fx.from")}</th>
        <th>{t("fx.to")}</th>
        <th>{t("fx.rate")}</th>
        <th>{t("fx.effectiveOn")}</th>
        <th>{t("fx.source")}</th>
        <th>{t("fx.setBy")}</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#each list as row (row.pid)}
        <tr data-testid="fx-row">
          <td>{row.from}</td>
          <td>{row.to}</td>
          <td>{row.rate}</td>
          <td>{row.effective_on}</td>
          <td>{row.source}</td>
          <td>{row.set_by}</td>
          <td>
            <button type="button" onclick={() => void remove(row.pid)}>{t("fx.delete")}</button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
