<!--
  Custom fields (`/custom-fields`): declare a field on contacts, accounts, leads or deals (CRM-R41).
  The key, type and sensitivity cannot change once the field is made, because values already
  depend on them; a field can be deleted, which deletes its values.
-->
<script lang="ts">
  import { errorMessage, fieldErrors } from "#lib/errors.js";
  import { resource } from "#lib/resource.svelte.js";
  import {
    createCustomField,
    deleteCustomField,
    listCustomFields,
    type CustomField,
  } from "#lib/api/crm.js";
  import { t } from "#lib/i18n.svelte.js";

  const list = resource(listCustomFields);
  const fields = $derived(list.value ?? []);

  const ENTITIES = ["contact", "account", "lead", "deal"] as const;
  const KINDS = {
    text: "cf.kind.text",
    number: "cf.kind.number",
    date: "cf.kind.date",
    choice: "cf.kind.choice",
  } as const;
  const SENSITIVITIES = {
    business: "cf.sens.business",
    personal: "cf.sens.personal",
    sensitive: "cf.sens.sensitive",
  } as const;

  let entity = $state<CustomField["entity"]>("contact");
  let key = $state("");
  let label = $state("");
  let kind = $state<CustomField["kind"]>("text");
  let choices = $state("");
  let sensitivity = $state<CustomField["sensitivity"]>("business");
  let error = $state<string | null>(null);
  let problems = $state<Record<string, string>>({});

  async function add() {
    error = null;
    problems = {};
    try {
      await createCustomField({
        entity,
        key,
        label,
        kind,
        sensitivity,
        choices:
          kind === "choice"
            ? choices
                .split(",")
                .map((c) => c.trim())
                .filter(Boolean)
            : [],
      });
      key = "";
      label = "";
      choices = "";
      await list.reload();
    } catch (cause) {
      problems = fieldErrors(cause);
      error = errorMessage(cause);
    }
  }

  async function remove(field: CustomField) {
    if (!confirm(t("cf.deleteConfirm"))) return;
    error = null;
    try {
      await deleteCustomField(field.pid);
      await list.reload();
    } catch (cause) {
      error = errorMessage(cause);
    }
  }
</script>

<h1>{t("cf.title")}</h1>
<p class="muted">{t("cf.hint")}</p>

<form
  class="panel"
  data-testid="cf-form"
  onsubmit={(e) => {
    e.preventDefault();
    void add();
  }}
>
  <label>
    {t("cf.entity")}
    <select bind:value={entity} data-testid="cf-entity">
      {#each ENTITIES as option (option)}<option value={option}>{option}</option>{/each}
    </select>
  </label>
  <label>
    {t("cf.key")}
    <input bind:value={key} required data-testid="cf-key" aria-invalid={problems.key ? "true" : undefined} />
    {#if problems.key}<span class="error small">{problems.key}</span>{/if}
  </label>
  <label>
    {t("cf.label")}
    <input bind:value={label} required data-testid="cf-label" />
  </label>
  <label>
    {t("cf.kind")}
    <select bind:value={kind} data-testid="cf-kind">
      {#each Object.entries(KINDS) as [value, text] (value)}
        <option {value}>{t(text)}</option>
      {/each}
    </select>
  </label>
  {#if kind === "choice"}
    <label>
      {t("cf.choices")}
      <input bind:value={choices} data-testid="cf-choices" />
    </label>
  {/if}
  <label>
    {t("cf.sensitivity")}
    <select bind:value={sensitivity} data-testid="cf-sensitivity">
      {#each Object.entries(SENSITIVITIES) as [value, text] (value)}
        <option {value}>{t(text)}</option>
      {/each}
    </select>
  </label>
  {#if error}<p class="error" data-testid="cf-form-error">{error}</p>{/if}
  <button type="submit" data-testid="cf-add">{t("cf.add")}</button>
</form>

{#if fields.length === 0}
  <p class="muted">{t("cf.none")}</p>
{:else}
  <ul data-testid="cf-list">
    {#each fields as field (field.pid)}
      <li>
        <strong>{field.label}</strong>
        <code>{field.entity}.{field.key}</code>
        <span class="chip">{t(KINDS[field.kind])}</span>
        <span class="chip">{field.sensitivity}</span>
        {#if field.choices.length}<span class="muted">{field.choices.join(", ")}</span>{/if}
        <button type="button" onclick={() => void remove(field)}>{t("cf.delete")}</button>
      </li>
    {/each}
  </ul>
{/if}
