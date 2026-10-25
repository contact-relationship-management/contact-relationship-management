<script lang="ts">
  import { resource } from "#lib/resource.svelte.js";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { listContacts } from "#lib/api/crm.js";
  import DataTable from "#lib/components/DataTable.svelte";
  import TagFilter from "#lib/components/TagFilter.svelte";
  import { t } from "#lib/i18n.svelte.js";
  import type { Contact } from "#lib/api/crm.js";
  import ViewBar from "#lib/components/ViewBar.svelte";
  import { applyFilter, applyShape } from "#lib/views.js";
  import type { SavedView } from "#lib/api/crm.js";

  let tag = $state("");
  const list = resource(() => listContacts(undefined, tag || undefined));
  const contacts = $derived(list.value);
  const error = $derived(list.error);

  const columns = $derived([
    {
      id: "display_name",
      header: t("common.name"),
      href: (row: Record<string, unknown>) => `/contacts/${row.id}`,
    },
    { id: "job_title", header: t("contact.jobTitle") },
    { id: "status", kind: "chip", header: t("common.status") },
    { id: "marketing_consent", kind: "chip", header: t("contact.consent") },
  ]);

  let view = $state<SavedView | null>(null);
  const rows = $derived(
    applyFilter(contacts ?? [], view).map((c) => ({
      id: c.pid,
      display_name: c.display_name,
      job_title: c.job_title ?? "",
      status: c.status,
      marketing_consent: c.marketing_consent,
    })),
  );

  // The header search sends a bare query (`/contacts?ada`), or `?q=ada`.
  const query = $derived(
    (page.url.searchParams.get("q") ?? decodeURIComponent(page.url.search.slice(1)))
      .trim()
      .toLowerCase(),
  );
  const searched = $derived(
    query === ""
      ? rows
      : rows.filter((r) =>
          [r.display_name, r.job_title, r.status].some((v) =>
            v.toLowerCase().includes(query),
          ),
        ),
  );
  const shaped = $derived(applyShape(searched, columns, view));
  const headings = $derived(Object.fromEntries(columns.map((c) => [c.id, c.header])));
</script>

<h1>{t("nav.contacts")}</h1>
<p><TagFilter bind:value={tag} /></p>
<p><a class="button" href="/contacts/import" data-testid="import-link">{t("import.contactsTitle")}</a></p>
<ViewBar kind="contact" bind:active={view} labels={headings} />

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if contacts === null}
  <p>{t("common.loading")}</p>
{:else}
  <DataTable
    label={t("nav.contacts")}
    columns={shaped.columns}
    rows={shaped.rows}
    onselect={(pid) => void goto(`/contacts/${pid}`)}
    testid="contact-table"
  />
{/if}

