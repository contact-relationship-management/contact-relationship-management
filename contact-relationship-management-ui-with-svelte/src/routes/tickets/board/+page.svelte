<!--
  Ticket board (`/tickets/board`): open / pending / resolved / closed
  as SVAR Kanban columns; drag = the status transition (the lifecycle
  machine owns legality; a refused move reloads to the stored truth).
  Cards badge priority and live SLA-breach flags.
-->
<script lang="ts">
  import Board from "#lib/components/Board.svelte";
  import type { KanbanCard } from "@lilydesignsystem/svelte-kanban-board";
  import { listTickets, ticketStatus, type Ticket } from "#lib/api/crm.js";
  import { slaBreached } from "#lib/format.js";
  import { t } from "#lib/i18n.svelte.js";

  let tickets = $state<Ticket[] | null>(null);
  let error = $state<string | null>(null);

  async function load() {
    try {
      tickets = await listTickets();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }
  $effect(() => {
    void load();
  });

  const columns = [
    { id: "open", title: "Open" },
    { id: "pending", title: "Pending" },
    { id: "resolved", title: "Resolved" },
    { id: "closed", title: "Closed" },
  ];

  // The service's ticket lifecycle (CRM-R10): only legal moves are offered.
  const NEXT: Record<string, string[]> = {
    open: ["pending", "resolved"],
    pending: ["open", "resolved"],
    resolved: ["closed", "open"],
    closed: [],
  };

  const cards = $derived<KanbanCard[]>(
    (tickets ?? []).map((ticket) => ({
      id: ticket.pid,
      columnId: ticket.status,
      title: ticket.title,
      subtitle: ticket.priority + (slaBreached(ticket) ? " · SLA breached" : ""),
      flag: slaBreached(ticket) ? "SLA breached" : undefined,
      locked: (NEXT[ticket.status] ?? []).length === 0,
    })),
  );
  const allowedColumns = (card: KanbanCard) => NEXT[card.columnId] ?? [];

  async function onMove(id: string, status: string) {
    await ticketStatus(id, status);
    await load();
  }
</script>

<svelte:head><title>{t("nav.tickets")} — CRM</title></svelte:head>

<h1>{t("nav.tickets")} {t("common.board")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if tickets === null}
  <p>{t("common.loading")}</p>
{:else}
  <Board
    label={`${t("nav.tickets")} ${t("common.board")}`}
    {columns}
    {cards}
    {allowedColumns}
    {onMove}
    testid="ticket-board"
  />
{/if}
