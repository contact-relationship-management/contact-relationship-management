<!--
  Import leads (`/leads/import`): choose a CSV, map its columns, read the report of each
  row (valid, or invalid with a reason, with likely duplicates as advice only), then import
  the valid rows and, if it was a mistake, undo the import. Imported leads are not contacts
  and carry no consent (CRM-R34).
-->
<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import DataTable from "#lib/components/DataTable.svelte";
  import {
    commitLeadImport,
    previewLeadImport,
    undoLeadImport,
    type LeadImport,
    type LeadImportMapping,
    type LeadImportPreview,
  } from "#lib/api/crm.js";
  import { t } from "#lib/i18n.svelte.js";

  const MAX_BYTES = 1_000_000;

  let csv = $state("");
  let preview = $state<LeadImportPreview | null>(null);
  let mapping = $state<LeadImportMapping>({});
  let sourceNote = $state("");
  let obtainedOn = $state(new Date().toISOString().slice(0, 10));
  let skipDuplicates = $state(false);
  let done = $state<LeadImport | null>(null);
  let undone = $state<{ removed: number; kept: number } | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);

  async function run(work: () => Promise<void>) {
    error = null;
    busy = true;
    try {
      await work();
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }

  async function choose(event: Event) {
    const file = (event.currentTarget as HTMLInputElement).files?.[0];
    preview = null;
    done = null;
    undone = null;
    if (!file) return;
    if (file.size > MAX_BYTES) {
      error = `${file.name}: > ${MAX_BYTES} bytes`;
      return;
    }
    csv = await file.text();
    await run(async () => {
      preview = await previewLeadImport(csv);
      mapping = { ...preview.suggested_mapping };
    });
  }

  const check = () =>
    run(async () => {
      preview = await previewLeadImport(csv, mapping);
    });
  const importIt = () =>
    run(async () => {
      done = await commitLeadImport({
        csv,
        mapping,
        source_note: sourceNote,
        obtained_on: obtainedOn,
        skip_likely_duplicates: skipDuplicates,
      });
    });
  const undo = () =>
    run(async () => {
      if (done) undone = await undoLeadImport(done.pid);
    });

  const columns = $derived([
    { id: "row", kind: "number", header: t("import.row") },
    { id: "status", kind: "chip", header: t("import.rows") },
    { id: "name", header: t("common.name") },
    { id: "reason", header: t("import.reason") },
    { id: "looks", header: t("import.lookslike") },
  ]);
  const rows = $derived(
    (preview?.report ?? []).map((r) => ({
      id: String(r.row),
      row: r.row,
      status: r.status === "valid" ? t("import.valid") : t("import.invalid"),
      name: r.name ?? "",
      reason: r.reason ?? "",
      looks: r.likely_duplicates
        .map((d) => `${d.name} (${Math.round(d.probability * 100)}%)`)
        .join(", "),
    })),
  );
</script>

<svelte:head><title>{t("import.title")} — CRM</title></svelte:head>

<h1>{t("import.title")}</h1>
<p class="muted">{t("import.consentNote")}</p>
{#if error}<p class="error" role="alert" data-testid="error">{error}</p>{/if}

<p>
  <label>
    {t("import.file")}
    <input type="file" accept=".csv,text/csv" data-testid="file" onchange={choose} />
  </label>
</p>

{#if preview}
  <fieldset>
    {#each [["display_name", "import.nameCol"], ["email", "import.emailCol"], ["company", "import.companyCol"]] as const as [field, key] (field)}
      <p>
        <label>
          {t(key)}
          <select data-testid={`map-${field}`} bind:value={mapping[field]}>
            <option value={null}>{t("import.none")}</option>
            {#each preview.columns as column (column)}
              <option value={column}>{column}</option>
            {/each}
          </select>
        </label>
      </p>
    {/each}
    <button type="button" data-testid="check" disabled={busy || !mapping.display_name} onclick={check}>
      {t("import.check")}
    </button>
  </fieldset>

  {#if preview.report}
    <p data-testid="summary">
      {t("import.rows")}: {preview.rows} · {t("import.valid")}: {preview.valid} ·
      {t("import.invalid")}: {preview.invalid} · {t("import.lookslike")}: {preview.likely_duplicates}
    </p>
    <DataTable label={t("import.title")} {columns} {rows} testid="import-report" />

    {#if !done}
      <fieldset>
        <p>
          <label>
            {t("import.source")}
            <input data-testid="source" bind:value={sourceNote} maxlength="200" />
          </label>
        </p>
        <p>
          <label>
            {t("import.obtained")}
            <input type="date" data-testid="obtained" bind:value={obtainedOn} />
          </label>
        </p>
        <p>
          <label>
            <input type="checkbox" bind:checked={skipDuplicates} />
            {t("import.skipDup")}
          </label>
        </p>
        <button
          type="button"
          data-testid="commit"
          disabled={busy || preview.valid === 0 || sourceNote.trim() === ""}
          onclick={importIt}
        >
          {t("import.commit")}
        </button>
      </fieldset>
    {/if}
  {/if}
{/if}

{#if done}
  <p role="status" data-testid="done">
    {t("import.done").replace("{count}", String(done.created_count))}
  </p>
  {#if !undone}
    <button type="button" data-testid="undo" disabled={busy} onclick={undo}>{t("import.undo")}</button>
  {:else}
    <p role="status" data-testid="undone">
      {t("import.undone")
        .replace("{removed}", String(undone.removed))
        .replace("{kept}", String(undone.kept))}
    </p>
  {/if}
{/if}
