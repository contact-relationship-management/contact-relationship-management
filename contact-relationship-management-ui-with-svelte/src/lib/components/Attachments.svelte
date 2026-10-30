<!--
  Attachments — the files attached to one record (CRM-R42).

  The service accepts documents, images and text only, checks them, and scans them when it has a
  scanner. A download is always saved, never opened in the page. A masked reader is refused, and the
  panel then says nothing rather than showing an error.

  Props
    - kind  contacts | accounts | deals | leads
    - pid   the record
-->
<script lang="ts">
  import { errorMessage } from "#lib/errors.js";
  import { resource } from "#lib/resource.svelte.js";
  import {
    attachmentUrl,
    deleteAttachment,
    listAttachments,
    uploadAttachment,
    type Attachment,
    type LinkKind,
  } from "#lib/api/crm.js";
  import { t } from "#lib/i18n.svelte.js";
  import { formatNumber } from "#lib/locale-format.js";

  let { kind, pid }: { kind: LinkKind; pid: string } = $props();

  const files = resource(async () => {
    try {
      return await listAttachments(kind, pid);
    } catch {
      return null;
    }
  });
  const rows = $derived<Attachment[] | null>(files.value);

  let busy = $state(false);
  let error = $state<string | null>(null);

  // A file's size in the page's number format and unit names ("1,5 MB" in German).
  const size = (bytes: number) =>
    bytes < 1024
      ? formatNumber(bytes, { style: "unit", unit: "byte", unitDisplay: "short" })
      : bytes < 1048576
        ? formatNumber(bytes / 1024, { style: "unit", unit: "kilobyte", maximumFractionDigits: 0 })
        : formatNumber(bytes / 1048576, { style: "unit", unit: "megabyte", maximumFractionDigits: 1 });

  async function pick(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    busy = true;
    error = null;
    try {
      await uploadAttachment(kind, pid, file);
      await files.reload();
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
      input.value = "";
    }
  }

  async function remove(file: Attachment) {
    if (!confirm(t("att.deleteConfirm"))) return;
    error = null;
    try {
      await deleteAttachment(file.pid);
      await files.reload();
    } catch (cause) {
      error = errorMessage(cause);
    }
  }
</script>

{#if rows !== null}
  <div class="panel" data-testid="attachments">
    <h2>{t("att.title")}</h2>
    {#if rows.length === 0}
      <p class="muted" data-testid="att-none">{t("att.none")}</p>
    {:else}
      <ul data-testid="att-list">
        {#each rows as file (file.pid)}
          <li>
            {file.name}
            <span class="muted small">{size(file.size_bytes)}</span>
            <a href={attachmentUrl(file.pid)} download>{t("att.download")}</a>
            <button type="button" onclick={() => void remove(file)}>{t("att.delete")}</button>
          </li>
        {/each}
      </ul>
    {/if}
    <label>
      {t("att.add")}
      <input type="file" onchange={pick} disabled={busy} data-testid="att-file" />
    </label>
    <p class="muted small">{t("att.hint")}</p>
    {#if error}<p class="error" data-testid="att-error">{error}</p>{/if}
  </div>
{/if}
