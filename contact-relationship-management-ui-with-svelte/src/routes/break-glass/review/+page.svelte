<script lang="ts">
  import { listBreakGlass, reviewBreakGlass } from "#lib/api/crm.js";
  import type { BreakGlassGrant } from "#lib/api/crm.js";
  import { i18n, t } from "#lib/i18n.svelte.js";

  const MIN_NOTE = 10;

  let grants = $state<BreakGlassGrant[] | null>(null);
  let unreviewed = $state(0);
  let error = $state<string | null>(null);
  let actionError = $state<string | null>(null);
  let notes = $state<Record<string, string>>({});

  async function load() {
    try {
      const data = await listBreakGlass();
      grants = data.grants;
      unreviewed = data.unreviewed;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  $effect(() => {
    void load();
  });

  async function review(grant: BreakGlassGrant) {
    actionError = null;
    try {
      await reviewBreakGlass(grant.pid, (notes[grant.pid] ?? "").trim());
      await load();
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  function when(value: string): string {
    return new Date(value).toLocaleString(i18n.locale);
  }
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
  <table data-testid="grants">
    <thead>
      <tr>
        <th>{t("breakGlass.subject")}</th>
        <th>{t("breakGlass.requestedAt")}</th>
        <th>{t("breakGlass.reason")}</th>
        <th>{t("common.actions")}</th>
      </tr>
    </thead>
    <tbody>
      {#each grants as grant (grant.pid)}
        <tr>
          <td>{grant.subject}</td>
          <td>{when(grant.granted_at)}</td>
          <td>{grant.reason}</td>
          <td>
            {#if grant.reviewed_at}
              {t("breakGlass.reviewedBy")} {grant.reviewed_by}: {grant.review_note}
            {:else}
              <label class="visually-hidden" for={`note-${grant.pid}`}>{t("breakGlass.note")}</label>
              <input
                id={`note-${grant.pid}`}
                placeholder={t("breakGlass.note")}
                bind:value={notes[grant.pid]}
              />
              <button
                type="button"
                disabled={(notes[grant.pid] ?? "").trim().length < MIN_NOTE}
                onclick={() => void review(grant)}
              >
                {t("breakGlass.recordReview")}
              </button>
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
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
