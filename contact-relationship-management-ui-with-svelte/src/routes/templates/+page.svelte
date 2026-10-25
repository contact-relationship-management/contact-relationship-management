<!--
  Email templates (`/templates`): write a message once with merge tokens, see it rendered for a
  made-up contact (never a real one), and save it. Every body must carry the unsubscribe link, and a
  token with no value falls back to a declared text (CRM-R36). The preview shows the plain text
  that would be mailed.
-->
<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import { resource } from "#lib/resource.svelte.js";
  import {
    deleteTemplate,
    listTemplates,
    mergeTokens,
    previewTemplate,
    replaceTemplate,
    saveTemplate,
    type EmailTemplate,
    type TemplatePreview,
  } from "#lib/api/crm.js";
  import { t } from "#lib/i18n.svelte.js";

  const list = resource(listTemplates);
  const tokens = resource(mergeTokens);
  const templates = $derived(list.value ?? []);

  let editing = $state<EmailTemplate | null>(null);
  let name = $state("");
  let subject = $state("");
  let body = $state("");
  let fallbacks = $state<Record<string, string>>({});
  let blank = $state<string[]>([]);
  let preview = $state<TemplatePreview | null>(null);
  let error = $state<string | null>(null);
  let previewError = $state<string | null>(null);

  const withFallback = $derived((tokens.value ?? []).filter((x) => x.fallback !== null));

  function open(template: EmailTemplate | null) {
    editing = template;
    name = template?.name ?? "";
    subject = template?.subject ?? "";
    body = template?.body ?? "";
    fallbacks = { ...template?.fallbacks };
    blank = [];
    error = null;
  }

  // Render the draft as it is typed, a moment after the last change.
  $effect(() => {
    const draft = { subject, body, fallbacks: clean(fallbacks) };
    const missing = [...blank];
    const timer = setTimeout(async () => {
      try {
        preview = await previewTemplate(draft, missing);
        previewError = null;
      } catch (cause) {
        preview = null;
        previewError = errorMessage(cause);
      }
    }, 250);
    return () => clearTimeout(timer);
  });

  const clean = (all: Record<string, string>) =>
    Object.fromEntries(Object.entries(all).filter(([, v]) => v.trim() !== ""));

  async function act(work: () => Promise<unknown>, after?: () => void) {
    error = null;
    try {
      await work();
      after?.();
      await list.reload();
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  const save = () =>
    act(
      () => {
        const input = { name, subject, body, fallbacks: clean(fallbacks) };
        return editing ? replaceTemplate(editing.pid, input) : saveTemplate(input);
      },
      () => open(null),
    );
  const remove = () => act(() => deleteTemplate(editing!.pid), () => open(null));
</script>

<svelte:head><title>{t("tpl.title")} — CRM</title></svelte:head>

<h1>{t("tpl.title")}</h1>
{#if list.error}<p class="error" data-testid="error">{list.error}</p>{/if}

<div class="templates">
  <section>
    <p><button type="button" data-testid="template-new" onclick={() => open(null)}>{t("tpl.new")}</button></p>
    {#if list.value && templates.length === 0}
      <p class="muted">{t("tpl.empty")}</p>
    {/if}
    <ul data-testid="template-list">
      {#each templates as template (template.pid)}
        <li>
          <button type="button" class="link" onclick={() => open(template)}>{template.name}</button>
          {#if template.in_use > 0}<span class="chip">{t("tpl.inUse")}: {template.in_use}</span>{/if}
        </li>
      {/each}
    </ul>
  </section>

  <section>
    {#if error}<p class="error" role="alert" data-testid="template-error">{error}</p>{/if}
    <label>{t("tpl.name")} <input data-testid="template-name" bind:value={name} /></label>
    <label>{t("tpl.subject")} <input data-testid="template-subject" bind:value={subject} /></label>
    <label>
      {t("tpl.body")}
      <textarea data-testid="template-body" rows="8" bind:value={body}></textarea>
    </label>
    <p>
      {t("tpl.insert")}:
      {#each tokens.value ?? [] as token (token.name)}
        <button
          type="button"
          title={token.description}
          data-testid={`insert-${token.name}`}
          onclick={() => (body += `{{${token.name}}}`)}
        >
          {`{{${token.name}}}`}
        </button>
      {/each}
    </p>
    <fieldset>
      <legend>{t("tpl.fallbacks")}</legend>
      {#each withFallback as token (token.name)}
        <label>
          {token.name}
          <input
            data-testid={`fallback-${token.name}`}
            placeholder={token.fallback ?? ""}
            bind:value={fallbacks[token.name]}
          />
        </label>
      {/each}
    </fieldset>
    <p>
      <button type="button" data-testid="template-save" disabled={!name.trim()} onclick={save}>
        {t("tpl.save")}
      </button>
      {#if editing}
        <button
          type="button"
          data-testid="template-delete"
          disabled={editing.in_use > 0}
          onclick={remove}
        >
          {t("tpl.delete")}
        </button>
      {/if}
    </p>
  </section>

  <section class="panel" data-testid="template-preview">
    <h2>{t("tpl.preview")}</h2>
    <p>
      {#each withFallback as token (token.name)}
        <label>
          <input
            type="checkbox"
            data-testid={`blank-${token.name}`}
            checked={blank.includes(token.name)}
            onchange={(event) =>
              (blank = event.currentTarget.checked
                ? [...blank, token.name]
                : blank.filter((x) => x !== token.name))}
          />
          {t("tpl.blank")}: {token.name}
        </label>
      {/each}
    </p>
    {#if previewError}
      <p class="error" data-testid="preview-error">{previewError}</p>
    {:else if preview}
      <p><strong data-testid="preview-subject">{preview.subject}</strong></p>
      <pre data-testid="preview-text">{preview.text}</pre>
      {#if preview.fell_back.length}
        <p class="muted" data-testid="preview-fell-back">
          {t("tpl.fellBack")}: {preview.fell_back.join(", ")}
        </p>
      {/if}
      {#if preview.problems.length}
        <div data-testid="preview-problems">
          <strong>{t("tpl.problems")}</strong>
          <ul>
            {#each preview.problems as problem (problem)}<li>{problem}</li>{/each}
          </ul>
        </div>
      {/if}
    {/if}
  </section>
</div>

<style>
  .templates {
    display: grid;
    grid-template-columns: minmax(10rem, 1fr) minmax(16rem, 2fr) minmax(16rem, 2fr);
    gap: 1.5rem;
  }
  @media (max-width: 60rem) {
    .templates {
      grid-template-columns: 1fr;
    }
  }
  label {
    display: block;
    margin-bottom: 0.5rem;
  }
  input:not([type="checkbox"]),
  textarea {
    display: block;
    width: 100%;
    box-sizing: border-box;
  }
  pre {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  ul {
    list-style: none;
    padding: 0;
  }
</style>
