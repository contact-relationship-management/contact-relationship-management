<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import { resource } from "#lib/resource.svelte.js";
  import { listTickets, ticketStatus } from "#lib/api/crm.js";
  import { breachedCount, slaBreached } from "#lib/format.js";
  import DataTable from "#lib/components/DataTable.svelte";
  import { t } from "#lib/i18n.svelte.js";
  import type { Ticket } from "#lib/api/crm.js";

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

  const breachedTotal = $derived(breachedCount(tickets ?? []));
  const selected = $derived((tickets ?? []).find((x) => x.pid === selectedPid) ?? null);

  const columns = $derived([
    { id: "title", header: t("common.name") },
    { id: "priority", kind: "chip", header: t("ticket.priority") },
    { id: "status", kind: "chip", header: t("common.status") },
    { id: "due", header: t("ticket.due") },
    { id: "breach", kind: "flag", header: t("ticket.breached") },
  ]);

  const rows = $derived(
    (tickets ?? []).map((x) => ({
      id: x.pid,
      title: x.title,
      priority: x.priority,
      status: x.status,
      due: x.first_response_due_at
        ? new Date(x.first_response_due_at).toLocaleString()
        : "",
      breach: slaBreached(x) ? t("ticket.breached") : "",
    })),
  );


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
    {columns}
    {rows}
    onselect={(pid) => (selectedPid = pid)}
    testid="ticket-queue"
  />
  {#if selected}
    <div class="panel">
      <strong>{selected.title}</strong>
      {#each NEXT[selected.status] ?? [] as to (to)}
        <button onclick={() => void move(selected.pid, to)}>→ {to}</button>
      {/each}
      {#if actionError}
        <p class="error" data-testid="action-error">{actionError}</p>
      {/if}
    </div>
  {/if}
{/if}


<style>
  .breach {
    color: inherit;
  }
</style>
