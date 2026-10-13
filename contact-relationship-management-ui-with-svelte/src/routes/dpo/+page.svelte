<!--
  DPO area (`/dpo`): consent coverage (each contact's current state
  counted verbatim), withdrawals in the window, consent events by
  source, and duplicate-contact hygiene (CRM-local rows sharing one
  person URN — the server's note states identity dedup stays
  upstream). Also the SLA register lives with support, not here.
-->
<script lang="ts">
  import DataTable from "#lib/components/DataTable.svelte";
  import { consentByAccount, dpo } from "#lib/api/crm.js";
  import { t } from "#lib/i18n.svelte.js";

  type DpoView = Awaited<ReturnType<typeof dpo>>;
  type ByAccount = Awaited<ReturnType<typeof consentByAccount>>;
  let view = $state<DpoView | null>(null);
  let byAccount = $state<ByAccount | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        view = await dpo();
        byAccount = await consentByAccount();
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });

  const sourceColumns = [
    { id: "source", header: "Source" },
    { id: "events", header: "Events" },
  ];
  const sourceRows = $derived(
    Object.entries(view?.consent_events_by_source ?? {}).map(([source, count]) => ({
      id: source,
      source,
      events: count,
    })),
  );
  const accountColumns = $derived([
    { id: "account", header: "Account" },
    { id: "coverage", header: "Coverage" },
    {
      id: "withdrawals",
      header: `Withdrawals (${byAccount?.window_days ?? ""}d)`,
    },
  ]);
  const accountRows = $derived(
    (byAccount?.accounts ?? []).map((row) => ({
      id: row.pid,
      account: row.display_name,
      coverage:
        Object.entries(row.consent_coverage)
          .map(([state, count]) => `${state}: ${count}`)
          .join("  ") || "no contacts",
      withdrawals: row.withdrawals_in_window,
    })),
  );
  const duplicateColumns = [
    { id: "person", header: "Person URN" },
    { id: "contacts", header: "Contacts" },
  ];
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
    <div class="tile"><strong>{view.contacts}</strong><span>contacts</span></div>
    {#each Object.entries(view.consent_coverage) as [state, count] (state)}
      <div class="tile"><strong>{count}</strong><span>consent: {state}</span></div>
    {/each}
    <div class="tile">
      <strong>{view.withdrawals_in_window}</strong>
      <span>withdrawals ({view.window_days}d)</span>
    </div>
  </section>

  <h2>Consent events by source</h2>
  <DataTable
    label="Consent events by source"
    columns={sourceColumns}
    rows={sourceRows}
    testid="dpo-sources"
    empty="No consent events recorded."
  />

  <h2>Consent by account</h2>
  {#if byAccount}
    <DataTable
      label="Consent by account"
      columns={accountColumns}
      rows={accountRows}
      testid="dpo-by-account"
      empty="No accounts."
    />
  {/if}

  <h2>Duplicate contact rows</h2>
  <DataTable
    label="Duplicate contact rows"
    columns={duplicateColumns}
    rows={duplicateRows}
    testid="dpo-duplicates"
    empty="No duplicates — one row per person."
  />
{/if}
