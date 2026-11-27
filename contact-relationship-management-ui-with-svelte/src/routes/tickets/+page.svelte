<script lang="ts">
  import ViewBar from "#lib/components/ViewBar.svelte";
  import { applyFilter, applyShape } from "#lib/views.js";
  import type { SavedView } from "#lib/api/crm.js";
  import { errorMessage } from "#lib/errors.js";
  import { resource } from "#lib/resource.svelte.js";
  import {
    assignTicket,
    listTickets,
    ticketCustomerReply,
    ticketReference,
    ticketStatus,
    ticketMovements,
    ticketSuggestions,
    stuckTickets,
  } from "#lib/api/crm.js";
  import { breachedCount, slaBreached } from "#lib/format.js";
  import DataTable from "#lib/components/DataTable.svelte";
  import { t } from "#lib/i18n.svelte.js";
  import { arrow, formatDateTime } from "#lib/locale-format.js";

  /** The forward move(s) a ticket offers per status. */
  const NEXT: Record<string, string[]> = {
    open: ["pending", "resolved"],
    pending: ["open", "resolved"],
    resolved: ["closed", "open"],
    closed: [],
  };

  const list = resource(listTickets);
  const tickets = $derived(list.value);
  const error = $derived(list.error);
  let selectedPid = $state<string | null>(null);
  let actionError = $state<string | null>(null);

  const suggestions = resource(() => (selectedPid ? ticketSuggestions(selectedPid) : Promise.resolve([])));
  const suggested = $derived(suggestions.value ?? []);

  const history = resource(() => (selectedPid ? ticketMovements(selectedPid) : Promise.resolve([])));
  const movements = $derived(history.value ?? []);
  let handingOver = $state(false);
  let handTo = $state("");
  let handReason = $state("");
  let handBusy = $state(false);

  // Open and pending tickets nothing has touched for a while (CRM-T317): tickets, never people.
  let stuckDays = $state(7);
  const stuck = resource(() => stuckTickets(stuckDays));
  const stuckColumns = $derived([
    { id: "title", header: t("common.name") },
    { id: "status", kind: "chip", header: t("common.status") },
    { id: "idle", kind: "number", header: t("ticket.idleDays") },
    { id: "touched", header: t("ticket.lastTouched") },
  ]);
  const stuckRows = $derived(
    (stuck.value?.tickets ?? []).map((s) => ({
      id: s.pid,
      title: s.title,
      status: s.status,
      idle: s.idle_days,
      touched: formatDateTime(s.last_touched_at),
    })),
  );

  const breachedTotal = $derived(breachedCount(tickets ?? []));
  const selected = $derived((tickets ?? []).find((x) => x.pid === selectedPid) ?? null);

  const columns = $derived([
    { id: "title", header: t("common.name") },
    { id: "priority", kind: "chip", header: t("ticket.priority") },
    { id: "status", kind: "chip", header: t("common.status") },
    { id: "channel", header: t("rpt.field.channel") },
    { id: "due", header: t("ticket.due") },
    { id: "breach", kind: "flag", header: t("ticket.breached") },
  ]);

  let view = $state<SavedView | null>(null);
  const rows = $derived(
    applyFilter(tickets ?? [], view).map((x) => ({
      id: x.pid,
      title: x.title,
      priority: x.priority,
      status: x.status,
      channel: x.channel ?? "",
      due: x.first_response_due_at ? formatDateTime(x.first_response_due_at) : "",
      breach: [
        slaBreached(x) ? t("ticket.breached") : x.paused_at ? t("ticket.paused") : "",
        x.wait_limit_exceeded_at ? t("ticket.waitExceeded") : "",
      ]
        .filter(Boolean)
        .join(" · "),
    })),
  );

  const shaped = $derived(applyShape(rows, columns, view));
  const headings = $derived(Object.fromEntries(columns.map((c) => [c.id, c.header])));

  async function replied(pid: string) {
    actionError = null;
    try {
      await ticketCustomerReply(pid);
      await list.reload();
    } catch (cause) {
      actionError = errorMessage(cause);
    }
  }

  /** Hand the ticket over: blank `handTo` leaves it with no one (CRM-T317). */
  async function handOver(pid: string) {
    actionError = null;
    handBusy = true;
    try {
      await assignTicket(pid, handTo.trim() || null, handReason.trim());
      handingOver = false;
      handTo = "";
      handReason = "";
      await Promise.all([list.reload(), history.reload()]);
    } catch (cause) {
      actionError = errorMessage(cause);
    } finally {
      handBusy = false;
    }
  }

  async function move(pid: string, to: string) {
    actionError = null;
    try {
      await ticketStatus(pid, to);
      await list.reload();
    } catch (cause) {
      actionError = errorMessage(cause);
    }
  }
