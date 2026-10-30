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
    deleteClawback,
    deleteDealSplit,
    setClawback,
    listDeals,
    setDealSplit,
    deleteQuota,
    deleteTeam,
    importTeams,
    type TeamImport,
    checkEmployment,
    type EmploymentCheck,
    deleteTerritory,
    listTerritories,
    setTerritory,
    territoryAttainment,
    listCommissionPlans,
    listTeams,
    setTeam,
    teamAttainment,
    listQuotas,
    quotaAttainment,
    setCommissionPlan,
    setQuota,
    type CommissionTier,
  } from "#lib/api/crm.js";
  import { formMinor, money } from "#lib/money.js";
  import { i18n, t } from "#lib/i18n.svelte.js";

  const today = new Date().toISOString().slice(0, 10);

  let planName = $state("");
  // One tier per line: "from_percent:rate_bps", e.g. "0:500" then "100:800".
  let planTiers = $state("0:500\n100:800");
  let ownerRef = $state("");
  let periodFrom = $state(today);
  let periodTo = $state(today);
  // No currency is assumed (CRM-T168): the person types the quota's own.
  let currency = $state("");
  let amount = $state("");
  let planPid = $state("");
  let error = $state<string | null>(null);

  // Splitting a won deal between workers (CRM-R65): one "person:percent" per line (a `person:` reference, CRM-D18), adding up to 100.
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

  // Territories of named accounts and what was won in them (CRM-R68): one organization per line.
  let territoryName = $state("");
  let territoryOwner = $state("");
  let territoryAccounts = $state("");
  const territories = resource(listTerritories);
  const territoryStanding = resource(() => territoryAttainment());
  const territoryRows = $derived(territoryStanding.value?.rows ?? []);
  const unassigned = $derived(territoryStanding.value?.unassigned ?? []);

  const saveTerritory = () =>
    run(async () => {
      await setTerritory({
        name: territoryName.trim(),
        owner_ref: territoryOwner.trim() || null,
        accounts: territoryAccounts
          .split("\n")
          .map((line) => line.trim())
          .filter(Boolean),
      });
    });

  // Taking credit back from one worker on a won deal (CRM-R71); the amount is typed in major units
  // of the deal's currency, in the page's number format.
  let clawDeal = $state("");
  let clawOwner = $state("");
  let clawAmount = $state("");
  let clawReason = $state("");

  const saveClawback = () =>
    run(async () => {
      // In the won deal's own currency and minor unit (CRM-T167).
      const deal = wonList.find((d) => d.pid === clawDeal);
      await setClawback(clawDeal, {
        owner_ref: clawOwner.trim(),
        amount_minor: formMinor(clawAmount, deal?.currency, i18n.locale).minor,
        reason: clawReason.trim() || null,
      });
    });

  // Teams from the worker service's departments (CRM-R73).
  let imported = $state<TeamImport | null>(null);
  const importFromWorkerService = () =>
    run(async () => {
      imported = await importTeams();
    });

  // Contacts the worker service says are its workers (CRM-R94).
  let checked = $state<EmploymentCheck | null>(null);
  const checkWithWorkerService = () =>
    run(async () => {
      checked = await checkEmployment();
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
      territories.reload(),
      territoryStanding.reload(),
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
      // The amount is typed in the page's own number format and sent in the currency's own minor
      // unit, exactly (CRM-T167): 1500 yen is 1500, 12,50 euros in German is 1250.
      const typed = formMinor(amount, currency, i18n.locale);
      await setQuota({
        owner_ref: ownerRef.trim(),
        period_from: periodFrom,
        period_to: periodTo,
        currency: typed.currency,
        amount_minor: typed.minor,
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

<h2>{t("quota.clawback")}</h2>
<form
  class="panel"
  data-testid="clawback-form"
  onsubmit={(e) => {
    e.preventDefault();
    void saveClawback();
  }}
>
  <label>
    {t("quota.splitDeal")}
    <select bind:value={clawDeal} required data-testid="clawback-deal">
      <option value="">—</option>
      {#each wonList as deal (deal.pid)}
        <option value={deal.pid}>{deal.name}</option>
      {/each}
    </select>
  </label>
  <label>
    {t("quota.owner")}
    <input bind:value={clawOwner} required data-testid="clawback-owner" />
  </label>
  <label>
    {t("quota.amount")}
    <input bind:value={clawAmount} required inputmode="decimal" data-testid="clawback-amount" />
  </label>
  <label>
    {t("quota.clawbackReason")}
    <input bind:value={clawReason} maxlength="200" data-testid="clawback-reason" />
  </label>
  <p class="muted">{t("quota.clawbackHint")}</p>
  <button type="submit" data-testid="clawback-set">{t("quota.clawbackSet")}</button>
  <button
    type="button"
    disabled={!clawDeal || !clawOwner.trim()}
    onclick={() => void run(() => deleteClawback(clawDeal, clawOwner.trim()))}
    data-testid="clawback-remove"
  >
    {t("quota.clawbackRemove")}
  </button>
</form>

<h2>{t("quota.teams")}</h2>
<p class="muted">{t("quota.teamsHint")}</p>
<p>
  <button type="button" onclick={() => void importFromWorkerService()} data-testid="team-import">
    {t("quota.teamImport")}
  </button>
  {#if imported}
    <span role="status" data-testid="team-import-result">
      {imported.mode === "stub"
        ? t("quota.teamImportStub")
        : `${t("quota.teamImportDone")}: ${imported.teams.length}`}
    </span>
  {/if}
</p>
<p>
  <button
    type="button"
    onclick={() => void checkWithWorkerService()}
    data-testid="employment-check"
  >
    {t("quota.employmentCheck")}
  </button>
  {#if checked}
    <span role="status" data-testid="employment-check-result">
      {checked.mode === "stub"
        ? `${t("quota.employmentCheckStub")}: ${checked.asked}`
        : `${t("quota.employmentCheckDone")}: ${checked.recorded}`}
    </span>
  {/if}
</p>
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

<h2>{t("quota.territories")}</h2>
<p class="muted">{t("quota.territoriesHint")}</p>
<form
  class="panel"
  data-testid="territory-form"
  onsubmit={(e) => {
    e.preventDefault();
    void saveTerritory();
  }}
>
  <label>
    {t("quota.territoryName")}
    <input bind:value={territoryName} required maxlength="80" data-testid="territory-name" />
  </label>
  <label>
    {t("quota.owner")}
    <input bind:value={territoryOwner} data-testid="territory-owner" />
  </label>
  <label>
    {t("quota.territoryAccounts")}
    <textarea bind:value={territoryAccounts} rows="3" required data-testid="territory-accounts"
    ></textarea>
  </label>
  <p class="muted">{t("quota.territoryAccountsHint")}</p>
  <button type="submit" data-testid="territory-set">{t("quota.territorySet")}</button>
</form>

{#if territoryRows.length > 0 || unassigned.length > 0}
  <table data-testid="territory-list">
    <thead>
      <tr>
        <th>{t("quota.territoryName")}</th>
        <th>{t("quota.owner")}</th>
        <th>{t("quota.territoryAccounts")}</th>
        <th>{t("quota.won")}</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#each territoryRows as row (row.territory_pid)}
        <tr data-testid="territory-row">
          <td>{row.name}</td>
          <td>{row.owner_ref ?? "—"}</td>
          <td>{row.accounts}</td>
          <td>
            {#each row.currencies as c (c.currency)}
              <div>{money(c.revenue_minor, c.currency, i18n.locale)} · {c.won_deals}</div>
            {/each}
          </td>
          <td>
            <button
              type="button"
              onclick={() => void run(() => deleteTerritory(row.territory_pid))}
            >
              {t("quota.delete")}
            </button>
          </td>
        </tr>
      {/each}
      {#if unassigned.length > 0}
        <tr data-testid="territory-unassigned">
          <td colspan="3">{t("quota.territoryUnassigned")}</td>
          <td colspan="2">
            {#each unassigned as c (c.currency)}
              <div>{money(c.revenue_minor, c.currency, i18n.locale)} · {c.won_deals}</div>
            {/each}
          </td>
        </tr>
      {/if}
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
