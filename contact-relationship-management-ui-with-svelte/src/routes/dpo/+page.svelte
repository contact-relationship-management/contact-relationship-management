<!--
  DPO area (`/dpo`): consent coverage (each contact's current state
  counted verbatim), withdrawals in the window, consent events by
  source, and duplicate-contact hygiene (CRM-local rows sharing one
  person URN — the server's note states identity dedup stays
  upstream). Also the SLA register lives with support, not here.
-->
<script lang="ts">
  import { resource } from "#lib/resource.svelte.js";
  import DataTable from "#lib/components/DataTable.svelte";
  import { consentByAccount, dpo } from "#lib/api/crm.js";
  import { t, tf } from "#lib/i18n.svelte.js";

  type DpoView = Awaited<ReturnType<typeof dpo>>;
  type ByAccount = Awaited<ReturnType<typeof consentByAccount>>;
  const data = resource(async () => {
    const [view, byAccount] = await Promise.all([dpo(), consentByAccount()]);
    return { view, byAccount };
  });
  const view = $derived<DpoView | null>(data.value?.view ?? null);
  const byAccount = $derived<ByAccount | null>(data.value?.byAccount ?? null);
  const error = $derived(data.error);

  const sourceColumns = $derived([
    { id: "source", header: t("lead.source") },
    { id: "events", header: t("col.events") },
  ]);
  const sourceRows = $derived(
    Object.entries(view?.consent_events_by_source ?? {}).map(([source, count]) => ({
      id: source,
      source,
      events: count,
    })),
  );
  const accountColumns = $derived([
    { id: "account", header: t("col.account") },
    { id: "coverage", header: t("col.coverage") },
    {
      id: "withdrawals", kind: "number",
      header: tf("col.withdrawals", { days: byAccount?.window_days ?? "" }),
    },
  ]);
  const accountRows = $derived(
    (byAccount?.accounts ?? []).map((row) => ({
      id: row.pid,
      account: row.display_name,
      coverage:
        Object.entries(row.consent_coverage)
          .map(([state, count]) => `${state}: ${count}`)
          .join("  ") || t("dpo.noContacts"),
      withdrawals: row.withdrawals_in_window,
    })),
  );
  const duplicateColumns = $derived([
    { id: "person", header: t("col.personUrn") },
    { id: "contacts", header: t("nav.contacts") },
  ]);
  const duplicateRows = $derived(
    (view?.duplicate_person_refs ?? []).map((group) => ({
      id: group.person_ref,
      person: group.person_ref,
      contacts: group.contacts.map((c) => c.display_name).join("  "),
    })),
  );
</script>

<svelte:head><title>{t("nav.dpo")} — CRM</title></svelte:head>

<h1>{t("nav.dpo")}</h1>
{#if error}<p class="error" data-testid="error">{error}</p>{/if}

{#if view}
  <p class="muted">{view.note}</p>
  <section class="tiles" data-testid="dpo-tiles">
    <div class="tile"><strong>{view.contacts}</strong><span>{t("dpo.contacts")}</span></div>
    {#each Object.entries(view.consent_coverage) as [state, count] (state)}
      <div class="tile"><strong>{count}</strong><span>{tf("dpo.consentState", { state })}</span></div>
    {/each}
    <div class="tile">
      <strong>{view.withdrawals_in_window}</strong>
      <span>{tf("dpo.withdrawals", { days: view.window_days })}</span>
    </div>
  </section>

  <h2>{t("dpo.bySource")}</h2>
  <DataTable
    label={t("dpo.bySource")}
    columns={sourceColumns}
    rows={sourceRows}
    testid="dpo-sources"
    empty={t("dpo.noEvents")}
  />

  <h2>{t("dpo.byAccount")}</h2>
  {#if byAccount}
    <DataTable
      label={t("dpo.byAccount")}
      columns={accountColumns}
      rows={accountRows}
      testid="dpo-by-account"
      empty={t("dpo.noAccounts")}
    />
  {/if}

  <h2>{t("dpo.duplicates")}</h2>
  <DataTable
    label={t("dpo.duplicates")}
    columns={duplicateColumns}
    rows={duplicateRows}
    testid="dpo-duplicates"
    empty={t("dpo.noDuplicates")}
  />
{/if}
