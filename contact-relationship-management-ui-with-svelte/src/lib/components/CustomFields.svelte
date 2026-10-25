<!--
  CustomFields — the custom fields of one record and their values (CRM-R41).

  A masked read shows a personal or sensitive field as hidden rather than empty. Saving sends
  only what the person changed, and the service checks each value against its field's kind.

  Props
    - kind  contacts | accounts | deals | leads
    - pid   the record
-->
<script lang="ts">
  import { errorMessage, fieldErrors } from "#lib/errors.js";
  import { resource } from "#lib/resource.svelte.js";
  import {
    getCustomFieldValues,
    setCustomFieldValues,
    type CustomFieldValue,
    type LinkKind,
  } from "#lib/api/crm.js";
  import { t } from "#lib/i18n.svelte.js";

  let { kind, pid }: { kind: LinkKind; pid: string } = $props();

  const fields = resource(() => getCustomFieldValues(kind, pid));
  const rows = $derived<CustomFieldValue[]>(fields.value ?? []);

  // What the person has typed, by key; a key absent here is unchanged.
  let draft = $state<Record<string, string>>({});
  let problems = $state<Record<string, string>>({});
  let error = $state<string | null>(null);
  let saved = $state(false);

  const shown = (row: CustomFieldValue) => draft[row.key] ?? (row.value === null ? "" : String(row.value));

  async function save() {
    error = null;
    problems = {};
    saved = false;
    const values: Record<string, string | number | null> = {};
    for (const row of rows) {
      const typed = draft[row.key];
      if (typed === undefined) continue;
      if (typed.trim() === "") values[row.key] = null;
      else values[row.key] = row.kind === "number" ? Number(typed) : typed;
    }
    if (Object.keys(values).length === 0) return;
    try {
      await setCustomFieldValues(kind, pid, values);
      draft = {};
      saved = true;
      await fields.reload();
    } catch (cause) {
      problems = fieldErrors(cause);
      error = errorMessage(cause);
    }
  }
</script>

{#if rows.length > 0}
  <form
    class="panel"
    data-testid="custom-fields"
    onsubmit={(e) => {
      e.preventDefault();
      void save();
    }}
  >
    <h2>{t("cf.title")}</h2>
    {#each rows as row (row.key)}
      <label>
        {row.label}
        {#if row.masked}
          <span class="muted" data-testid={`cf-masked-${row.key}`}>{t("cf.masked")}</span>
        {:else if row.kind === "choice"}
          <select
            value={shown(row)}
            onchange={(e) => (draft[row.key] = e.currentTarget.value)}
            aria-invalid={problems[row.key] ? "true" : undefined}
            data-testid={`cf-${row.key}`}
          >
            <option value=""></option>
            {#each row.choices as choice (choice)}<option value={choice}>{choice}</option>{/each}
          </select>
        {:else}
          <input
            type={row.kind === "number" ? "number" : row.kind === "date" ? "date" : "text"}
            step={row.kind === "number" ? "any" : undefined}
            value={shown(row)}
            oninput={(e) => (draft[row.key] = e.currentTarget.value)}
            aria-invalid={problems[row.key] ? "true" : undefined}
            data-testid={`cf-${row.key}`}
          />
        {/if}
        {#if problems[row.key]}<span class="error small">{problems[row.key]}</span>{/if}
      </label>
    {/each}
    {#if error}<p class="error" data-testid="cf-error">{error}</p>{/if}
    <button type="submit" data-testid="cf-save">{t("cf.save")}</button>
    {#if saved}<span data-testid="cf-saved">{t("contact.saved")}</span>{/if}
  </form>
{/if}
