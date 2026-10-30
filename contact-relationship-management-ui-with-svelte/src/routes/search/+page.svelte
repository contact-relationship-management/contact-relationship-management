<!--
  Search (`/search?q=…`): the header search box lands here. The service finds
  contacts, accounts, leads, deals, tickets, campaigns and articles by name or
  title, ignoring accents and forgiving typos, and leaves out records the caller
  may not read (CRM-R33). The page groups what comes back.
-->
<script lang="ts">
  import { page } from "$app/state";
  import { searchRecords, type SearchHit } from "#lib/api/crm.js";
  import { longEnough } from "#lib/search.js";
  import { t, type StringKey, tn } from "#lib/i18n.svelte.js";

  type Hit = { id: string; title: string; detail: string; href: string };
  type Group = { key: StringKey; hits: Hit[] };

  // The header sends `?q=ada`, or a bare `?ada` (the old contact search).
  const query = $derived(
    (page.url.searchParams.get("q") ?? decodeURIComponent(page.url.search.slice(1))).trim(),
  );

  // The service searches names and titles (accent-blind, typo-tolerant) and leaves out what the
  // caller may not read; the page only groups the hits and links to each record.
  const KINDS: { kind: string; key: StringKey; href: (pid: string) => string }[] = [
    { kind: "contact", key: "nav.contacts", href: (p) => `/contacts/${p}` },
    { kind: "account", key: "nav.accounts", href: () => "/accounts" },
    { kind: "lead", key: "nav.leads", href: () => "/leads" },
    { kind: "deal", key: "nav.deals", href: (p) => `/deals/${p}` },
    { kind: "ticket", key: "nav.tickets", href: () => "/tickets" },
    { kind: "campaign", key: "nav.campaigns", href: () => "/campaigns" },
    { kind: "article", key: "nav.articles", href: () => "/articles" },
  ];

  function group(hits: SearchHit[]): Group[] {
    return KINDS.map(({ kind, key, href }) => ({
      key,
      hits: hits
        .filter((h) => h.kind === kind)
        .map((h) => ({ id: h.pid, title: h.title, detail: h.detail ?? "", href: href(h.pid) })),
    }));
  }

  let groups = $state<Group[]>([]);
  let loading = $state(false);
  let failed = $state(false);

  $effect(() => {
    const current = query;
    // The service wants two characters or more, or one ideograph; less is no search.
    if (!longEnough(current)) {
      groups = [];
      failed = false;
      loading = false;
      return;
    }
    let stale = false;
    loading = true;
    void (async () => {
      let result: Group[] = [];
      let error = false;
      try {
        result = group(await searchRecords(current));
      } catch {
        error = true;
      }
      if (stale) return;
      groups = result;
      failed = error;
      loading = false;
    })();
    return () => {
      stale = true;
    };
  });

  const total = $derived(groups.reduce((n, g) => n + g.hits.length, 0));
</script>

<svelte:head><title>{t("search.title")} — CRM</title></svelte:head>

<h1>{t("search.title")}</h1>

{#if query === ""}
  <p class="muted" data-testid="search-empty">{t("search.prompt")}</p>
{:else}
  <p role="status" data-testid="search-status">
    {#if loading}
      {t("common.loading")}
    {:else}
      {tn("search.summary", total, { count: total, query })}
    {/if}
  </p>
  {#if failed && !loading}
    <p class="error" role="alert" data-testid="search-failed">{t("search.failed")}</p>
  {/if}
  {#each groups as group (group.key)}
    {#if group.hits.length > 0}
      <section data-testid={`search-${group.key}`}>
        <h2>{t(group.key)} ({group.hits.length})</h2>
        <ul>
          {#each group.hits as hit (hit.id)}
            <li>
              <a href={hit.href}>{hit.title}</a>
              {#if hit.detail}<span class="muted"> — {hit.detail}</span>{/if}
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  {/each}
{/if}
