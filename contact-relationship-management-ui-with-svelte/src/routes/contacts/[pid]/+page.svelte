<script lang="ts">
  import DataTable from "#lib/components/DataTable.svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { eraseContact, getContact, money, recordConsent } from "#lib/api/crm.js";
  import { i18n, t } from "#lib/i18n.svelte.js";

  let detail = $state<Awaited<ReturnType<typeof getContact>> | null>(null);
  let error = $state<string | null>(null);
  let actionError = $state<string | null>(null);
  let erasing = $state(false);

  const pid = $derived(page.params.pid ?? "");
  // Rough client-side gate from data already on the page — the
  // service is the authority (CRM-R20 also checks nurture
  // enrolments, which this page does not load); this only avoids
  // offering the button when it would obviously be refused.
  const hasOpenEngagement = $derived(
    (detail?.deals.some((deal) => deal.closed_at === null) ?? false) ||
      (detail?.tickets.some((ticket) => ticket.status === "open" || ticket.status === "pending") ??
        false),
  );

  async function load() {
    try {
      detail = await getContact(pid);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  $effect(() => {
    if (pid) void load();
  });

  async function consent(action: "granted" | "withdrawn") {
    actionError = null;
    try {
      await recordConsent(pid, action);
      await load();
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  async function erase() {
    if (!confirm(t("contact.eraseConfirm"))) return;
    actionError = null;
    erasing = true;
    try {
      await eraseContact(pid);
      await goto("/contacts");
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    } finally {
      erasing = false;
    }
  }

  // Tables have headers now (the lists were headerless); English literals, as on
  // the other detail pages.
  const timelineColumns = [
    { id: "when", header: "When", accessor: (r: Record<string, unknown>) => r.at, format: (_v: unknown, r: Record<string, unknown>) => String(r.when) },
    { id: "kind", header: "Kind" },
    { id: "summary", header: "Summary" },
  ];
  const timelineRows = $derived(
    (detail?.activities ?? []).map((a) => ({
      id: a.pid,
      at: a.occurred_at,
      when: new Date(a.occurred_at).toLocaleString(),
      kind: a.kind,
      summary: a.summary,
    })),
  );
  const dealColumns = [
    { id: "deal", header: "Deal" },
    { id: "amount", header: "Amount", accessor: (r: Record<string, unknown>) => r.minor, format: (_v: unknown, r: Record<string, unknown>) => String(r.amount) },
    { id: "outcome", header: "Outcome" },
  ];
  const dealRows = $derived(
    (detail?.deals ?? []).map((d) => ({
      id: d.pid,
      deal: d.name,
      minor: d.amount_minor,
      amount: money(d.amount_minor, d.currency, i18n.locale),
      outcome: d.closed_at ? (d.won ? t("deal.won") : t("deal.lost")) : "",
    })),
  );
  const ticketColumns = [
    { id: "ticket", header: "Ticket" },
    { id: "status", header: "Status" },
  ];
  const ticketRows = $derived(
    (detail?.tickets ?? []).map((x) => ({ id: x.pid, ticket: x.title, status: x.status })),
  );
</script>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if detail === null}
  <p>{t("common.loading")}</p>
{:else}
  <h1>{detail.contact.display_name}</h1>
  <div class="panel">
    <p>
      {detail.contact.job_title ?? ""} ·
      <span class="chip">{detail.contact.status}</span> ·
      {t("contact.consent")}:
      <strong data-testid="consent">{detail.contact.marketing_consent}</strong>
    </p>
    {#if detail.contact.marketing_consent === "granted"}
      <button onclick={() => void consent("withdrawn")}>{t("contact.withdraw")}</button>
    {:else}
      <button onclick={() => void consent("granted")}>{t("contact.grant")}</button>
    {/if}
    {#if actionError}
      <p class="error" data-testid="action-error">{actionError}</p>
    {/if}
  </div>

  <div class="panel">
    <a
      href={`/api/proxy/contacts/${pid}/subject-access`}
      target="_blank"
      rel="noreferrer"
      data-testid="subject-access"
    >
      {t("contact.subjectAccess")}
    </a>
    {#if !hasOpenEngagement}
      <button onclick={() => void erase()} disabled={erasing} data-testid="erase">
        {t("contact.erase")}
      </button>
    {/if}
  </div>

  <h2>{t("contact.timeline")}</h2>
  <DataTable
    label={t("contact.timeline")}
    columns={timelineColumns}
    rows={timelineRows}
    testid="timeline"
  />

  <h2>{t("nav.deals")}</h2>
  <DataTable
    label={t("nav.deals")}
    columns={dealColumns}
    rows={dealRows}
    testid="contact-deals"
  />

  <h2>{t("nav.tickets")}</h2>
  <DataTable
    label={t("nav.tickets")}
    columns={ticketColumns}
    rows={ticketRows}
    testid="contact-tickets"
  />
{/if}
