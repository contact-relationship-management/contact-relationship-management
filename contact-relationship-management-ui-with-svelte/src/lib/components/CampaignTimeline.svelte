<!--
  CampaignTimeline — a read-only Gantt chart of campaigns, on the Lily
  `@lilydesignsystem/svelte-gantt-chart`. A bar runs from the day a campaign was
  created to its last status change, or to today while it is still open
  (draft, scheduled, running). The service stores no planned send dates, so the
  chart shows what happened, not a plan. Strings come from the `timeline.*` keys.
-->
<script lang="ts">
  import GanttChart from "@lilydesignsystem/svelte-gantt-chart";
  import type { GanttTask } from "@lilydesignsystem/svelte-gantt-chart";
  import type { Campaign } from "#lib/api/crm.js";
  import { i18n, t } from "#lib/i18n.svelte.js";

  let { campaigns }: { campaigns: Campaign[] } = $props();

  const day = (iso: string) => iso.slice(0, 10);
  const today = new Date().toISOString().slice(0, 10);
  const addDays = (iso: string, n: number) =>
    new Date(Date.parse(`${iso}T00:00:00Z`) + n * 86_400_000)
      .toISOString()
      .slice(0, 10);

  const OPEN = ["draft", "scheduled", "running"];

  const dated = $derived(campaigns.filter((c) => c.created_at));
  const tasks = $derived<GanttTask[]>(
    dated.map((c) => {
      const start = day(c.created_at as string);
      const stop = OPEN.includes(c.status) ? today : day(c.updated_at ?? today);
      return {
        id: c.pid,
        label: `${c.name} (${c.status})`,
        start,
        end: stop < start ? start : stop,
        type: c.status,
      };
    }),
  );
  const range = $derived({
    start: tasks.reduce((min, x) => (x.start! < min ? x.start! : min), today),
    end: addDays(tasks.reduce((max, x) => (x.end! > max ? x.end! : max), today), 7),
  });

  const fmt = $derived(
    new Intl.DateTimeFormat(i18n.locale, {
      timeZone: "UTC",
      day: "numeric",
      month: "short",
    }),
  );
  const labels = $derived({
    taskColumnHeader: t("timeline.task"),
    // Short heading text; the column's full name says "week of …".
    columnLabel: (start: string) => fmt.format(new Date(`${start}T00:00:00Z`)),
    zoomIn: t("timeline.zoomIn"),
    zoomOut: t("timeline.zoomOut"),
  });
</script>

{#if tasks.length > 0}
  <h2>{t("timeline.title")}</h2>
  <p class="muted">{t("timeline.note")}</p>
  <div class="campaign-timeline" data-testid="campaign-timeline">
    <GanttChart
      label={t("timeline.title")}
      {range}
      {tasks}
      {today}
      timeUnit="week"
      readonly
      {labels}
      locale={i18n.locale}
    />
  </div>
{/if}
