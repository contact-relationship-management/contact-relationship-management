<script lang="ts">
  import { page } from "$app/state";
  import { listDeals, money, type Deal } from "#lib/api/crm.js";
  import LinksPanel from "#lib/components/LinksPanel.svelte";
  import { i18n, t } from "#lib/i18n.svelte.js";

  let deal = $state<Deal | null>(null);
  let error = $state<string | null>(null);
  const pid = $derived(page.params.pid ?? "");

  // There is no single-deal read; the list is already scoped by the caller's rights.
  $effect(() => {
    const id = pid;
    void (async () => {
      try {
        const found = (await listDeals()).find((d) => d.pid === id);
        if (found) deal = found;
        else error = "404";
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });
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
  <LinksPanel kind="deals" {pid} />
{/if}
