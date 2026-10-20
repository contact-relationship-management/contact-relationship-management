<script lang="ts">
  import DataTable from "#lib/components/DataTable.svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import {
    eraseContact,
    getContact,
    addContactLink,
    getContactDuplicates,
    getContactLinks,
    getContactMerges,
    LINK_SYSTEMS,
    mergeContact,
    removeLink,
    unmergeContact,
    money,
    type MergeResult,
    recordConsent,
    type Duplicate,
    type ExternalLink,
    type LinkSystem,
    type MergeEntry,
    type UnmergeResult,
  } from "#lib/api/crm.js";
  import { i18n, t } from "#lib/i18n.svelte.js";

  let detail = $state<Awaited<ReturnType<typeof getContact>> | null>(null);
  let error = $state<string | null>(null);
  let actionError = $state<string | null>(null);
  let erasing = $state(false);
  // Likely duplicates (CRM-R23). Secondary: a failure here leaves the page as
  // it was, so it is not an error state.
  let duplicates = $state<Duplicate[] | null>(null);

  // Merging a duplicate into this contact (CRM-R24): a dry run first, so the
  // confirmation shows exactly what will move.
  let merging = $state<Duplicate | null>(null);
  let preview = $state<MergeResult | null>(null);
  let mergeReason = $state("");
  let mergeBusy = $state(false);
  let mergeError = $state<string | null>(null);
  let mergedName = $state<string | null>(null);

  // Undoing a merge (CRM-R26): a preview first, then a reason.
  let merges = $state<MergeEntry[]>([]);
  let undoing = $state<MergeEntry | null>(null);
  let undoPreview = $state<UnmergeResult | null>(null);
  let undoReason = $state("");
  let undoBusy = $state(false);
  let undoError = $state<string | null>(null);
  let undoneName = $state<string | null>(null);

  // Links to the same contact in other systems (CRM-R27). Only links: nothing
  // is fetched from, or sent to, those systems.
  let links = $state<ExternalLink[] | null>(null);
  let linkSystem = $state<LinkSystem>("hubspot");
  let linkUrl = $state("");
  let linkLabel = $state("");
  let linkBusy = $state(false);
  let linkError = $state<string | null>(null);

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

  async function loadLinks() {
    try {
      links = (await getContactLinks(pid)).links;
    } catch {
      links = null;
    }
  }

  async function loadMerges() {
    try {
      merges = (await getContactMerges(pid)).merges;
    } catch {
      merges = [];
    }
  }

  async function startUndo(entry: MergeEntry) {
    undoError = null;
    undoneName = null;
    undoReason = "";
    undoPreview = null;
    undoing = entry;
    try {
      undoPreview = await unmergeContact(entry.merged_pid, "preview", true);
    } catch (cause) {
      undoError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  function cancelUndo() {
    undoing = null;
    undoPreview = null;
    undoError = null;
  }

  async function confirmUndo() {
    if (!undoing || !undoReason.trim()) return;
    undoBusy = true;
    undoError = null;
    try {
      await unmergeContact(undoing.merged_pid, undoReason.trim(), false);
      undoneName = undoing.merged_name;
      cancelUndo();
      await Promise.all([load(), loadDuplicates(), loadLinks(), loadMerges()]);
    } catch (cause) {
      undoError = cause instanceof Error ? cause.message : String(cause);
    } finally {
      undoBusy = false;
    }
  }

  async function addLink() {
    if (!linkUrl.trim()) return;
    linkBusy = true;
    linkError = null;
    try {
      await addContactLink(pid, { system: linkSystem, url: linkUrl.trim(), label: linkLabel });
      linkUrl = "";
      linkLabel = "";
      await loadLinks();
    } catch (cause) {
      linkError = cause instanceof Error ? cause.message : String(cause);
    } finally {
      linkBusy = false;
    }
  }

  async function dropLink(link: ExternalLink) {
    linkError = null;
    try {
      await removeLink(link.pid);
      await loadLinks();
    } catch (cause) {
      linkError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  const systemName = (system: LinkSystem) =>
    ({
      hubspot: "HubSpot",
      salesforce: "Salesforce",
      attio: "Attio",
      pipedrive: "Pipedrive",
      dynamics: "Microsoft Dynamics",
      other: t("contact.linkSystemOther"),
    })[system];

  $effect(() => {
    if (pid) {
      void load();
      void loadDuplicates();
      void loadLinks();
      void loadMerges();
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

  const TABLE_LABELS = {
    activities: "contact.mergeTable.activities",
    leads: "contact.mergeTable.leads",
    deals: "contact.mergeTable.deals",
    tickets: "contact.mergeTable.tickets",
    email_deliveries: "contact.mergeTable.email_deliveries",
    consent_events: "contact.mergeTable.consent_events",
    nurture_enrollments: "contact.mergeTable.nurture_enrollments",
    working_group_members: "contact.mergeTable.working_group_members",
    external_links: "contact.mergeTable.external_links",
  } as const;
  const counted = (counts: Record<string, number>) =>
    Object.entries(counts)
      .filter(([key, n]) => n > 0 && key in TABLE_LABELS)
      .map(([key, n]) => `${t(TABLE_LABELS[key as keyof typeof TABLE_LABELS])}: ${n}`)
      .join(", ");

  async function startMerge(d: Duplicate) {
    mergeError = null;
    mergedName = null;
    mergeReason = "";
    preview = null;
    merging = d;
    try {
      preview = await mergeContact(d.contact.pid, pid, "preview", true);
    } catch (cause) {
      mergeError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  function cancelMerge() {
    merging = null;
    preview = null;
    mergeError = null;
  }

  async function confirmMerge() {
    if (!merging || !mergeReason.trim()) return;
    mergeBusy = true;
    mergeError = null;
    try {
      await mergeContact(merging.contact.pid, pid, mergeReason.trim(), false);
      mergedName = merging.contact.display_name;
      cancelMerge();
      await load();
      await loadDuplicates();
      await loadMerges();
      await loadLinks();
    } catch (cause) {
      mergeError = cause instanceof Error ? cause.message : String(cause);
    } finally {
      mergeBusy = false;
    }
  }

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
            <br />
            <button
              onclick={() => void startMerge(d)}
              disabled={merging !== null}
              data-testid="merge"
            >
              {t("contact.merge")}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
  {#if mergedName}
    <p role="status" data-testid="merge-done">{mergedName}: {t("contact.mergeDone")}</p>
  {/if}
  {#if merging}
    <section data-testid="merge-panel" aria-labelledby="merge-heading">
      <h3 id="merge-heading">{t("contact.merge")}: {merging.contact.display_name}</h3>
      {#if preview}
        <p data-testid="merge-moves">
          {t("contact.mergeMoves")}:
          {counted(preview.moved) || t("contact.mergeNothing")}.
        </p>
        {#if counted(preview.dropped)}
          <p data-testid="merge-drops">
            {t("contact.mergeDrops")}: {counted(preview.dropped)}.
          </p>
        {/if}
        <p data-testid="merge-consent">
          {t("contact.mergeConsent")}: <strong>{preview.consent.result}</strong>
        </p>
        <p class="muted">{t("contact.mergeWarn")}</p>
        <label>
          {t("contact.mergeReason")}
          <input bind:value={mergeReason} data-testid="merge-reason" />
        </label>
        <button
          onclick={() => void confirmMerge()}
          disabled={mergeBusy || !mergeReason.trim()}
          data-testid="merge-confirm"
        >
          {t("contact.mergeConfirm")}
        </button>
      {/if}
      <button onclick={cancelMerge} data-testid="merge-cancel">{t("contact.mergeCancel")}</button>
      {#if mergeError}<p role="alert" data-testid="merge-error">{mergeError}</p>{/if}
    </section>
  {/if}

  {#if merges.length > 0 || undoneName}
    <h2>{t("contact.merges")}</h2>
    <p class="muted">{t("contact.mergesHint")}</p>
    {#if merges.length > 0}
      <ul data-testid="merges">
        {#each merges as entry (entry.merged_pid)}
          <li data-testid="merged-entry">
            {entry.merged_name} · {t("contact.mergedOn")}
            {new Date(entry.merged_at).toLocaleDateString(i18n.locale)}
            {#if entry.undoable}
              <button
                onclick={() => void startUndo(entry)}
                disabled={undoing !== null}
                data-testid="unmerge"
              >
                {t("contact.unmerge")}
              </button>
            {:else}
              <span class="muted small">{t("contact.unmergeNot")}</span>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
    {#if undoneName}
      <p role="status" data-testid="unmerge-done">{undoneName}: {t("contact.unmergeDone")}</p>
    {/if}
    {#if undoing}
      <section data-testid="unmerge-panel" aria-labelledby="unmerge-heading">
        <h3 id="unmerge-heading">{t("contact.unmerge")}: {undoing.merged_name}</h3>
        {#if undoPreview}
          <p data-testid="unmerge-restores">
            {t("contact.unmergeRestores")}:
            {counted(undoPreview.restored) || t("contact.mergeNothing")}.
          </p>
          {#if counted(undoPreview.left_behind)}
            <p data-testid="unmerge-left">
              {t("contact.unmergeLeft")}: {counted(undoPreview.left_behind)}.
            </p>
          {/if}
          {#if counted(undoPreview.dropped_restored)}
            <p data-testid="unmerge-drops">
              {t("contact.unmergeRestoredDrops")}: {counted(undoPreview.dropped_restored)}.
            </p>
          {/if}
          {#if undoPreview.fields_cleared.length > 0}
            <p data-testid="unmerge-cleared">
              {t("contact.unmergeCleared")}: {undoPreview.fields_cleared.join(", ")}.
            </p>
          {/if}
          <p data-testid="unmerge-consent">
            {t("contact.unmergeConsent")}: <strong>{undoPreview.consent.merged_result}</strong>
          </p>
          <p class="muted">{t("contact.unmergeWarn")}</p>
          <label>
            {t("contact.mergeReason")}
            <input bind:value={undoReason} data-testid="unmerge-reason" />
          </label>
          <button
            onclick={() => void confirmUndo()}
            disabled={undoBusy || !undoReason.trim()}
            data-testid="unmerge-confirm"
          >
            {t("contact.unmergeConfirm")}
          </button>
        {/if}
        <button onclick={cancelUndo} data-testid="unmerge-cancel">{t("contact.mergeCancel")}</button>
        {#if undoError}<p role="alert" data-testid="unmerge-error">{undoError}</p>{/if}
      </section>
    {/if}
  {/if}

  <h2>{t("contact.links")}</h2>
  <p class="muted">{t("contact.linksHint")}</p>
  {#if links !== null}
    {#if links.length === 0}
      <p data-testid="links-none">{t("contact.linksNone")}</p>
    {:else}
      <ul data-testid="links">
        {#each links as link (link.pid)}
          <li data-testid="link">
            <span class="chip">{systemName(link.system)}</span>
            <a
              href={link.url}
              target="_blank"
              rel="noopener noreferrer"
              aria-label={`${link.label ?? new URL(link.url).hostname} (${t("contact.linkOpens")})`}
              data-testid="link-open">{link.label ?? new URL(link.url).hostname}</a
            >
            <button onclick={() => void dropLink(link)} data-testid="link-remove">
              {t("contact.linkRemove")}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
  <form
    onsubmit={(event) => {
      event.preventDefault();
      void addLink();
    }}
    data-testid="link-form"
  >
    <label>
      {t("contact.linkSystem")}
      <select bind:value={linkSystem} data-testid="link-system">
        {#each LINK_SYSTEMS as system (system)}
          <option value={system}>{systemName(system)}</option>
        {/each}
      </select>
    </label>
    <label>
      {t("contact.linkUrl")}
      <input type="url" bind:value={linkUrl} required data-testid="link-url" />
    </label>
    <label>
      {t("contact.linkLabel")}
      <input bind:value={linkLabel} data-testid="link-label" />
    </label>
    <button type="submit" disabled={linkBusy || !linkUrl.trim()} data-testid="link-add">
      {t("contact.linkAdd")}
    </button>
    {#if linkError}<p role="alert" data-testid="link-error">{linkError}</p>{/if}
  </form>

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
