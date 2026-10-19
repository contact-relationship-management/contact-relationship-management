<script lang="ts">
  import DataTable from "#lib/components/DataTable.svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import {
    eraseContact,
    getContact,
    getContactDuplicates,
    money,
    recordConsent,
    type Duplicate,
  } from "#lib/api/crm.js";
  import { i18n, t } from "#lib/i18n.svelte.js";

  let detail = $state<Awaited<ReturnType<typeof getContact>> | null>(null);
  let error = $state<string | null>(null);
  let actionError = $state<string | null>(null);
  let erasing = $state(false);
  // Likely duplicates (CRM-R23). Secondary: a failure here leaves the page as
  // it was, so it is not an error state.
  let duplicates = $state<Duplicate[] | null>(null);

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

  async function loadDuplicates() {
    duplicates = null;
    try {
      duplicates = (await getContactDuplicates(pid)).duplicates;
    } catch {
      duplicates = null;
    }
  }

  $effect(() => {
    if (pid) {
      void load();
      void loadDuplicates();
    }
  });

  // Rounded down, so a near-certain match never reads as "100%": only a
  // shared person record is certain.
  const percent = (d: Duplicate) =>
    new Intl.NumberFormat(i18n.locale, { style: "percent", maximumFractionDigits: 0 }).format(
      d.deterministic ? 1 : Math.floor(d.probability * 100) / 100,
    );
  // Only fields the CRM can supply are named; the matcher also reports phone,
  // which is always missing here.
  const FIELD_LABELS = {
    given_name: "contact.dupField.given_name",
    family_name: "contact.dupField.family_name",
    email: "contact.dupField.email",
    organization: "contact.dupField.organization",
    job_title: "contact.dupField.job_title",
  } as const;
  const fieldNames = (d: Duplicate, outcome: "agree" | "disagree") =>
    d.fields
      .filter((f) => f.outcome === outcome && f.field in FIELD_LABELS)
      .map((f) => t(FIELD_LABELS[f.field as keyof typeof FIELD_LABELS]))
      .join(", ");

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
    { id: "kind", kind: "chip", header: "Kind" },
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
    { id: "amount", kind: "money", header: "Amount", accessor: (r: Record<string, unknown>) => r.minor, format: (_v: unknown, r: Record<string, unknown>) => String(r.amount) },
    { id: "outcome", kind: "chip", header: "Outcome" },
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
    { id: "status", kind: "chip", header: "Status" },
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

  <h2>{t("contact.duplicates")}</h2>
  <p class="muted">{t("contact.duplicatesHint")}</p>
  {#if duplicates !== null}
    {#if duplicates.length === 0}
      <p data-testid="duplicates-none">{t("contact.duplicatesNone")}</p>
    {:else}
      <ul data-testid="duplicates">
        {#each duplicates as d (d.contact.pid)}
          <li data-testid="duplicate">
            <a href={`/contacts/${d.contact.pid}`}>{d.contact.display_name}</a>
            {#if d.contact.job_title}· {d.contact.job_title}{/if}
            · <span class="chip"
              >{d.deterministic
                ? t("contact.dupCertain")
                : d.verdict === "match"
                  ? t("contact.dupMatch")
                  : t("contact.dupReview")}</span
            >
            · <span data-testid="duplicate-probability">{percent(d)}</span>
            {#if !d.deterministic}
              <br />
              <span class="muted small">
                {#if fieldNames(d, "agree")}{t("contact.dupAgrees")}: {fieldNames(d, "agree")}.{/if}
                {#if fieldNames(d, "disagree")}
                  {t("contact.dupDiffers")}: {fieldNames(d, "disagree")}.{/if}
              </span>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/if}

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
