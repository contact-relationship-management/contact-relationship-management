<script lang="ts">
  import { resource } from "#lib/resource.svelte.js";
  import { listAccounts } from "#lib/api/crm.js";
  import DataTable from "#lib/components/DataTable.svelte";
  import { t } from "#lib/i18n.svelte.js";
  import type { Account } from "#lib/api/crm.js";

  const list = resource(listAccounts);
  const accounts = $derived(list.value);
  const error = $derived(list.error);

  const columns = $derived([
    {
      id: "display_name",
      header: t("common.name"),
      href: (row: Record<string, unknown>) => `/accounts/${row.id}`,
    },
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

