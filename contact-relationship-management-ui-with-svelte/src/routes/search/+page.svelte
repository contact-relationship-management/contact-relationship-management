<!--
  Search (`/search?q=…`): the header search box lands here. It looks in every
  list the app shows (contacts, accounts, leads, deals, tickets, campaigns and
  the knowledge base), filtering in the browser from the same reads the screens
  use, so masking and record-level access apply as they do there. A group that
  fails to load says so; the others still show.
-->
<script lang="ts">
  import { page } from "$app/state";
  import {
    listAccounts,
    listArticles,
    listCampaigns,
    listContacts,
    listDeals,
    listLeads,
    listTickets,
  } from "#lib/api/crm.js";
  import { matches } from "#lib/search.js";
  import { t, type StringKey } from "#lib/i18n.svelte.js";

  type Hit = { id: string; title: string; detail: string; href: string };
  type Group = { key: StringKey; hits: Hit[] | null; failed: boolean };

  // The header sends `?q=ada`, or a bare `?ada` (the old contact search).
  const query = $derived(
    (page.url.searchParams.get("q") ?? decodeURIComponent(page.url.search.slice(1))).trim(),
  );

  let groups = $state<Group[]>([]);
  let loading = $state(false);

  const SOURCES: {
    key: StringKey;
    run: () => Promise<Hit[]>;
  }[] = [
    {
      key: "nav.contacts",
      run: async () =>
        (await listContacts())
          .filter((c) => matches([c.display_name, c.job_title, c.status], query))
          .map((c) => ({
            id: c.pid,
            title: c.display_name,
            detail: [c.job_title, c.status].filter(Boolean).join(" · "),
            href: `/contacts/${c.pid}`,
          })),
    },
    {
      key: "nav.accounts",
      run: async () =>
        (await listAccounts())
          .filter((a) => matches([a.display_name, a.tier, a.industry], query))
          .map((a) => ({
            id: a.pid,
            title: a.display_name,
            detail: [a.tier, a.industry].filter(Boolean).join(" · "),
            href: "/accounts",
          })),
    },
    {
      key: "nav.leads",
      run: async () =>
        (await listLeads())
          .filter((l) => matches([l.display_name, l.source, l.status], query))
          .map((l) => ({
            id: l.pid,
            title: l.display_name,
            detail: [l.source, l.status].join(" · "),
            href: "/leads",
          })),
    },
    {
      key: "nav.deals",
      run: async () =>
        (await listDeals())
          .filter((d) => matches([d.name], query))
          .map((d) => ({ id: d.pid, title: d.name, detail: "", href: "/deals" })),
    },
    {
      key: "nav.tickets",
      run: async () =>
        (await listTickets())
          .filter((x) => matches([x.title, x.priority, x.status], query))
          .map((x) => ({
            id: x.pid,
            title: x.title,
            detail: [x.priority, x.status].join(" · "),
            href: "/tickets",
          })),
    },
    {
      key: "nav.campaigns",
      run: async () =>
        (await listCampaigns())
          .filter((c) => matches([c.name, c.status], query))
          .map((c) => ({ id: c.pid, title: c.name, detail: c.status, href: "/campaigns" })),
    },
    {
      key: "nav.articles",
      run: async () =>
        (await listArticles())
          .filter((a) => matches([a.title, a.body], query))
          .map((a) => ({ id: a.pid, title: a.title, detail: a.status, href: "/articles" })),
    },
  ];

  $effect(() => {
    const current = query;
    if (current === "") {
      groups = [];
      return;
    }
    let stale = false;
    loading = true;
    void (async () => {
      const settled = await Promise.allSettled(SOURCES.map((s) => s.run()));
      if (stale) return;
      groups = settled.map((result, i) => ({
        key: SOURCES[i]!.key,
        hits: result.status === "fulfilled" ? result.value : null,
        failed: result.status === "rejected",
      }));
      loading = false;
    })();
    return () => {
      stale = true;
    };
  });

  const total = $derived(groups.reduce((n, g) => n + (g.hits?.length ?? 0), 0));
  const anyFailed = $derived(groups.some((g) => g.failed));
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
      {t("search.summary").replace("{count}", String(total)).replace("{query}", query)}
    {/if}
  </p>
  {#if anyFailed && !loading}
    <p class="error" role="alert" data-testid="search-failed">{t("search.failed")}</p>
  {/if}
  {#each groups as group (group.key)}
    {#if group.hits && group.hits.length > 0}
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
