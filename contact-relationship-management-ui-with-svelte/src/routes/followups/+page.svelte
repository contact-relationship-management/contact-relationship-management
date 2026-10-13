<!--
  Follow-ups (`/followups`): the overdue list (server-derived aging)
  and the next 30 days on a SVAR Calendar. The server's note is shown
  verbatim (actor_ref is the recorder, not necessarily an assignee).
-->
<script lang="ts">
  import DataTable from "#lib/components/DataTable.svelte";
  import { Calendar, Willow } from "@svar-ui/svelte-calendar";
  import { followups, type Followup } from "#lib/api/crm.js";
  import { t } from "#lib/i18n.svelte.js";

  let overdue = $state<Followup[] | null>(null);
  let upcoming = $state<Followup[]>([]);
  let note = $state("");
  let kindFilter = $state("");
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        const view = await followups(kindFilter || undefined);
        overdue = view.overdue;
        upcoming = view.upcoming_30d;
        note = view.note;
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });

  // `@svar-ui/calendar-store` requires an all-day event's `end` to be
  // strictly *after* `start` (`!(e.end > start)` silently drops the
  // event — confirmed against the compiled library source). `end: day`
  // — the same Date as `start` — therefore filtered out every
  // upcoming follow-up this calendar was ever asked to show (the same
  // bug and fix as worker-front-end's/person-front-end's `/expiry`
  // and ppm-front-end's `/calendar`). Fixed with the following
  // calendar day, the minimum exclusive span a one-day all-day event
  // needs.
  const events = $derived(
    upcoming.map((item) => {
      const day = new Date(`${item.due_on}T00:00:00`);
      const nextDay = new Date(
        day.getFullYear(),
        day.getMonth(),
        day.getDate() + 1,
      );
      return {
        id: item.pid,
        start: day,
        end: nextDay,
        allDay: true,
        text: `${item.kind}: ${item.summary}`,
      };
    }),
  );

  const overdueColumns = [
    { id: "due", header: "Due" },
    { id: "days", kind: "number", header: "Overdue (days)" },
    { id: "kind", kind: "chip", header: "Kind" },
    { id: "summary", header: "Summary" },
    { id: "recorder", header: "Recorder" },
  ];
  const overdueRows = $derived(
    (overdue ?? []).map((i) => ({
      id: i.pid,
      due: i.due_on,
      days: i.overdue_days ?? "",
      kind: i.kind,
      summary: i.summary,
      recorder: i.actor_ref ?? "—",
    })),
  );
</script>

<svelte:head><title>{t("nav.followups")} — CRM</title></svelte:head>

<h1>{t("nav.followups")}</h1>
{#if error}<p class="error" data-testid="error">{error}</p>{/if}
{#if note}<p class="muted">{note}</p>{/if}

<p>
  <label>
    <select
      data-testid="kind-filter"
      value={kindFilter}
      onchange={(event) => {
        kindFilter = event.currentTarget.value;
      }}
    >
      <option value="">(all kinds)</option>
      {#each ["call", "email", "meeting", "note", "task"] as kind (kind)}
        <option value={kind}>{kind}</option>
      {/each}
    </select>
    <span class="muted">renewals convention: due-dated task activities</span>
  </label>
</p>

{#if overdue !== null}
  <h2>Overdue</h2>
  <DataTable
    label="Overdue follow-ups"
    columns={overdueColumns}
    rows={overdueRows}
    testid="followups-overdue"
    empty="Nothing overdue."
  />

  <h2>Next 30 days</h2>
  <div class="calendar-wrap" data-testid="followups-calendar">
    <Willow>
      <Calendar {events} view="month" readonly />
    </Willow>
  </div>
{/if}

<style>
  .calendar-wrap {
    height: 560px;
    overflow-x: auto;
  }
</style>
