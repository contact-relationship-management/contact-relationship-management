<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import DataTable from "#lib/components/DataTable.svelte";
  import { listBreakGlass, reviewBreakGlass } from "#lib/api/crm.js";
  import type { BreakGlassGrant } from "#lib/api/crm.js";
  import { i18n, t } from "#lib/i18n.svelte.js";
  import { createIntents } from "#lib/intent.js";

  const MIN_NOTE = 10;

  let grants = $state<BreakGlassGrant[] | null>(null);
  let unreviewed = $state(0);
  let error = $state<string | null>(null);
  let actionError = $state<string | null>(null);
  let notes = $state<Record<string, string>>({});
  // One key per grant across retries of its review (CRM-T52).
  const intentFor = createIntents();

  async function load() {
    try {
      const data = await listBreakGlass();
      grants = data.grants;
      unreviewed = data.unreviewed;
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  $effect(() => {
    void load();
  });

  async function review(grant: BreakGlassGrant) {
    actionError = null;
    try {
      const note = (notes[grant.pid] ?? "").trim();
      const intent = intentFor(grant.pid);
      await reviewBreakGlass(grant.pid, note, intent.keyFor({ note }));
      intent.done();
      await load();
    } catch (cause) {
      actionError = errorMessage(cause);
    }
  }

  function when(value: string): string {
    return new Date(value).toLocaleString(i18n.locale);
  }

  let selectedPid = $state<string | null>(null);
  const selected = $derived(
    (grants ?? []).find((g) => g.pid === selectedPid) ?? null,
  );

  const columns = $derived([
    { id: "subject", header: t("breakGlass.subject") },
    { id: "when", header: t("breakGlass.requestedAt") },
    { id: "reason", header: t("breakGlass.reason") },
    { id: "review", header: t("common.actions") },
  ]);
  const rows = $derived(
    (grants ?? []).map((g) => ({
      id: g.pid,
      subject: g.subject,
      when: when(g.granted_at),
      reason: g.reason,
      review: g.reviewed_at
        ? `${t("breakGlass.reviewedBy")} ${g.reviewed_by}: ${g.review_note}`
        : "",
    })),
  );
</script>

<svelte:head><title>{t("breakGlass.reviewTitle")}</title></svelte:head>

<h1>{t("breakGlass.reviewTitle")}</h1>

{#if error}
  <p class="error" role="alert" data-testid="error">{t("common.error")}: {error}</p>
{:else if grants === null}
  <p>{t("common.loading")}</p>
{:else}
  <p data-testid="unreviewed">{t("breakGlass.unreviewed")}: <strong>{unreviewed}</strong></p>
  {#if actionError}
    <p class="error" role="alert" data-testid="action-error">{actionError}</p>
  {/if}
  <DataTable
    label={t("breakGlass.reviewTitle")}
    {columns}
    {rows}
    onselect={(pid) => (selectedPid = pid)}
    testid="grants"
  />
  {#if selected && !selected.reviewed_at}
    <div class="panel" data-testid="grant-review">
      <strong>{selected.subject}</strong>
      <label class="visually-hidden" for={`note-${selected.pid}`}>{t("breakGlass.note")}</label>
      <input
        id={`note-${selected.pid}`}
        placeholder={t("breakGlass.note")}
        bind:value={notes[selected.pid]}
      />
      <button
        type="button"
        disabled={(notes[selected.pid] ?? "").trim().length < MIN_NOTE}
        onclick={() => void review(selected)}
      >
        {t("breakGlass.recordReview")}
      </button>
    </div>
  {/if}
{/if}

<style>
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
