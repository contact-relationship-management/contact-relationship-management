<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { listContacts } from "#lib/api/crm.js";
  import DataTable from "#lib/components/DataTable.svelte";
  import { t } from "#lib/i18n.svelte.js";
  import type { Contact } from "#lib/api/crm.js";

  let contacts = $state<Contact[] | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        contacts = await listContacts();
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });

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

  const rows = $derived(
    (contacts ?? []).map((c) => ({
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
</script>

<h1>{t("nav.contacts")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if contacts === null}
  <p>{t("common.loading")}</p>
{:else}
  <DataTable
    label={t("nav.contacts")}
    {columns}
    rows={searched}
    onselect={(pid) => void goto(`/contacts/${pid}`)}
    testid="contact-table"
  />
{/if}

