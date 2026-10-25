<!--
  ViewBar (CRM-R35): the saved views of one list. Choose a view to apply it; "Save a view" opens a
  small editor for the filter, sort and columns, and a view you own can be changed, shared or
  deleted. The view only says what to ask for: the page applies it to the rows the caller's own
  list read returned, so sharing a view shares no access.

  Props:
    - kind     which list (`contact`, `account`, `deal`, `lead` or `ticket`)
    - active   (bindable) the view chosen, or null for the whole list
    - labels   a column id to its heading, for the editor
-->
<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import { resource } from "#lib/resource.svelte.js";
  import {
    deleteView,
    listViews,
    replaceView,
    saveView,
    type SavedView,
    type SavedViewInput,
  } from "#lib/api/crm.js";
  import { t } from "#lib/i18n.svelte.js";
  import { KINDS, type ViewKind } from "#lib/views.js";

  let {
    kind,
    active = $bindable(null),
    labels = {},
  }: {
    kind: ViewKind;
    active?: SavedView | null;
    labels?: Record<string, string>;
  } = $props();

  const config = $derived(KINDS[kind]);
  const views = resource(() => listViews(kind), { keepWhileLoading: true });
  const label = (id: string) => labels[id] ?? id;

  let editing = $state(false);
  let error = $state<string | null>(null);
  let name = $state("");
  let shared = $state(false);
  let filter = $state<Record<string, string[]>>({});
  let sortField = $state("");
  let sortDirection = $state<"asc" | "desc">("asc");
  let columns = $state<string[]>([]);

  function open(from: SavedView | null) {
    error = null;
    name = from?.name ?? "";
    shared = from?.shared ?? false;
    filter = structuredClone($state.snapshot(from?.filter ?? {}));
    sortField = from?.sort?.field ?? "";
    sortDirection = from?.sort?.direction ?? "asc";
    columns = [...(from?.columns ?? config.columns)];
    editing = true;
  }

  function toggle(field: string, value: string) {
    const kept = filter[field] ?? [];
    const next = kept.includes(value) ? kept.filter((v) => v !== value) : [...kept, value];
    if (next.length === 0) delete filter[field];
    else filter[field] = next;
  }

  const toggleColumn = (id: string) => {
    columns = columns.includes(id) ? columns.filter((c) => c !== id) : [...columns, id];
  };

  async function save(replace: boolean) {
    error = null;
    const view: SavedViewInput = {
      kind,
      name,
      filter: $state.snapshot(filter),
      sort: sortField ? { field: sortField, direction: sortDirection } : null,
      columns: columns.length === config.columns.length ? null : $state.snapshot(columns),
      shared,
    };
    try {
      const saved = replace && active ? await replaceView(active.pid, view) : await saveView(view);
      await views.reload();
      active = views.value?.find((v) => v.pid === saved.pid) ?? null;
      editing = false;
    } catch (cause) {
      error = errorMessage(cause);
    }
  }

  async function remove() {
    if (!active) return;
    try {
      await deleteView(active.pid);
      active = null;
      await views.reload();
    } catch (cause) {
      error = errorMessage(cause);
    }
  }
</script>

<div class="viewbar" data-testid="view-bar">
  <label>
    {t("view.label")}
    <select
      data-testid="view-select"
      value={active?.pid ?? ""}
      onchange={(event) => {
        const pid = event.currentTarget.value;
        active = (views.value ?? []).find((v) => v.pid === pid) ?? null;
      }}
    >
      <option value="">{t("view.all")}</option>
      {#each views.value ?? [] as v (v.pid)}
        <option value={v.pid}>{v.name}{v.mine ? "" : ` · ${t("view.shared")}`}</option>
      {/each}
    </select>
  </label>
  <button type="button" data-testid="view-new" onclick={() => open(null)}>{t("view.save")}</button>
  {#if active?.mine}
    <button type="button" data-testid="view-edit" onclick={() => open(active)}>{t("view.edit")}</button>
    <button type="button" data-testid="view-delete" onclick={remove}>{t("view.delete")}</button>
  {/if}
</div>

{#if error}<p class="error" role="alert" data-testid="view-error">{error}</p>{/if}

{#if editing}
  <fieldset class="vieweditor" data-testid="view-editor">
    <p>
      <label>
        {t("view.name")}
        <input data-testid="view-name" bind:value={name} maxlength="80" />
      </label>
      <label>
        <input type="checkbox" data-testid="view-shared" bind:checked={shared} />
        {t("view.shareWith")}
      </label>
    </p>
    {#each Object.entries(config.filters) as [field, values] (field)}
      <p>
        <strong>{label(field)}</strong>
        {#each values as value (value)}
          <label>
            <input
              type="checkbox"
              checked={(filter[field] ?? []).includes(value)}
              onchange={() => toggle(field, value)}
            />
            {value}
          </label>
        {/each}
      </p>
    {/each}
    <p>
      <label>
        {t("view.sort")}
        <select data-testid="view-sort" bind:value={sortField}>
          <option value="">{t("view.none")}</option>
          {#each config.columns as id (id)}<option value={id}>{label(id)}</option>{/each}
        </select>
      </label>
      <select aria-label={t("view.direction")} bind:value={sortDirection}>
        <option value="asc">{t("view.asc")}</option>
        <option value="desc">{t("view.desc")}</option>
      </select>
    </p>
    <p>
      <strong>{t("view.columns")}</strong>
      {#each config.columns as id (id)}
        <label>
          <input type="checkbox" checked={columns.includes(id)} onchange={() => toggleColumn(id)} />
          {label(id)}
        </label>
      {/each}
    </p>
    <button type="button" data-testid="view-create" disabled={name.trim() === ""} onclick={() => save(false)}>
      {t("view.saveNew")}
    </button>
    {#if active?.mine}
      <button type="button" data-testid="view-replace" disabled={name.trim() === ""} onclick={() => save(true)}>
        {t("view.update")}
      </button>
    {/if}
    <button type="button" onclick={() => (editing = false)}>{t("view.cancel")}</button>
  </fieldset>
{/if}

<style>
  .viewbar {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    flex-wrap: wrap;
    margin-block: 0.5rem;
  }
  .vieweditor label {
    margin-inline-end: 0.75rem;
  }
</style>
