<!--
  Tags — the tags on one record (CRM-R47). The whole set is saved on each change. A masked reader
  is refused by the service, and the panel then says nothing.

  Props
    - kind  contacts | accounts | deals | leads
    - pid   the record
-->
<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import { resource } from "#lib/resource.svelte.js";
  import { getRecordTags, setRecordTags, type LinkKind } from "#lib/api/crm.js";
  import { t } from "#lib/i18n.svelte.js";

  let { kind, pid }: { kind: LinkKind; pid: string } = $props();

  const held = resource(async () => {
    try {
      return await getRecordTags(kind, pid);
    } catch {
      return null;
    }
  });
  const names = $derived<string[] | null>(held.value);

  let draft = $state("");
  let error = $state<string | null>(null);

  async function save(next: string[]) {
    error = null;
    try {
      await setRecordTags(kind, pid, next);
      draft = "";
      await held.reload();
    } catch (cause) {
      error = errorMessage(cause);
    }
  }
</script>

{#if names !== null}
  <div class="panel" data-testid="tags">
    <h2>{t("tag.title")}</h2>
    {#if names.length === 0}
      <p class="muted" data-testid="tags-none">{t("tag.none")}</p>
    {:else}
      <ul class="tag-list" data-testid="tag-list">
        {#each names as name (name)}
          <li class="chip">
            {name}
            <button
              type="button"
              aria-label={`${t("tag.remove")}: ${name}`}
              onclick={() => void save(names.filter((n) => n !== name))}>×</button
            >
          </li>
        {/each}
      </ul>
    {/if}
    <form
      onsubmit={(e) => {
        e.preventDefault();
        if (draft.trim()) void save([...names, draft]);
      }}
    >
      <label>
        {t("tag.new")}
        <input bind:value={draft} maxlength="40" data-testid="tag-input" />
      </label>
      <button type="submit" data-testid="tag-add">{t("tag.add")}</button>
    </form>
    {#if error}<p class="error" data-testid="tag-error">{error}</p>{/if}
  </div>
{/if}

<style>
  .tag-list {
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
    padding: 0;
    margin: 0 0 0.5rem;
  }
  .tag-list button {
    border: 0;
    background: transparent;
    padding: 0 0.25rem;
    cursor: pointer;
  }
</style>
