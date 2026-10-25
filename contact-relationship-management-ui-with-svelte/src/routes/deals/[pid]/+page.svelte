<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import { resource } from "#lib/resource.svelte.js";
  import { page } from "$app/state";
  import { editDeal, listDeals, money, reopenDeal, type Deal } from "#lib/api/crm.js";
  import Attachments from "#lib/components/Attachments.svelte";
  import CustomFields from "#lib/components/CustomFields.svelte";
  import Tags from "#lib/components/Tags.svelte";
  import LinksPanel from "#lib/components/LinksPanel.svelte";
  import { i18n, t } from "#lib/i18n.svelte.js";

  const pid = $derived(page.params.pid ?? "");
  // There is no single-deal read; the list is already scoped by the caller's rights.
  const record = resource(async () => {
    const id = pid;
    const found = (await listDeals()).find((d) => d.pid === id);
    if (!found) throw new Error("404");
    return found;
  });
  const deal = $derived(record.value);
  const error = $derived(record.error);

  let editing = $state(false);
  let reopening = $state(false);
  let reopened = $state(false);
  let reason = $state("");
  let busy = $state(false);
  let saved = $state(false);
  let formError = $state<string | null>(null);
  let name = $state("");
  // A number input binds a number (or null when empty), so keep it loose.
  let amount = $state<string | number | null>("");
  let closeOn = $state("");

  function startEdit(current: Deal) {
    name = current.name;
    // An amount the caller's policy masks arrives null and stays uneditable.
    amount = current.amount_minor === null ? "" : String(current.amount_minor / 100);
    closeOn = current.expected_close_on ?? "";
    formError = null;
    saved = false;
    reopened = false;
    editing = true;
  }

  // Re-read the deal afterwards: the reopen response is the raw row, not the masked view.
  async function reopen(current: Deal) {
    busy = true;
    formError = null;
    try {
      await reopenDeal(current.pid, reason.trim());
      await record.reload();
      reopening = false;
      reason = "";
      saved = false;
      reopened = true;
    } catch (cause) {
      formError = errorMessage(cause);
    } finally {
      busy = false;
    }
  }

  // Only what changed is sent, so a masked amount is never overwritten with nothing.
  async function save(current: Deal) {
    const fields: { name?: string; amount_minor?: number; expected_close_on?: string } = {};
    if (name.trim() !== current.name) fields.name = name.trim();
    if (current.amount_minor !== null && String(amount ?? "").trim() !== "") {
      const minor = Math.round(Number(amount) * 100);
      if (minor !== current.amount_minor) fields.amount_minor = minor;
    }
    if (closeOn && closeOn !== (current.expected_close_on ?? "")) fields.expected_close_on = closeOn;
    if (Object.keys(fields).length === 0) {
      editing = false;
      return;
    }
    busy = true;
    formError = null;
    try {
      record.value = await editDeal(current.pid, fields);
      editing = false;
      saved = true;
    } catch (cause) {
      formError = errorMessage(cause);
    } finally {
      busy = false;
    }
  }
</script>

<p><a href="/deals">← {t("nav.deals")}</a></p>
{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if deal === null}
  <p>{t("common.loading")}</p>
{:else}
  <h1 data-testid="record-title">{deal.name}</h1>
  <p>
    {#if deal.amount_minor !== null}<strong>{money(deal.amount_minor, deal.currency, i18n.locale)}</strong>{/if}
    {#if deal.closed_at}<span class="chip">{deal.won ? "won" : "closed"}</span>{/if}
  </p>
  {#if reopened}<p role="status" data-testid="deal-reopened">{t("deal.reopened")}</p>{/if}
  {#if saved}<p role="status" data-testid="deal-saved">{t("deal.saved")}</p>{/if}
  {#if deal.closed_at}
    <p data-testid="deal-locked">{t("deal.closedLocked")}</p>
    {#if reopening}
      <form
        onsubmit={(event) => {
          event.preventDefault();
          if (deal) void reopen(deal);
        }}
        data-testid="deal-reopen-form"
      >
        <label>
          {t("deal.reopenReason")}
          <input bind:value={reason} required data-testid="deal-reopen-reason" />
        </label>
        <button type="submit" disabled={busy || !reason.trim()} data-testid="deal-reopen-confirm">
          {t("deal.reopenConfirm")}
        </button>
        <button type="button" onclick={() => (reopening = false)} data-testid="deal-reopen-cancel">
          {t("deal.cancel")}
        </button>
        {#if formError}<p role="alert" data-testid="deal-error">{formError}</p>{/if}
      </form>
    {:else}
      <button
        onclick={() => {
          reopening = true;
          formError = null;
        }}
        data-testid="deal-reopen"
      >
        {t("deal.reopen")}
      </button>
    {/if}
  {:else if editing}
    <form
      onsubmit={(event) => {
        event.preventDefault();
        if (deal) void save(deal);
      }}
      data-testid="deal-form"
    >
      <label>
        {t("common.name")}
        <input bind:value={name} required data-testid="deal-name" />
      </label>
      <label>
        {t("kanban.amount")}
        <input
          type="number"
          min="0"
          step="0.01"
          bind:value={amount}
          disabled={deal.amount_minor === null}
          data-testid="deal-amount"
        />
      </label>
      <label>
        {t("kanban.closeOn")}
        <input type="date" bind:value={closeOn} data-testid="deal-close-on" />
      </label>
      <button type="submit" disabled={busy || !name.trim()} data-testid="deal-save">
        {t("deal.save")}
      </button>
      <button type="button" onclick={() => (editing = false)} data-testid="deal-cancel">
        {t("deal.cancel")}
      </button>
      {#if formError}<p role="alert" data-testid="deal-error">{formError}</p>{/if}
    </form>
  {:else}
    <button onclick={() => deal && startEdit(deal)} data-testid="deal-edit">{t("deal.edit")}</button>
  {/if}
  <Tags kind="deals" {pid} />
  <CustomFields kind="deals" {pid} />
  <Attachments kind="deals" {pid} />
  <LinksPanel kind="deals" {pid} />
{/if}
