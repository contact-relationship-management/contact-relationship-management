<script lang="ts">
  import { resource } from "#lib/resource.svelte.js";
  import { page } from "$app/state";
  import { getAccount, money } from "#lib/api/crm.js";
  import LinksPanel from "#lib/components/LinksPanel.svelte";
  import { i18n, t } from "#lib/i18n.svelte.js";

  const pid = $derived(page.params.pid ?? "");
  const record = resource(() => getAccount(pid));
  const detail = $derived(record.value);
  const error = $derived(record.error);
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
  <LinksPanel kind="accounts" {pid} />
{/if}
