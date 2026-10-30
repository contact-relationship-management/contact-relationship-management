<!--
  Follow-ups (`/followups`): the overdue list (server-derived aging)
  and the next 30 days on the Lily CalendarView. The server's note is shown
  verbatim (actor_ref is the recorder, not necessarily an assignee).
-->
<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import { resource } from "#lib/resource.svelte.js";
  import { packLabels } from "#lib/lily-labels.js";
  import DataTable from "#lib/components/DataTable.svelte";
  import CalendarView, {
    type CalendarEvent,
    type CalendarViewMode,
  } from "@lilydesignsystem/svelte-calendar";
  import { editFollowup, followups } from "#lib/api/crm.js";
  import { i18n, t, tn } from "#lib/i18n.svelte.js";
  import { formatDate } from "#lib/locale-format.js";
  import { browserLanguages, calendarLocale, todayIso, weekInfo } from "#lib/reader-calendar.js";

  let kindFilter = $state("");
  const list = resource(() => followups(kindFilter || undefined));
  const overdue = $derived(list.value?.overdue ?? null);
  const upcoming = $derived(list.value?.upcoming_30d ?? []);
  const note = $derived(list.value?.note ?? "");
  const error = $derived(list.error);

  // Lily's CalendarView takes ISO dates; `date` places a one-day event.
  // Dragging an event to another day (or Alt+arrow on it) reschedules the
  // follow-up; the service audits it. A refusal puts the page back as stored.
  let moveError = $state<string | null>(null);
  async function onEventMove(calendarEvent: { id: string }, toDate: string) {
    moveError = null;
    try {
      await editFollowup(calendarEvent.id, { due_on: toDate });
    } catch (cause) {
      moveError = errorMessage(cause);
    }
    await list.reload();
  }

  const events = $derived(
    upcoming.map((item) => ({
      id: item.pid,
      date: item.due_on,
      title: `${item.kind}: ${item.summary}`,
    })),
  );

  // Today in the reader's own time zone, not UTC; the week's first day and weekend from CLDR for the
  // reader's region (CRM-T172, as WPM-T378). The Lily calendar reads the first day from the locale
  // tag, so it gets the page's locale with the CLDR `fw` keyword set; the weekend it takes as numbers.
  const today = todayIso();
  const week = $derived(weekInfo(i18n.locale, browserLanguages()));
  const calendarTag = $derived(calendarLocale(i18n.locale, week));
  let view = $state<CalendarViewMode>("month");
  let anchorDate = $state(today);

  // Weekday and day names come from Intl in the UI language (UTC, so
  // the ISO date never shifts).
  const fmt = (options: Intl.DateTimeFormatOptions) =>
    new Intl.DateTimeFormat(i18n.locale, { timeZone: "UTC", ...options });
  // 2024-01-07 is a Sunday, so weekday 0 = Sunday.
  const weekdayDate = (wd: number) => new Date(Date.UTC(2024, 0, 7 + wd));
  const calendarLabels = $derived({
    ...packLabels("calendar", i18n.locale),
    dayView: t("cal.day"),
    agendaView: t("cal.agenda"),
    agendaEmpty: t("cal.agendaEmpty"),
    weekNumberHeader: t("cal.weekNumberHeader"),
    export: t("cal.export"),
    exportCsv: "CSV",
    exportExcel: "Excel",
    exportHeaders: {
      title: t("cal.exportTitle"),
      start: t("cal.exportStart"),
      end: t("cal.exportEnd"),
    },
    eventMoveAnnouncement: (e: CalendarEvent, toDate: string) =>
      t("cal.moved").replace("{title}", String(e.title ?? "")).replace("{date}", formatDate(toDate)),
    weekView: t("cal.week"),
    fourWeekView: t("cal.fourWeeks"),
    monthView: t("cal.month"),
    previous: () => t("cal.previous"),
    next: () => t("cal.next"),
    today: t("cal.today"),
    periodAnnouncement: (_v: CalendarViewMode, start: string, end: string) =>
      t("cal.showing").replace("{start}", formatDate(start)).replace("{end}", formatDate(end)),
    weekday: (wd: number) => fmt({ weekday: "long" }).format(weekdayDate(wd)),
    weekdayAbbr: (wd: number) =>
      fmt({ weekday: "short" }).format(weekdayDate(wd)),
    // A cell shows its day number and that day's events, so its name starts with exactly those
    // words and then gives the full date: the visible text must be inside the accessible name,
    // as one run (WCAG 2.5.3).
    day: (iso: string, _wd: number, outside: boolean) => {
      const full = fmt({ dateStyle: "full" }).format(new Date(`${iso}T00:00:00Z`));
      const titles = events.filter((e) => e.date === iso).map((e) => e.title);
      const shown = titles.length ? `${Number(iso.slice(8))} ${titles.join(" ")}, ` : "";
      return shown + full + (outside ? " ·" : "");
    },
  });

  const overdueColumns = [
    { id: "due", kind: "date", header: "Due" },
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
{#if moveError}<p class="error" role="alert" data-testid="move-error">{moveError}</p>{/if}
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
      {onEventMove}
      {today}
      locale={calendarTag}
      labels={calendarLabels}
      showWeekNumbers
      weekendDays={week.weekend}
      maxEvents={3}
      exportFilename="follow-ups"
    >
      {#snippet day({ date, visibleEvents, isToday, hiddenCount, eventProps })}
        <span class="calendar-day-number" data-today={isToday ? "" : undefined}
          >{Number(date.slice(8))}</span
        >
        {#each visibleEvents as event (event.id)}
          <button type="button" class="calendar-event" {...eventProps(event)}>
            {event.title}
          </button>
        {/each}
        {#if hiddenCount > 0}
          <span class="calendar-event muted"
            >{tn("cal.more", hiddenCount, { count: hiddenCount })}</span
          >
        {/if}
      {/snippet}
    </CalendarView>
  </div>
{/if}

<style>
  .calendar-wrap {
    overflow-x: auto;
  }
</style>
