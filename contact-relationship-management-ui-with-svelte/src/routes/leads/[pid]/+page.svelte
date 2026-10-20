<script lang="ts">
  import { page } from "$app/state";
  import { getLead } from "#lib/api/crm.js";
  import LinksPanel from "#lib/components/LinksPanel.svelte";
  import { t } from "#lib/i18n.svelte.js";

  let detail = $state<Awaited<ReturnType<typeof getLead>> | null>(null);
  let error = $state<string | null>(null);
  const pid = $derived(page.params.pid ?? "");

  $effect(() => {
    const id = pid;
    void (async () => {
      try {
        detail = await getLead(id);
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });
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
