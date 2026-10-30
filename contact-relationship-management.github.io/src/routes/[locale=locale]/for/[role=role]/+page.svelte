<script lang="ts">
  import Doc from "#lib/Doc.svelte";
  import { href } from "#lib/href.js";
  import { t } from "#lib/i18n.js";
  import { langTag } from "#lib/site.js";
  import type { PageProps } from "./$types";

  let { data }: PageProps = $props();
</script>

<svelte:head>
  <title>{data.title} · Contact Relationship Management</title>
  {#each data.alternates as a (a.locale)}
    <link rel="alternate" hreflang={langTag(a.locale)} href={href(`/${a.locale}/for/${a.slug}/`)} />
  {/each}
</svelte:head>

<p class="up"><a href={href(`/${data.locale}/for/`)}>{t(data.locale, "allBriefs")}</a></p>
<Doc html={data.html} path={data.path} locale={data.locale} />

<style>
  .up {
    margin-block: 0 1rem;
    font-size: 0.875rem;
  }
</style>