</script>

<h1>{t("nav.tickets")}</h1>
<p><a class="button" href="/tickets/board">{t("common.board")}</a></p>
<ViewBar kind="ticket" bind:active={view} labels={headings} />

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if tickets === null}
  <p>{t("common.loading")}</p>
{:else}
  {#if breachedTotal > 0}
    <p><strong class="breach" data-testid="breached">{t("ticket.breached")}: {breachedTotal}</strong></p>
  {/if}
  <DataTable
    label={t("nav.tickets")}
    columns={shaped.columns}
    rows={shaped.rows}
    onselect={(pid) => (selectedPid = pid)}
    testid="ticket-queue"
  />
  {#if selected}
    <div class="panel">
      <strong>{selected.title}</strong>
      {#each NEXT[selected.status] ?? [] as to (to)}
        <button onclick={() => void move(selected.pid, to)}>{arrow()} {to}</button>
      {/each}
      {#if selected.status === "pending"}
        <button onclick={() => void replied(selected.pid)} data-testid="ticket-replied">{t("ticket.replied")}</button>
      {/if}
      <p class="muted small" data-testid="ticket-reference">
        {t("ticket.reference")}: <code>[{ticketReference(selected.pid)}]</code>
      </p>
      <p class="small" data-testid="ticket-assignee">
        {t("ticket.assignee")}: {selected.assignee_ref ?? t("kanban.unassigned")}
      </p>
      {#if handingOver}
        <form
          onsubmit={(event) => {
            event.preventDefault();
            if (selected) void handOver(selected.pid);
          }}
          data-testid="ticket-handover-form"
        >
          <label>
            {t("ticket.handOverTo")}
            <input bind:value={handTo} data-testid="ticket-handover-to" />
          </label>
          <label>
            {t("ticket.handOverReason")}
            <input bind:value={handReason} required data-testid="ticket-handover-reason" />
          </label>
          <button type="submit" disabled={handBusy || !handReason.trim()} data-testid="ticket-handover-confirm">
            {t("ticket.handOver")}
          </button>
          <button type="button" onclick={() => (handingOver = false)}>{t("deal.cancel")}</button>
        </form>
      {:else if selected.status !== "closed"}
        <button onclick={() => (handingOver = true)} data-testid="ticket-handover">{t("ticket.handOver")}</button>
      {/if}
      {#if actionError}
        <p class="error" data-testid="action-error">{actionError}</p>
      {/if}
      {#if movements.length > 0}
        <h2>{t("ticket.history")}</h2>
        <ol data-testid="ticket-movements">
          {#each movements as m (m.created_at + (m.to_ref ?? ""))}
            <li>
              {m.from_ref ?? t("kanban.unassigned")} {arrow()} {m.to_ref ?? t("kanban.unassigned")}:
              {m.reason}
              <span class="muted small">{formatDateTime(m.created_at)}</span>
            </li>
          {/each}
        </ol>
      {/if}
      {#if suggested.length > 0}
        <h2>{t("ticket.suggested")}</h2>
        <ul data-testid="ticket-suggestions">
          {#each suggested as s (s.article_pid)}
            <li>
              <a href="/articles">{s.title}</a>
              <span class="muted">{s.matched.join(", ")}</span>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
{/if}

<h2>{t("ticket.stuck")}</h2>
<label>
  {t("ticket.stuckDays")}
  <input
    type="number"
    min="1"
    max="365"
    value={stuckDays}
    onchange={(event) => {
      const n = Number(event.currentTarget.value);
      if (Number.isInteger(n) && n >= 1 && n <= 365) stuckDays = n;
    }}
    data-testid="stuck-days"
  />
</label>
{#if stuck.error}
  <p class="error">{t("common.error")}: {stuck.error}</p>
{:else if stuck.value}
  <p class="muted small">{stuck.value.derivation}</p>
  <DataTable
    label={t("ticket.stuck")}
    columns={stuckColumns}
    rows={stuckRows}
    testid="stuck-tickets"
    empty={t("ticket.noneStuck")}
  />
{/if}


<style>
  .breach {
    color: inherit;
  }
</style>
