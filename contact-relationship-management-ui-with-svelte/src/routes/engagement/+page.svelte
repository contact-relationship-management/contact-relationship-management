<!--
  Engagement area (`/engagement`): relationship-cadence aging
  (untouched contacts/accounts, no-next-touch coverage), the
  engagement workload (kinds + recorded sentiment — unrecorded stays
  unrecorded), and member-account health with the silent list. All
  server-derived; derivations shown verbatim.
-->
<script lang="ts">
  import { resource } from "#lib/resource.svelte.js";
  import DataTable from "#lib/components/DataTable.svelte";
  import { cadence, engagementWorkload, membersHealth } from "#lib/api/crm.js";
  import { t, tf, tn } from "#lib/i18n.svelte.js";
  import { formatDate } from "#lib/locale-format.js";

  type Cadence = Awaited<ReturnType<typeof cadence>>;
  type Workload = Awaited<ReturnType<typeof engagementWorkload>>;
  type Members = Awaited<ReturnType<typeof membersHealth>>;

  const data = resource(async () => {
    const [aging, workload, members] = await Promise.all([
      cadence(),
      engagementWorkload(),
      membersHealth(),
    ]);
    return { aging, workload, members };
  });
  const aging = $derived<Cadence | null>(data.value?.aging ?? null);
  const workload = $derived<Workload | null>(data.value?.workload ?? null);
  const members = $derived<Members | null>(data.value?.members ?? null);
  const error = $derived(data.error);

  const contactColumns = $derived([
    { id: "contact", header: t("col.contact") },
    { id: "role", header: t("col.role") },
    { id: "days", kind: "number", header: t("col.daysSinceTouch") },
    { id: "next", header: t("col.nextTouch") },
  ]);
  const contactRows = $derived(
    (aging?.untouched_contacts ?? []).map((row) => ({
      id: row.pid,
      contact: row.display_name,
      role: row.stakeholder_role ?? "—",
      days: row.days_since_touch,
      next: row.has_next_touch ? t("common.yes") : t("common.no"),
    })),
  );
  const accountColumns = $derived([
    { id: "account", header: t("col.account") },
    { id: "role", header: t("col.role") },
    { id: "days", kind: "number", header: t("col.daysSinceTouch") },
  ]);
  const accountRows = $derived(
    (aging?.untouched_accounts ?? []).map((row) => ({
      id: row.pid,
      account: row.display_name,
      role: row.stakeholder_role ?? "—",
      days: row.days_since_touch,
    })),
  );
  const kindColumns = $derived([
    { id: "kind", header: t("col.kind") },
    { id: "touches", kind: "number", total: "sum", header: t("attr.touches") },
  ]);
  const kindRows = $derived(
    Object.entries(workload?.per_kind ?? {}).map(([kind, count]) => ({
      id: kind,
      kind,
      touches: count,
    })),
  );
  const sentimentColumns = $derived([
    { id: "sentiment", header: t("col.sentiment") },
    { id: "touches", kind: "number", total: "sum", header: t("attr.touches") },
  ]);
  const sentimentRows = $derived(
    Object.entries(workload?.sentiment ?? {}).map(([key, count]) => ({
      id: key,
      sentiment: key,
      touches: count,
    })),
  );
  const memberColumns = $derived([
    { id: "account", header: t("col.account") },
    { id: "membership", header: t("col.membership") },
    { id: "contacts", kind: "number", total: "sum", header: t("nav.contacts") },
    { id: "days", kind: "number", header: t("col.daysSinceTouch") },
    { id: "followups", kind: "number", total: "sum", header: t("col.openFollowups") },
    { id: "tickets", kind: "number", total: "sum", header: t("col.openTickets") },
    { id: "silent", kind: "chip", header: t("col.silent") },
  ]);
  const memberRows = $derived(
    (members?.accounts ?? []).map((row) => ({
      id: row.pid,
      account: row.display_name,
      membership: row.membership
        ? tf("engage.since", {
            status: row.membership.status,
            date: formatDate(row.membership.joined_on),
          })
        : "—",
      contacts: row.contacts,
      days: row.days_since_touch,
      followups: row.open_followups,
      tickets: row.open_tickets,
      silent: row.silent ? t("common.yes") : t("common.no"),
    })),
  );
</script>

<svelte:head><title>{t("nav.engagement")} — CRM</title></svelte:head>

<h1>{t("nav.engagement")}</h1>
{#if error}<p class="error" data-testid="error">{error}</p>{/if}

{#if aging}
  <h2>{t("engage.cadence")}</h2>
  <p class="muted">
    {aging.derivation} · {tf("engage.threshold", { days: aging.threshold_days })}
  </p>
  <p data-testid="cadence-coverage">
    {tn("engage.noNextTouch", aging.contacts_without_next_touch, {
      count: aging.contacts_without_next_touch,
    })}
  </p>
  <h3>{t("engage.untouchedContacts")}</h3>
  <DataTable
    label={t("engage.untouchedContacts")}
    columns={contactColumns}
    rows={contactRows}
    testid="cadence-contacts"
    empty={t("engage.everyoneTouched")}
  />
  <h3>{t("engage.untouchedAccounts")}</h3>
  <DataTable
    label={t("engage.untouchedAccounts")}
    columns={accountColumns}
    rows={accountRows}
    testid="cadence-accounts"
    empty={t("engage.everyAccountTouched")}
  />
{/if}

{#if workload}
  <h2>{tf("engage.workload", { days: workload.window_days })}</h2>
  <p class="muted">{workload.note}</p>
  <p data-testid="workload-touches">
    {tn("engage.touches", workload.touches, { count: workload.touches })}
  </p>
  <div class="cols">
    <DataTable
      label={t("engage.byKind")}
      columns={kindColumns}
      rows={kindRows}
      testid="workload-kinds"
    />
    <DataTable
      label={t("engage.bySentiment")}
      columns={sentimentColumns}
      rows={sentimentRows}
      testid="workload-sentiment"
    />
  </div>
{/if}

{#if members}
  <h2>{t("engage.memberHealth")}</h2>
  <p class="muted">
    {members.derivation} ·
    {tn("engage.silent", members.silent_accounts, {
      count: members.silent_accounts,
      days: members.threshold_days,
    })}
  </p>
  <DataTable
    label={t("engage.memberHealth")}
    columns={memberColumns}
    rows={memberRows}
    testid="member-health"
    empty={t("engage.noAccounts")}
  />
{/if}

<style>
  .cols { display: flex; gap: 2rem; flex-wrap: wrap; }
</style>
