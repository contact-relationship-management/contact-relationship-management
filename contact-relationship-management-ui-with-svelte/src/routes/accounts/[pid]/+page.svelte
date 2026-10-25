<script lang="ts">
  import { resource } from "#lib/resource.svelte.js";
  import { page } from "$app/state";
  import { editAccount, getAccount, money } from "#lib/api/crm.js";
  import Attachments from "#lib/components/Attachments.svelte";
  import CustomFields from "#lib/components/CustomFields.svelte";
  import Tags from "#lib/components/Tags.svelte";
  import LinksPanel from "#lib/components/LinksPanel.svelte";
  import { i18n, t } from "#lib/i18n.svelte.js";
  import { errorMessage, fieldErrors } from "#lib/errors.js";

  const pid = $derived(page.params.pid ?? "");
  const record = resource(() => getAccount(pid));
  const detail = $derived(record.value);
  const error = $derived(record.error);

  // Editing the account's own details (CRM-R39). An emptied industry clears it.
  const TIERS = ["prospect", "customer", "partner", "former"];
  let editing = $state(false);
  let saved = $state(false);
  let editError = $state<string | null>(null);
  let problems = $state<Record<string, string>>({});
  let form = $state({ display_name: "", tier: "prospect", industry: "" });

  function startEdit() {
    const a = detail?.account;
    if (!a) return;
    form = { display_name: a.display_name, tier: a.tier, industry: a.industry ?? "" };
    saved = false;
    editError = null;
    editing = true;
  }

  async function saveEdit() {
    editError = null;
    problems = {};
    try {
      await editAccount(pid, {
        display_name: form.display_name,
        tier: form.tier,
        industry: form.industry.trim() ? form.industry.trim() : null,
      });
      editing = false;
      saved = true;
      await record.reload();
    } catch (cause) {
      problems = fieldErrors(cause);
      editError = errorMessage(cause);
    }
  }
</script>

<p><a href="/accounts">← {t("nav.accounts")}</a></p>
{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if detail === null}
  <p>{t("common.loading")}</p>
{:else}
  <h1 data-testid="record-title">{detail.account.display_name}</h1>
  <p>
    <span class="chip">{detail.account.tier}</span>
    {#if detail.account.industry}<span class="muted">{detail.account.industry}</span>{/if}
  </p>
  {#if editing}
    <form
      class="panel"
      onsubmit={(e) => {
        e.preventDefault();
        void saveEdit();
      }}
      data-testid="account-form"
    >
      <label>{t("common.name")} <input bind:value={form.display_name} aria-invalid={problems.display_name ? "true" : undefined} />{#if problems.display_name}<span class="error small" data-testid="problem-display_name">{problems.display_name}</span>{/if}</label>
      <label>
        {t("account.tier")}
        <select bind:value={form.tier}>
          {#each TIERS as tier (tier)}<option value={tier}>{tier}</option>{/each}
        </select>
      </label>
      <label>{t("account.industry")} <input bind:value={form.industry} aria-invalid={problems.industry ? "true" : undefined} />{#if problems.industry}<span class="error small" data-testid="problem-industry">{problems.industry}</span>{/if}</label>
      <button type="submit" data-testid="account-save">{t("contact.editSave")}</button>
      <button type="button" onclick={() => (editing = false)}>{t("contact.editCancel")}</button>
      {#if editError}<p class="error" data-testid="action-error">{editError}</p>{/if}
    </form>
  {:else}
    <button onclick={startEdit} data-testid="account-edit">{t("contact.edit")}</button>
    {#if saved}<span data-testid="account-saved">{t("contact.saved")}</span>{/if}
  {/if}
  {#if detail.contacts.length > 0}
    <h2>{t("nav.contacts")}</h2>
    <ul>
      {#each detail.contacts as contact (contact.pid)}
        <li><a href={`/contacts/${contact.pid}`}>{contact.display_name}</a></li>
      {/each}
    </ul>
  {/if}
  {#if detail.deals.length > 0}
    <h2>{t("nav.deals")}</h2>
    <ul>
      {#each detail.deals as deal (deal.pid)}
        <li>
          <a href={`/deals/${deal.pid}`}>{deal.name}</a>
          {#if deal.amount_minor !== null}{money(deal.amount_minor, deal.currency, i18n.locale)}{/if}
        </li>
      {/each}
    </ul>
  {/if}
  <Tags kind="accounts" {pid} />
  <CustomFields kind="accounts" {pid} />
  <Attachments kind="accounts" {pid} />
  <LinksPanel kind="accounts" {pid} />
{/if}
