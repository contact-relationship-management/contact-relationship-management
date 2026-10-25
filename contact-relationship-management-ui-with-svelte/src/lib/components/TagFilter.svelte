<!--
  TagFilter — narrow a list to the records that carry one tag (CRM-R47). The service leaves out
  records a masked reader may not see, so this reveals nothing the list does not.

  Props
    - value  the chosen tag name, bindable; empty means no filter
-->
<script lang="ts">
  import { resource } from "#lib/resource.svelte.js";
  import { listTags } from "#lib/api/crm.js";
  import { t } from "#lib/i18n.svelte.js";

  let { value = $bindable("") }: { value?: string } = $props();

  const tags = resource(async () => {
    try {
      return await listTags();
    } catch {
      return [];
    }
  });
  const all = $derived(tags.value ?? []);
</script>

{#if all.length > 0}
  <label>
    {t("tag.filter")}
    <select bind:value data-testid="tag-filter">
      <option value="">{t("tag.any")}</option>
      {#each all as tag (tag.pid)}<option value={tag.name}>{tag.name}</option>{/each}
    </select>
  </label>
{/if}
