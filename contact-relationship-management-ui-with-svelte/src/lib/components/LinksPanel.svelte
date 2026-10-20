<!--
  LinksPanel — links from a record to the same record in other systems (CRM-R27).
  Only links: nothing is fetched from, or sent to, those systems.

  Props
    - kind     contacts | accounts | deals | leads
    - pid      the record
    - version  change it to make the panel reload (the contact page does after a merge)
-->
<script lang="ts">
  import {
    LINK_SYSTEMS,
    addLink,
    getLinks,
    removeLink,
    type ExternalLink,
    type LinkKind,
    type LinkSystem,
  } from "#lib/api/crm.js";
  import { t } from "#lib/i18n.svelte.js";

  let { kind, pid, version = 0 }: { kind: LinkKind; pid: string; version?: number } = $props();

  let links = $state<ExternalLink[] | null>(null);
  let system = $state<LinkSystem>("hubspot");
  let url = $state("");
  let label = $state("");
  let busy = $state(false);
  let error = $state<string | null>(null);

  const systemName = (value: LinkSystem) =>
    ({
      hubspot: "HubSpot",
      salesforce: "Salesforce",
      attio: "Attio",
      pipedrive: "Pipedrive",
      dynamics: "Microsoft Dynamics",
      other: t("contact.linkSystemOther"),
    })[value];

  async function load() {
    try {
      links = (await getLinks(kind, pid)).links;
    } catch {
      links = null;
    }
  }

  $effect(() => {
    void version;
    void kind;
    void pid;
    void load();
  });

  async function add() {
    if (!url.trim()) return;
    busy = true;
    error = null;
    try {
      await addLink(kind, pid, { system, url: url.trim(), label });
      url = "";
      label = "";
      await load();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      busy = false;
    }
  }

  async function drop(link: ExternalLink) {
    error = null;
    try {
      await removeLink(link.pid);
      await load();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }
</script>

<h2>{t("contact.links")}</h2>
<p class="muted">{t("contact.linksHint")}</p>
{#if links !== null}
  {#if links.length === 0}
    <p data-testid="links-none">{t("contact.linksNone")}</p>
  {:else}
    <ul data-testid="links">
      {#each links as link (link.pid)}
        <li data-testid="link">
          <span class="chip">{systemName(link.system)}</span>
          <a
            href={link.url}
            target="_blank"
            rel="noopener noreferrer"
            aria-label={`${link.label ?? new URL(link.url).hostname} (${t("contact.linkOpens")})`}
            data-testid="link-open">{link.label ?? new URL(link.url).hostname}</a
          >
          <button onclick={() => void drop(link)} data-testid="link-remove">
            {t("contact.linkRemove")}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
{/if}
<form
  onsubmit={(event) => {
    event.preventDefault();
    void add();
  }}
  data-testid="link-form"
>
  <label>
    {t("contact.linkSystem")}
    <select bind:value={system} data-testid="link-system">
      {#each LINK_SYSTEMS as option (option)}
        <option value={option}>{systemName(option)}</option>
      {/each}
    </select>
  </label>
  <label>
    {t("contact.linkUrl")}
    <input type="url" bind:value={url} required data-testid="link-url" />
  </label>
  <label>
    {t("contact.linkLabel")}
    <input bind:value={label} data-testid="link-label" />
  </label>
  <button type="submit" disabled={busy || !url.trim()} data-testid="link-add">
    {t("contact.linkAdd")}
  </button>
  {#if error}<p role="alert" data-testid="link-error">{error}</p>{/if}
</form>
