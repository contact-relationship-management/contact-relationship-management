<script lang="ts">
  import { resource } from "#lib/resource.svelte.js";
  import { listAccounts } from "#lib/api/crm.js";
  import DataTable from "#lib/components/DataTable.svelte";
  import { t } from "#lib/i18n.svelte.js";
  import type { Account } from "#lib/api/crm.js";
  import ViewBar from "#lib/components/ViewBar.svelte";
  import { applyFilter, applyShape } from "#lib/views.js";
  import type { SavedView } from "#lib/api/crm.js";

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

  let view = $state<SavedView | null>(null);
  const rows = $derived(
    applyFilter(accounts ?? [], view).map((a) => ({
      id: a.pid,
      display_name: a.display_name,
      tier: a.tier,
      industry: a.industry ?? "",
      pid: a.pid,
    })),
  );

  const shaped = $derived(applyShape(rows, columns, view));
  const headings = $derived(Object.fromEntries(columns.map((c) => [c.id, c.header])));
</script>

<h1>{t("nav.accounts")}</h1>
<p><a class="button" href="/accounts/import" data-testid="import-link">{t("import.accountsTitle")}</a></p>
<ViewBar kind="account" bind:active={view} labels={headings} />

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if accounts === null}
  <p>{t("common.loading")}</p>
{:else}
  <DataTable
    label={t("nav.accounts")}
    columns={shaped.columns}
    rows={shaped.rows}
    testid="account-table"
  />
{/if}

