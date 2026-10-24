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
  import { t } from "#lib/i18n.svelte.js";

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

  const contactColumns = [
    { id: "contact", header: "Contact" },
    { id: "role", header: "Role" },
    { id: "days", kind: "number", header: "Days since touch" },
    { id: "next", header: "Next touch?" },
  ];
  const contactRows = $derived(
    (aging?.untouched_contacts ?? []).map((row) => ({
      id: row.pid,
      contact: row.display_name,
      role: row.stakeholder_role ?? "—",
      days: row.days_since_touch,
      next: row.has_next_touch ? "yes" : "no",
    })),
  );
  const accountColumns = [
    { id: "account", header: "Account" },
    { id: "role", header: "Role" },
    { id: "days", kind: "number", header: "Days since touch" },
  ];
  const accountRows = $derived(
    (aging?.untouched_accounts ?? []).map((row) => ({
      id: row.pid,
      account: row.display_name,
      role: row.stakeholder_role ?? "—",
      days: row.days_since_touch,
    })),
  );
  const kindColumns = [
    { id: "kind", header: "Kind" },
    { id: "touches", kind: "number", total: "sum", header: "Touches" },
  ];
  const kindRows = $derived(
    Object.entries(workload?.per_kind ?? {}).map(([kind, count]) => ({
      id: kind,
      kind,
      touches: count,
    })),
  );
  const sentimentColumns = [
    { id: "sentiment", header: "Sentiment" },
    { id: "touches", kind: "number", total: "sum", header: "Touches" },
  ];
  const sentimentRows = $derived(
    Object.entries(workload?.sentiment ?? {}).map(([key, count]) => ({
      id: key,
      sentiment: key,
      touches: count,
    })),
  );
  const memberColumns = [
    { id: "account", header: "Account" },
    { id: "membership", header: "Membership" },
    { id: "contacts", kind: "number", total: "sum", header: "Contacts" },
    { id: "days", kind: "number", header: "Days since touch" },
    { id: "followups", kind: "number", total: "sum", header: "Open follow-ups" },
    { id: "tickets", kind: "number", total: "sum", header: "Open tickets" },
    { id: "silent", kind: "chip", header: "Silent?" },
  ];
  const memberRows = $derived(
    (members?.accounts ?? []).map((row) => ({
      id: row.pid,
      account: row.display_name,
      membership: row.membership
        ? `${row.membership.status} since ${row.membership.joined_on}`
        : "—",
      contacts: row.contacts,
      days: row.days_since_touch,
      followups: row.open_followups,
      tickets: row.open_tickets,
      silent: row.silent ? "yes" : "no",
    })),
  );
</script>

<svelte:head><title>{t("nav.engagement")} — CRM</title></svelte:head>

<h1>{t("nav.engagement")}</h1>
{#if error}<p class="error" data-testid="error">{error}</p>{/if}

{#if aging}
  <h2>Relationship cadence</h2>
  <p class="muted">{aging.derivation} · threshold {aging.threshold_days}d</p>
  <p data-testid="cadence-coverage">
    <strong>{aging.contacts_without_next_touch}</strong> contacts with no planned next touch
  </p>
  <h3>Untouched contacts</h3>
  <DataTable
    label="Untouched contacts"
    columns={contactColumns}
    rows={contactRows}
    testid="cadence-contacts"
    empty="Everyone touched within the threshold."
  />
  <h3>Untouched accounts</h3>
  <DataTable
    label="Untouched accounts"
    columns={accountColumns}
    rows={accountRows}
    testid="cadence-accounts"
    empty="Every account touched within the threshold."
  />
{/if}

{#if workload}
  <h2>Workload ({workload.window_days}d)</h2>
  <p class="muted">{workload.note}</p>
  <p data-testid="workload-touches"><strong>{workload.touches}</strong> touches</p>
  <div class="cols">
    <DataTable label="Touches by kind" columns={kindColumns} rows={kindRows} testid="workload-kinds" />
    <DataTable
      label="Touches by sentiment"
      columns={sentimentColumns}
      rows={sentimentRows}
      testid="workload-sentiment"
    />
  </div>
{/if}

{#if members}
  <h2>Member health</h2>
  <p class="muted">
    {members.derivation} · {members.silent_accounts} silent (over {members.threshold_days}d)
  </p>
  <DataTable
    label="Member health"
    columns={memberColumns}
    rows={memberRows}
    testid="member-health"
    empty="No accounts yet."
  />
{/if}

<style>
  .cols { display: flex; gap: 2rem; flex-wrap: wrap; }
</style>
