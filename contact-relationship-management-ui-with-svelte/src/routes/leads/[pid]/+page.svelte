<script lang="ts">
  import { resource } from "#lib/resource.svelte.js";
  import { page } from "$app/state";
  import { getLead } from "#lib/api/crm.js";
  import LinksPanel from "#lib/components/LinksPanel.svelte";
  import { t } from "#lib/i18n.svelte.js";

  const pid = $derived(page.params.pid ?? "");
  const record = resource(() => getLead(pid));
  const detail = $derived(record.value);
  const error = $derived(record.error);
</script>

<p><a href="/leads">← {t("nav.leads")}</a></p>
{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if detail === null}
  <p>{t("common.loading")}</p>
{:else}
  <h1 data-testid="record-title">{detail.lead.display_name}</h1>
  <p>
    <span class="chip">{detail.lead.status}</span>
    <span class="muted">{detail.lead.source}</span>
    · {detail.score.score} · {detail.score.label}
  </p>
  <LinksPanel kind="leads" {pid} />
{/if}
