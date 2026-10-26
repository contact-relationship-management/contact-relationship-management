<!--
  Attribution (`/attribution`): how the revenue of won deals is shared among the campaigns that
  touched the buyer (CRM-R49). Pick a model, a window and the days the deals closed on; each
  campaign's credit is shown per currency, and the deals no campaign touched are shown apart.
-->
<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import { attribution, type Attribution, type AttributionAmount } from "#lib/api/crm.js";
  import { i18n, t } from "#lib/i18n.svelte.js";

  let model = $state<"first" | "last" | "linear">("linear");
  let windowDays = $state(90);
  let from = $state("");
  let to = $state("");
  let convertTo = $state("");
  let rateDate = $state("");
  let result = $state<Attribution | null>(null);
  let error = $state<string | null>(null);

  async function run() {
    error = null;
    try {
      result = await attribution({
        model,
        windowDays,
        from,
        to,
        convertTo: convertTo.trim().toUpperCase(),
        rateDate,
      });
    } catch (cause) {
      result = null;
      error = errorMessage(cause);
    }
  }

  const money = (a: AttributionAmount) =>
    new Intl.NumberFormat(i18n.locale, { style: "currency", currency: a.currency }).format(
      a.amount_minor / 100,
    );
  // A bar is scaled within one currency, so one never dwarfs another.
  const widest = $derived((currency: string) =>
    Math.max(
      1,
      ...(result?.campaigns ?? []).flatMap((c) =>
        c.credit.filter((a) => a.currency === currency).map((a) => a.amount_minor),
      ),
    ),
  );
</script>

<h1>{t("attr.title")}</h1>
<p class="muted">{t("attr.hint")}</p>

<form
  class="panel"
  data-testid="attribution-form"
  onsubmit={(e) => {
    e.preventDefault();
    void run();
  }}
>
  <label>
    {t("attr.model")}
    <select bind:value={model} data-testid="attribution-model">
      <option value="first">{t("attr.model.first")}</option>
      <option value="last">{t("attr.model.last")}</option>
      <option value="linear">{t("attr.model.linear")}</option>
    </select>
  </label>
  <label>
    {t("attr.window")}
    <input
      type="number"
      min="1"
      max="365"
      bind:value={windowDays}
      data-testid="attribution-window"
    />
  </label>
  <label>{t("attr.from")} <input type="date" bind:value={from} /></label>
  <label>{t("attr.to")} <input type="date" bind:value={to} /></label>
  <label>
    {t("rpt.convertTo")}
    <input bind:value={convertTo} maxlength="3" size="4" data-testid="attribution-convert" />
  </label>
  {#if convertTo}
    <label>
      {t("rpt.rateDate")}
      <input type="date" bind:value={rateDate} data-testid="attribution-rate-date" />
    </label>
  {/if}
  <button type="submit" data-testid="attribution-run">{t("attr.run")}</button>
</form>

{#if error}<p class="error" role="alert" data-testid="attribution-error">{error}</p>{/if}

{#if result}
  <div class="panel" data-testid="attribution-result">
    <p class="muted">
      {t("attr.asOf")}: {new Date(result.as_of).toLocaleString(i18n.locale)} ·
      {t("attr.deals")}: {result.deals}
    </p>
    {#if result.campaigns.length === 0}
      <p data-testid="attribution-none">{t("attr.none")}</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th>{t("attr.campaign")}</th>
            <th>{t("attr.touches")}</th>
            <th>{t("attr.dealsTouched")}</th>
            <th>{t("attr.credit")}</th>
          </tr>
        </thead>
        <tbody>
          {#each result.campaigns as campaign (campaign.campaign_pid)}
            <tr data-testid="attribution-row">
              <td>{campaign.name}</td>
              <td>{campaign.touches}</td>
              <td>{campaign.deals}</td>
              <td>
                {#each campaign.credit as amount (amount.currency)}
                  <div>
                    {money(amount)}
                    <span
                      class="bar"
                      aria-hidden="true"
                      style:width={`${(amount.amount_minor / widest(amount.currency)) * 8}rem`}
                    ></span>
                  </div>
                {/each}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
    {#if result.unattributed_deals > 0}
      <p data-testid="attribution-unattributed">
        {t("attr.unattributed")}: {result.unattributed_deals}
        ({result.unattributed.map(money).join(", ")})
      </p>
    {/if}
    {#if result.truncated}<p class="muted">{t("attr.truncated")}</p>{/if}
    {#if result.rates.length > 0}
      <p class="muted small" data-testid="attribution-rates">
        {t("rpt.ratesUsed")} ({result.rate_date}):
        {#each result.rates as used (used.from)}
          <span class="chip">
            1 {used.from} = {used.rate} {used.to}, {used.effective_on}, {used.source}
            {#if used.inverted}({t("rpt.inverted")}){/if}
          </span>
        {/each}
      </p>
    {/if}
    {#if result.unconverted.length > 0}
      <p data-testid="attribution-unconverted">
        {t("rpt.unconverted")}:
        {#each result.unconverted as left (left.currency)}
          <span class="chip">{left.currency}: {money({ currency: left.currency, amount_minor: left.amount_minor })}</span>
        {/each}
        <span class="muted small">{t("rpt.unconvertedHint")}</span>
      </p>
    {/if}
    <p class="muted small">{result.note}</p>
  </div>
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
