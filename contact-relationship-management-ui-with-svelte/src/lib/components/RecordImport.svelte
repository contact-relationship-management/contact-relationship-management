<!--
  RecordImport — import contacts or accounts from a CSV (CRM-R43): choose a file, map its columns,
  read the report of each row (valid, or invalid with a reason, with likely duplicates as advice),
  import the valid rows and, if it was a mistake, undo the import.

  Props
    - kind  contacts | accounts
-->
<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import DataTable from "#lib/components/DataTable.svelte";
  import {
    commitRecordImport,
    previewRecordImport,
    undoRecordImport,
    type ImportKind,
    type RecordImport,
    type RecordImportMapping,
    type RecordImportPreview,
  } from "#lib/api/crm.js";
  import { t, tn, tf } from "#lib/i18n.svelte.js";
  import { formatPercent } from "#lib/locale-format.js";
  import { todayIso } from "#lib/reader-calendar.js";

  let { kind }: { kind: ImportKind } = $props();

  const MAX_BYTES = 1_000_000;

  const FIELDS = {
    contacts: [
      ["display_name", "import.nameCol"],
      ["email", "import.emailCol"],
      ["given_name", "import.givenCol"],
      ["family_name", "import.familyCol"],
      ["job_title", "import.titleCol"],
    ],
    accounts: [
      ["display_name", "import.nameCol"],
      ["organization_ref", "import.orgRefCol"],
      ["tier", "import.tierCol"],
      ["industry", "import.industryCol"],
    ],
  } as const;
  const required = $derived(kind === "contacts" ? ["display_name", "email"] : ["display_name"]);

  let csv = $state("");
  let preview = $state<RecordImportPreview | null>(null);
  let mapping = $state<RecordImportMapping>({});
  let sourceNote = $state("");
  let obtainedOn = $state(todayIso());
  let skipDuplicates = $state(false);
  let done = $state<RecordImport | null>(null);
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
      preview = await previewRecordImport(kind, csv);
      mapping = { ...preview.suggested_mapping };
    });
  }

  const check = () =>
    run(async () => {
      preview = await previewRecordImport(kind, csv, mapping);
    });
  const importIt = () =>
    run(async () => {
      done = await commitRecordImport(kind, {
        csv,
        mapping,
        source_note: sourceNote,
        obtained_on: obtainedOn,
        skip_likely_duplicates: skipDuplicates,
      });
    });
  const undo = () =>
    run(async () => {
      if (done) undone = await undoRecordImport(kind, done.pid);
    });

  const mapped = $derived(required.every((f) => mapping[f as keyof RecordImportMapping]));
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
        .map((d) => `${d.name} (${formatPercent(d.probability)})`)
        .join(", "),
    })),
  );
  const title = $derived(t(kind === "contacts" ? "import.contactsTitle" : "import.accountsTitle"));
</script>

<svelte:head><title>{title} — CRM</title></svelte:head>

<h1>{title}</h1>
<p class="muted">{t(kind === "contacts" ? "import.contactsNote" : "import.accountsNote")}</p>
{#if error}<p class="error" role="alert" data-testid="error">{error}</p>{/if}

<p>
  <label>
    {t("import.file")}
    <input type="file" accept=".csv,text/csv" data-testid="file" onchange={choose} />
  </label>
</p>

{#if preview}
  <fieldset>
    {#each FIELDS[kind] as [field, key] (field)}
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
    <button type="button" data-testid="check" disabled={busy || !mapped} onclick={check}>
      {t("import.check")}
    </button>
  </fieldset>

  {#if preview.report}
    <p data-testid="summary">
      {t("import.rows")}: {preview.rows} · {t("import.valid")}: {preview.valid} ·
      {t("import.invalid")}: {preview.invalid} · {t("import.lookslike")}: {preview.likely_duplicates}
    </p>
    <DataTable label={title} {columns} {rows} testid="import-report" />

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
    {tn("import.done", done.created_count, { count: done.created_count })}
  </p>
  <p class="muted" data-testid="done-detail">
    {t("import.leftOut")}: {done.skipped_count} · {t("import.lookupFailed")}: {done.failed_count}
  </p>
  {#if !undone}
    <button type="button" data-testid="undo" disabled={busy} onclick={undo}>{t("import.undo")}</button>
  {:else}
    <p role="status" data-testid="undone">
      {tf("import.undone", { removed: undone.removed, kept: undone.kept })}
    </p>
  {/if}
{/if}
