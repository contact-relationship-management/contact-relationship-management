<!--
  Quotas and commission (`/quotas`): what each worker is to win in a period, the tiered plan
  that pays on it, and where each worker stands today (CRM-R59). Attainment is the worker's won
  revenue in the quota's currency; other currencies are shown, never converted.
-->
<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import { resource } from "#lib/resource.svelte.js";
  import { ratioPercent } from "#lib/format.js";
  import {
    deleteCommissionPlan,
    deleteDealSplit,
    listDeals,
    setDealSplit,
    deleteQuota,
    deleteTeam,
    listCommissionPlans,
    listTeams,
    setTeam,
    teamAttainment,
    listQuotas,
    money,
    quotaAttainment,
    setCommissionPlan,
    setQuota,
    type CommissionTier,
  } from "#lib/api/crm.js";
  import { i18n, t } from "#lib/i18n.svelte.js";

  const today = new Date().toISOString().slice(0, 10);

  let planName = $state("");
  // One tier per line: "from_percent:rate_bps", e.g. "0:500" then "100:800".
  let planTiers = $state("0:500\n100:800");
  let ownerRef = $state("");
  let periodFrom = $state(today);
  let periodTo = $state(today);
  let currency = $state("GBP");
  let amount = $state("");
  let planPid = $state("");
  let error = $state<string | null>(null);

  // Splitting a won deal between workers (CRM-R65): one "worker:percent" per line, adding up to 100.
  let splitDeal = $state("");
  let splitShares = $state("");
  const wonDeals = resource(async () => (await listDeals()).filter((d) => d.won));
  const wonList = $derived(wonDeals.value ?? []);

  const saveSplit = () =>
    run(async () => {
      const shares = splitShares
        .split("\n")
        .map((line) => line.trim())
        .filter(Boolean)
        .map((line) => {
          const at = line.lastIndexOf(":");
          return {
            owner_ref: line.slice(0, at).trim(),
            // A percentage typed as 12.5 is 1250 basis points.
            share_bps: Math.round(Number(line.slice(at + 1)) * 100),
          };
        });
      await setDealSplit(splitDeal, shares);
    });

  // Teams of workers and what they reach together (CRM-R67): one worker per line.
  let teamName = $state("");
  let teamMembers = $state("");
  const teams = resource(listTeams);
  const teamStanding = resource(() => teamAttainment());
  const teamList = $derived(teams.value ?? []);
  const teamRows = $derived(teamStanding.value?.rows ?? []);

  const saveTeam = () =>
    run(async () => {
      await setTeam({
        name: teamName.trim(),
        members: teamMembers
          .split("\n")
          .map((line) => line.trim())
          .filter(Boolean),
      });
    });

  const plans = resource(listCommissionPlans);
  const quotas = resource(listQuotas);
  const standing = resource(() => quotaAttainment());
  const planList = $derived(plans.value ?? []);
  const quotaList = $derived(quotas.value ?? []);
  const rows = $derived(standing.value?.rows ?? []);

  const reloadAll = () =>
    Promise.all([
      plans.reload(),
      quotas.reload(),
      standing.reload(),
      teams.reload(),
      teamStanding.reload(),
    ]);

  function parseTiers(text: string): CommissionTier[] {
    return text
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean)
      .map((line) => {
        const [from = NaN, rate = NaN] = line.split(":").map((part) => Number(part.trim()));
        return { from_percent: from, rate_bps: rate };
      });
  }

  async function run(action: () => Promise<unknown>) {
    error = null;
    try {
      await action();
      await reloadAll();
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  const savePlan = () =>
    run(() => setCommissionPlan({ name: planName.trim(), tiers: parseTiers(planTiers) }));

  const saveQuota = () =>
    run(async () => {
      // The amount is typed in major units and sent in minor units, exactly.
      const minor = Math.round(Number(amount) * 100);
      await setQuota({
        owner_ref: ownerRef.trim(),
        period_from: periodFrom,
        period_to: periodTo,
        currency: currency.trim().toUpperCase(),
        amount_minor: minor,
        plan_pid: planPid || null,
      });
      amount = "";
    });

  const planName_ = (pid: string | null) => planList.find((p) => p.pid === pid)?.name ?? "—";
</script>

<h1>{t("quota.title")}</h1>
<p class="muted">{t("quota.hint")}</p>

{#if error}<p class="error" role="alert" data-testid="quota-error">{error}</p>{/if}

<h2>{t("quota.standing")}</h2>
{#if rows.length === 0}
  <p class="muted" data-testid="quota-standing-none">{t("quota.standingNone")}</p>
{:else}
  <table data-testid="quota-standing">
    <thead>
      <tr>
        <th>{t("quota.owner")}</th>
        <th>{t("quota.quota")}</th>
        <th>{t("quota.won")}</th>
        <th>{t("quota.attainment")}</th>
        <th>{t("quota.remaining")}</th>
        <th>{t("quota.commission")}</th>
      </tr>
    </thead>
    <tbody>
      {#each rows as row (row.quota_pid)}
        <tr data-testid="quota-standing-row">
          <td>{row.owner_ref}</td>
          <td>{money(row.quota_minor, row.currency, i18n.locale)}</td>
          <td>
            {money(row.revenue_minor, row.currency, i18n.locale)}
            {#each Object.entries(row.other_currencies) as [code, minor] (code)}
              <span class="chip">{money(minor, code, i18n.locale)}</span>
            {/each}
          </td>
          <td>{ratioPercent(row.attainment)}</td>
          <td>{money(row.remaining_minor, row.currency, i18n.locale)}</td>
          <td>{money(row.commission_minor, row.currency, i18n.locale)}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<h2>{t("quota.quotas")}</h2>
<form
  class="panel"
  data-testid="quota-form"
  onsubmit={(e) => {
    e.preventDefault();
    void saveQuota();
  }}
>
  <label>
    {t("quota.owner")}
    <input bind:value={ownerRef} required data-testid="quota-owner" />
  </label>
  <label>
    {t("quota.from")}
    <input type="date" bind:value={periodFrom} required data-testid="quota-from" />
  </label>
  <label>
    {t("quota.to")}
    <input type="date" bind:value={periodTo} required data-testid="quota-to" />
  </label>
  <label>
    {t("quota.currency")}
    <input bind:value={currency} required maxlength="3" size="4" data-testid="quota-currency" />
  </label>
  <label>
    {t("quota.amount")}
    <input bind:value={amount} required inputmode="decimal" data-testid="quota-amount" />
  </label>
  <label>
    {t("quota.plan")}
    <select bind:value={planPid} data-testid="quota-plan">
      <option value="">—</option>
      {#each planList as plan (plan.pid)}
        <option value={plan.pid}>{plan.name}</option>
      {/each}
    </select>
  </label>
  <button type="submit" data-testid="quota-set">{t("quota.set")}</button>
</form>

{#if quotaList.length === 0}
  <p class="muted" data-testid="quota-none">{t("quota.none")}</p>
{:else}
  <table data-testid="quota-list">
    <thead>
      <tr>
        <th>{t("quota.owner")}</th>
        <th>{t("quota.from")}</th>
        <th>{t("quota.to")}</th>
        <th>{t("quota.quota")}</th>
        <th>{t("quota.plan")}</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#each quotaList as row (row.pid)}
        <tr data-testid="quota-row">
          <td>{row.owner_ref}</td>
          <td>{row.period_from}</td>
          <td>{row.period_to}</td>
          <td>{money(row.amount_minor, row.currency, i18n.locale)}</td>
          <td>{planName_(row.plan_pid)}</td>
          <td>
            <button type="button" onclick={() => void run(() => deleteQuota(row.pid))}>
              {t("quota.delete")}
            </button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<h2>{t("quota.split")}</h2>
<form
  class="panel"
  data-testid="split-form"
  onsubmit={(e) => {
    e.preventDefault();
    void saveSplit();
  }}
>
  <label>
    {t("quota.splitDeal")}
    <select bind:value={splitDeal} required data-testid="split-deal">
      <option value="">—</option>
      {#each wonList as deal (deal.pid)}
        <option value={deal.pid}>{deal.name}</option>
      {/each}
    </select>
  </label>
  <label>
    {t("quota.splitShares")}
    <textarea bind:value={splitShares} rows="3" required data-testid="split-shares"></textarea>
  </label>
  <p class="muted">{t("quota.splitHint")}</p>
  <button type="submit" data-testid="split-set">{t("quota.splitSet")}</button>
  <button
    type="button"
    disabled={!splitDeal}
    onclick={() => void run(() => deleteDealSplit(splitDeal))}
    data-testid="split-remove"
  >
    {t("quota.splitRemove")}
  </button>
</form>

<h2>{t("quota.teams")}</h2>
<p class="muted">{t("quota.teamsHint")}</p>
<form
  class="panel"
  data-testid="team-form"
  onsubmit={(e) => {
    e.preventDefault();
    void saveTeam();
  }}
>
  <label>
    {t("quota.teamName")}
    <input bind:value={teamName} required maxlength="80" data-testid="team-name" />
  </label>
  <label>
    {t("quota.teamMembers")}
    <textarea bind:value={teamMembers} rows="3" required data-testid="team-members"></textarea>
  </label>
  <p class="muted">{t("quota.teamMembersHint")}</p>
  <button type="submit" data-testid="team-set">{t("quota.teamSet")}</button>
</form>

{#if teamList.length > 0}
  <table data-testid="team-list">
    <thead>
      <tr>
        <th>{t("quota.teamName")}</th>
        <th>{t("quota.teamWorkers")}</th>
        <th>{t("quota.quota")}</th>
        <th>{t("quota.won")}</th>
        <th>{t("quota.attainment")}</th>
        <th>{t("quota.remaining")}</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#each teamRows as row (row.team_pid)}
        <tr data-testid="team-row">
          <td>
            {row.name}
            {#each row.without_quota as member (member)}
              <span class="chip">{member}: {t("quota.teamNoQuota")}</span>
            {/each}
          </td>
          <td>{row.members.join(", ")}</td>
          <td>
            {#each row.currencies as c (c.currency)}
              <div>{money(c.quota_minor, c.currency, i18n.locale)}</div>
            {/each}
          </td>
          <td>
            {#each row.currencies as c (c.currency)}
              <div>{money(c.revenue_minor, c.currency, i18n.locale)}</div>
            {/each}
            {#each Object.entries(row.other_currencies) as [code, minor] (code)}
              <span class="chip">{money(minor, code, i18n.locale)}</span>
            {/each}
          </td>
          <td>
            {#each row.currencies as c (c.currency)}
              <div>{ratioPercent(c.attainment)}</div>
            {/each}
          </td>
          <td>
            {#each row.currencies as c (c.currency)}
              <div>{money(c.remaining_minor, c.currency, i18n.locale)}</div>
            {/each}
          </td>
          <td>
            <button type="button" onclick={() => void run(() => deleteTeam(row.team_pid))}>
              {t("quota.delete")}
            </button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<h2>{t("quota.plans")}</h2>
<form
  class="panel"
  data-testid="plan-form"
  onsubmit={(e) => {
    e.preventDefault();
    void savePlan();
  }}
>
  <label>
    {t("quota.planName")}
    <input bind:value={planName} required maxlength="80" data-testid="plan-name" />
  </label>
  <label>
    {t("quota.tiers")}
    <textarea bind:value={planTiers} rows="3" required data-testid="plan-tiers"></textarea>
  </label>
  <p class="muted">{t("quota.tiersHint")}</p>
  <button type="submit" data-testid="plan-set">{t("quota.setPlan")}</button>
</form>

{#if planList.length > 0}
  <table data-testid="plan-list">
    <thead>
      <tr>
        <th>{t("quota.planName")}</th>
        <th>{t("quota.tiers")}</th>
        <th>{t("quota.usedBy")}</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#each planList as plan (plan.pid)}
        <tr data-testid="plan-row">
          <td>{plan.name}</td>
          <td>
            {plan.tiers.map((tier) => `${tier.from_percent}% → ${tier.rate_bps / 100}%`).join(", ")}
          </td>
          <td>{plan.quotas}</td>
          <td>
            <button type="button" onclick={() => void run(() => deleteCommissionPlan(plan.pid))}>
              {t("quota.delete")}
            </button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
