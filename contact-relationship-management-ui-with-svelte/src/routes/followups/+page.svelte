<!--
  Follow-ups (`/followups`): the overdue list (server-derived aging)
  and the next 30 days on the Lily CalendarView. The server's note is shown
  verbatim (actor_ref is the recorder, not necessarily an assignee).
-->
<script lang="ts">
  import DataTable from "#lib/components/DataTable.svelte";
  import CalendarView, {
    type CalendarViewMode,
  } from "@lilydesignsystem/svelte-calendar";
  import { followups, type Followup } from "#lib/api/crm.js";
  import { i18n, t } from "#lib/i18n.svelte.js";

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

  // Lily's CalendarView takes ISO dates; `date` places a one-day event.
  const events = $derived(
    upcoming.map((item) => ({
      id: item.pid,
      date: item.due_on,
      title: `${item.kind}: ${item.summary}`,
    })),
  );

  const today = new Date().toISOString().slice(0, 10);
  let view = $state<CalendarViewMode>("month");
  let anchorDate = $state(today);

  // Weekday and day names come from Intl in the UI language (UTC, so
  // the ISO date never shifts).
  const fmt = (options: Intl.DateTimeFormatOptions) =>
    new Intl.DateTimeFormat(i18n.locale, { timeZone: "UTC", ...options });
  // 2024-01-07 is a Sunday, so weekday 0 = Sunday.
  const weekdayDate = (wd: number) => new Date(Date.UTC(2024, 0, 7 + wd));
  const calendarLabels = $derived({
    weekView: t("cal.week"),
    fourWeekView: t("cal.fourWeeks"),
    monthView: t("cal.month"),
    previous: () => t("cal.previous"),
    next: () => t("cal.next"),
    today: t("cal.today"),
    periodAnnouncement: (_v: CalendarViewMode, start: string, end: string) =>
      t("cal.showing").replace("{start}", start).replace("{end}", end),
    weekday: (wd: number) => fmt({ weekday: "long" }).format(weekdayDate(wd)),
    weekdayAbbr: (wd: number) =>
      fmt({ weekday: "short" }).format(weekdayDate(wd)),
    day: (iso: string, _wd: number, outside: boolean) =>
      fmt({ dateStyle: "full" }).format(new Date(`${iso}T00:00:00Z`)) +
      (outside ? " ·" : ""),
  });

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
    <CalendarView
      label={t("cal.label")}
      bind:view
      bind:anchorDate
      {events}
      {today}
      locale={i18n.locale}
      labels={calendarLabels}
    >
      {#snippet day({ events: dayEvents })}
        {#each dayEvents as event (event.id)}
          <span class="calendar-event">{event.title}</span>
        {/each}
      {/snippet}
    </CalendarView>
  </div>
{/if}

<style>
  .calendar-wrap {
    overflow-x: auto;
  }
</style>
