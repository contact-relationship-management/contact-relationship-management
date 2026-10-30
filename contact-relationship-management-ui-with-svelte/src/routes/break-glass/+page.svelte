<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import { requestBreakGlass } from "#lib/api/crm.js";
  import { t } from "#lib/i18n.svelte.js";
  import { formatDateTime } from "#lib/locale-format.js";
  import { createIntent } from "#lib/intent.js";

  const MIN_REASON = 20;

  let reason = $state("");
  let minutes = $state(30);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let until = $state<string | null>(null);
  // One key across retries of the same request (CRM-T52).
  const intent = createIntent();

  const reasonLongEnough = $derived(reason.trim().length >= MIN_REASON);

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    error = null;
    busy = true;
    try {
      const content = { reason: reason.trim(), minutes };
      const grant = await requestBreakGlass(
        content.reason,
        content.minutes,
        intent.keyFor(content),
      );
      intent.done();
      until = formatDateTime(grant.expires_at);
      reason = "";
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head><title>{t("breakGlass.title")}</title></svelte:head>

<h1>{t("breakGlass.title")}</h1>
<p>{t("breakGlass.intro")}</p>

<form class="panel" onsubmit={submit}>
  <p>
    <label for="bg-reason">{t("breakGlass.reason")}</label><br />
    <textarea id="bg-reason" rows="3" bind:value={reason} required minlength={MIN_REASON} maxlength="500"
    ></textarea>
  </p>
  <p>
    <label for="bg-minutes">{t("breakGlass.minutes")}</label>
    <input id="bg-minutes" type="number" min="1" max="60" bind:value={minutes} required />
  </p>
  <button type="submit" disabled={busy || !reasonLongEnough} data-testid="bg-request">
    {t("breakGlass.request")}
  </button>
  {#if error}
    <p class="error" role="alert" data-testid="bg-error">{error}</p>
  {/if}
  {#if until}
    <p role="status" data-testid="bg-granted">{t("breakGlass.granted")} <strong>{until}</strong></p>
  {/if}
</form>

<p><a href="/break-glass/review">{t("breakGlass.reviewTitle")}</a></p>
