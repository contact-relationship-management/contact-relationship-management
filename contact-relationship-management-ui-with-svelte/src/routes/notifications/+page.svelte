<!--
  Notifications (`/notifications`): the caller's inbox of task reminders and SLA
  breaches. The service leaves out records the caller may not read, and an entry
  carries no record text, so each one names why and links to the record.
-->
<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import { resource } from "#lib/resource.svelte.js";
  import {
    markAllNotificationsRead,
    markNotificationRead,
    notifications,
    type Notification,
  } from "#lib/api/crm.js";
  import { t } from "#lib/i18n.svelte.js";
  import { formatDateTime } from "#lib/locale-format.js";

  let unreadOnly = $state(false);
  const inbox = resource(() => notifications(unreadOnly), { keepWhileLoading: true });
  const items = $derived(inbox.value ?? null);
  const unread = $derived((items ?? []).filter((n) => n.read_at === null).length);
  let actionError = $state<string | null>(null);

  const kindLabel = (n: Notification) =>
    n.kind === "sla_breach" ? t("notif.slaBreach") : t("notif.taskReminder");
  // The record's own page, for the kinds that have one.
  const href = (n: Notification) => {
    const base: Record<string, string> = {
      contact: "contacts",
      account: "accounts",
      lead: "leads",
      deal: "deals",
      ticket: "tickets",
    };
    return base[n.subject_kind] ? `/${base[n.subject_kind]}/${n.subject_pid}` : null;
  };
  const when = (iso: string) => formatDateTime(iso);

  async function act(work: () => Promise<unknown>) {
    actionError = null;
    try {
      await work();
    } catch (cause) {
      actionError = errorMessage(cause);
    }
    await inbox.reload();
  }
</script>

<svelte:head><title>{t("nav.notifications")} — CRM</title></svelte:head>

<h1>{t("nav.notifications")}</h1>
{#if inbox.error}<p class="error" data-testid="error">{inbox.error}</p>{/if}
{#if actionError}<p class="error" role="alert" data-testid="action-error">{actionError}</p>{/if}

<p>
  <label>
    <input
      type="checkbox"
      data-testid="unread-only"
      checked={unreadOnly}
      onchange={(event) => {
        unreadOnly = event.currentTarget.checked;
      }}
    />
    {t("notif.unreadOnly")}
  </label>
  <button
    type="button"
    data-testid="mark-all"
    disabled={unread === 0}
    onclick={() => act(markAllNotificationsRead)}
  >
    {t("notif.markAll")}
  </button>
</p>

{#if items === null}
  <p>{t("common.loading")}</p>
{:else if items.length === 0}
  <p class="muted" data-testid="notifications-empty">{t("notif.empty")}</p>
{:else}
  <ul data-testid="notifications" class="notifications">
    {#each items as n (n.pid)}
      <li data-read={n.read_at !== null ? "" : undefined}>
        <strong>{kindLabel(n)}</strong>
        <time datetime={n.created_at}>{when(n.created_at)}</time>
        {#if href(n)}<a href={href(n)}>{t("notif.open")}</a>{/if}
        {#if n.read_at === null}
          <button type="button" onclick={() => act(() => markNotificationRead(n.pid))}>
            {t("notif.markRead")}
          </button>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  .notifications {
    list-style: none;
    padding: 0;
  }
  .notifications li {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    align-items: baseline;
    padding: 0.5rem 0;
    border-bottom: 1px solid var(--border, #ccc);
  }
  .notifications li[data-read] {
    opacity: 0.65;
  }
</style>
