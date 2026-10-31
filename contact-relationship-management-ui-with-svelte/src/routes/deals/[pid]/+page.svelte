<script lang="ts">
  import { ApiError } from "#lib/api/client.js";
  import { errorMessage } from "#lib/errors.js";
  import { resource } from "#lib/resource.svelte.js";
  import { page } from "$app/state";
  import {
    editDeal,
    listDeals,
    openDelivery,
    reopenDeal,
    type Deal,
    type ListedDeal,
    type DealDelivery,
  } from "#lib/api/crm.js";
  import { formMinor, minorToInput, money } from "#lib/money.js";
  import Attachments from "#lib/components/Attachments.svelte";
  import CustomFields from "#lib/components/CustomFields.svelte";
  import Tags from "#lib/components/Tags.svelte";
  import LinksPanel from "#lib/components/LinksPanel.svelte";
  import { i18n, t, tf } from "#lib/i18n.svelte.js";
  import { backArrow, formatDate } from "#lib/locale-format.js";

  const pid = $derived(page.params.pid ?? "");
  // There is no single-deal read; the list is already scoped by the caller's rights.
  const record = resource(async () => {
    const id = pid;
    const found = (await listDeals()).find((d) => d.pid === id);
    if (!found) throw new ApiError(404, null);
    return found;
  });
  const deal = $derived(record.value);
  const error = $derived(record.error);

  // Opening the delivery idea in the project portfolio service (CRM-R72).
  let delivery = $state<DealDelivery | null>(null);
  let deliveryError = $state<string | null>(null);

  async function openDeliveryIdea(current: Deal) {
    deliveryError = null;
    try {
      delivery = await openDelivery(current.pid);
    } catch (cause) {
      deliveryError = errorMessage(cause);
    }
  }

  let editing = $state(false);
  let reopening = $state(false);
  let reopened = $state(false);
  let reason = $state("");
  let busy = $state(false);
  let saved = $state(false);
  let formError = $state<string | null>(null);
  let name = $state("");
  // Typed in the page's own number format ("1234,50" in German), in the deal's own currency.
  let amount = $state("");
  let closeOn = $state("");

  function startEdit(current: Deal) {
    name = current.name;
    // An amount the caller's policy masks arrives null and stays uneditable.
    amount =
      current.amount_minor === null
        ? ""
        : minorToInput(current.amount_minor, current.currency, i18n.locale);
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
  async function save(current: ListedDeal) {
    const fields: { name?: string; amount_minor?: number; expected_close_on?: string } = {};
    if (name.trim() !== current.name) fields.name = name.trim();
    if (current.amount_minor !== null && amount.trim() !== "") {
      // The deal's own minor unit (CRM-T167): 1500 yen is 1500, not 150000.
      let minor: number;
      try {
        minor = formMinor(amount, current.currency, i18n.locale).minor;
      } catch (cause) {
        formError = errorMessage(cause);
        return;
      }
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
      // The edit answers the raw row; an edit is not a touch (CRM-D20), so the list's stalled
      // flag and last touch stay as they were.
      record.value = { ...current, ...(await editDeal(current.pid, fields)) };
      editing = false;
      saved = true;
    } catch (cause) {
      formError = errorMessage(cause);
    } finally {
      busy = false;
    }
  }
</script>

<p><a href="/deals"><span aria-hidden="true">{backArrow()}</span> {t("nav.deals")}</a></p>
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
  {#if deal.stalled}
    <!-- Derived by the service (CRM-R4, CRM-D20); the page only says it. -->
    <p data-testid="deal-stalled">
      <strong class="chip">{t("deal.stalled")}</strong>
      {tf("deal.stalledSince", { date: formatDate(deal.last_touched_at) })}
    </p>
  {/if}
  {#if reopened}<p role="status" data-testid="deal-reopened">{t("deal.reopened")}</p>{/if}
  {#if saved}<p role="status" data-testid="deal-saved">{t("deal.saved")}</p>{/if}
  {#if deal.won}
    <section class="panel" data-testid="deal-delivery">
      <button type="button" onclick={() => deal && void openDeliveryIdea(deal)} data-testid="deal-delivery-open">
        {t("deal.delivery")}
      </button>
      {#if delivery}
        <p role="status" data-testid="deal-delivery-result">
          {#if delivery.already_opened}
            {t("deal.deliveryAlready")}
          {:else if delivery.sent}
            {t("deal.deliverySent")}
          {:else}
            {t("deal.deliveryStub")}
          {/if}
          <span class="muted">{delivery.idea.title}</span>
        </p>
      {/if}
      {#if deliveryError}<p class="error" role="alert" data-testid="deal-delivery-error">{deliveryError}</p>{/if}
    </section>
  {/if}
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
          inputmode="decimal"
          bind:value={amount}
          disabled={deal.amount_minor === null}
          data-testid="deal-amount"
        />
        <span class="muted">{deal.currency}</span>
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
