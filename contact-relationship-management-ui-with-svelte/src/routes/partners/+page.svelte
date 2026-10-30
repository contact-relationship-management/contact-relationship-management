<!--
  Partners area (`/partners`): the declared-stakeholder register and
  power–interest grid (declared only — undeclared counted, never
  placed), the innovation-partnership register (forward-only
  lifecycle), and membership renewals + the lapsed list.
-->
<script lang="ts">
  import { resource } from "#lib/resource.svelte.js";
  import DataTable from "#lib/components/DataTable.svelte";
  import {
    membershipsView,
    partnershipsRegister,
    stakeholdersView,
  } from "#lib/api/crm.js";
  import { t, tf, tn } from "#lib/i18n.svelte.js";
  import { arrow } from "#lib/locale-format.js";

  type Stakeholders = Awaited<ReturnType<typeof stakeholdersView>>;
  type Partnerships = Awaited<ReturnType<typeof partnershipsRegister>>;
  type Memberships = Awaited<ReturnType<typeof membershipsView>>;

  const data = resource(async () => {
    const [stakeholders, partnerships, memberships] = await Promise.all([
      stakeholdersView(),
      partnershipsRegister(),
      membershipsView(),
    ]);
    return { stakeholders, partnerships, memberships };
  });
  const stakeholders = $derived<Stakeholders | null>(data.value?.stakeholders ?? null);
  const partnerships = $derived<Partnerships | null>(data.value?.partnerships ?? null);
  const memberships = $derived<Memberships | null>(data.value?.memberships ?? null);
  const error = $derived(data.error);

  const SCALE = [1, 2, 3, 4, 5] as const;
  const cell = (p: number, i: number) => stakeholders?.grid[`p${p}i${i}`] ?? 0;

  const partnershipColumns = $derived([
    { id: "account", header: t("col.account") },
    { id: "kind", header: t("col.kind") },
    { id: "stage", header: t("deal.stage") },
    { id: "summary", header: t("col.summary") },
  ]);
  const partnershipRows = $derived(
    (partnerships?.register ?? []).map((row) => ({
      id: row.pid,
      account: row.account ?? row.account_pid,
      kind: row.kind,
      stage: row.stage,
      summary: row.summary,
    })),
  );
  const renewalColumns = $derived([
    { id: "account", header: t("col.account") },
    { id: "status", header: t("common.status") },
    { id: "renewal", kind: "date", header: t("col.renewal") },
  ]);
  const renewalRows = $derived(
    (memberships?.renewals_due ?? []).map((row) => ({
      id: row.pid,
      account: row.account ?? "",
      status: row.status,
      renewal: row.renewal_on ?? "",
    })),
  );
</script>

<svelte:head><title>{t("nav.partners")} — CRM</title></svelte:head>

<h1>{t("nav.partners")}</h1>
{#if error}<p class="error" data-testid="error">{error}</p>{/if}

{#if stakeholders}
  <h2>{t("partner.register")}</h2>
  <p class="muted">{stakeholders.note}</p>
  <div class="cols" data-testid="stakeholder-register">
    {#each Object.entries(stakeholders.by_role) as [role, rows] (role)}
      {#if rows.length > 0}
        <section>
          <h3>{role} ({rows.length})</h3>
          <ul>
            {#each rows as row (row.pid)}
              <li>
                {row.display_name}
                <span class="muted">
                  {tf("common.daysShort", { days: row.days_since_touch })} ·
                  {row.influence !== null && row.interest !== null
                    ? tf("partner.gridScore", { influence: row.influence, interest: row.interest })
                    : t("partner.noGridScores")}
                </span>
              </li>
            {/each}
          </ul>
        </section>
      {/if}
    {/each}
  </div>
  <p class="muted">
    {tn("partner.undeclared", stakeholders.undeclared_contacts, {
      count: stakeholders.undeclared_contacts,
    })} ·
    {tn("partner.withoutScores", stakeholders.stakeholders_without_grid_scores, {
      count: stakeholders.stakeholders_without_grid_scores,
    })}
  </p>

  <h3>{t("partner.grid")}</h3>
  <table class="matrix" data-testid="stakeholder-grid">
    <thead>
      <tr><th>{tf("partner.gridAxes", { arrow: arrow() })}</th>{#each SCALE as interest (interest)}<th>{interest}</th>{/each}</tr>
    </thead>
    <tbody>
      {#each [...SCALE].reverse() as influence (influence)}
        <tr>
          <th>{influence}</th>
          {#each SCALE as interest (interest)}
            <td class:hot={influence >= 4 && interest >= 4}>{cell(influence, interest) || ""}</td>
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

{#if partnerships}
  <h2>{t("partner.partnerships")}</h2>
  <p class="muted">
    {#each Object.entries(partnerships.by_stage) as [stage, count] (stage)}
      {stage}: {count}&nbsp;
    {/each}
  </p>
  <DataTable
    label={t("partner.partnerships")}
    columns={partnershipColumns}
    rows={partnershipRows}
    testid="partnership-register"
    empty={t("partner.noPartnerships")}
  />
{/if}

{#if memberships}
  <h2>{t("partner.memberships")}</h2>
  <p class="muted">
    {tn("partner.recorded", memberships.memberships, {
      count: memberships.memberships,
      days: memberships.window_days,
    })}
  </p>
  <h3>{t("partner.renewalsDue")}</h3>
  <DataTable
    label={t("partner.renewalsDue")}
    columns={renewalColumns}
    rows={renewalRows}
    testid="membership-renewals"
    empty={t("partner.noRenewals")}
  />
  {#if memberships.lapsed.length > 0}
    <h3>{t("partner.lapsed")}</h3>
    <ul data-testid="membership-lapsed">
      {#each memberships.lapsed as row (row.pid)}
        <li>{row.account}</li>
      {/each}
    </ul>
  {/if}
{/if}

<style>
  .cols { display: flex; gap: 2rem; flex-wrap: wrap; }
  .cols section { min-width: 12rem; }
  .cols ul { list-style: none; padding: 0; margin: 0; }
  .matrix td { text-align: center; min-width: 2.5rem; }
  .matrix td.hot { background: color-mix(in srgb, #b91c1c 25%, transparent); }
</style>
