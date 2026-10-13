<script lang="ts">
  import { listAccounts } from "#lib/api/crm.js";
  import DataTable from "#lib/components/DataTable.svelte";
  import { t } from "#lib/i18n.svelte.js";
  import type { Account } from "#lib/api/crm.js";

  let accounts = $state<Account[] | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        accounts = await listAccounts();
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });

  const columns = $derived([
    { id: "display_name", header: t("common.name") },
    { id: "tier", kind: "chip", header: t("account.tier") },
    { id: "industry", header: t("account.industry") },
    { id: "pid", header: "pid" },
  ]);

  const rows = $derived(
    (accounts ?? []).map((a) => ({
      id: a.pid,
      display_name: a.display_name,
      tier: a.tier,
      industry: a.industry ?? "",
      pid: a.pid,
    })),
  );

</script>

<h1>{t("nav.accounts")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if accounts === null}
  <p>{t("common.loading")}</p>
{:else}
  <DataTable label={t("nav.accounts")} {columns} {rows} testid="account-table" />
{/if}

