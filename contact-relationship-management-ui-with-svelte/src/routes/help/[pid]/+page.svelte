<!--
  One help-centre article (`/help/{pid}`), open to anyone (CRM-R57). The text is plain and shown
  as text, never as markup. Under it a visitor can say whether it solved their problem; that is
  counted anonymously.
-->
<script lang="ts">
  import { page } from "$app/state";
  import { ApiError } from "#lib/api/client.js";
  import { errorMessage } from "#lib/errors.js";
  import { kbArticle, kbFeedback, type KbArticle } from "#lib/api/crm.js";
  import { t } from "#lib/i18n.svelte.js";

  let article = $state<KbArticle | null>(null);
  let missing = $state(false);
  let error = $state<string | null>(null);
  let voted = $state<boolean | null>(null);

  $effect(() => {
    const pid = page.params.pid ?? "";
    article = null;
    missing = false;
    voted = null;
    kbArticle(pid).then(
      (found) => (article = found),
      (cause) => {
        // Decided by the status the call failed with, never by the words of the error.
        if (cause instanceof ApiError && cause.status === 404) missing = true;
        else error = errorMessage(cause);
      },
    );
  });

  async function vote(helpful: boolean) {
    if (!article) return;
    try {
      await kbFeedback(article.pid, helpful);
      voted = helpful;
    } catch (cause) {
      error = errorMessage(cause);
    }
  }
</script>

<svelte:head><title>{article?.title ?? t("help.title")} · CRM</title></svelte:head>

<p><a href="/help">← {t("help.title")}</a></p>

{#if missing}
  <p data-testid="help-missing">{t("help.missing")}</p>
{:else if error}
  <p class="error" role="alert" data-testid="help-error">{error}</p>
{:else if article}
  <article data-testid="help-article">
    <h1>{article.title}</h1>
    <p class="body">{article.body}</p>
  </article>
  <div class="panel" data-testid="help-feedback">
    {#if voted === null}
      <span>{t("help.helpful")}</span>
      <button onclick={() => void vote(true)} data-testid="help-yes">{t("help.yes")}</button>
      <button onclick={() => void vote(false)} data-testid="help-no">{t("help.no")}</button>
    {:else}
      <span data-testid="help-thanks">{t("help.thanks")}</span>
    {/if}
  </div>
{:else}
  <p>{t("common.loading")}</p>
{/if}

<style>
  .body {
    white-space: pre-wrap;
    max-inline-size: 70ch;
  }
</style>
