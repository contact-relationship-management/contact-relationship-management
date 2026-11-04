<script lang="ts">
  import Landing from "#lib/Landing.svelte";
  import type { PageProps } from "./$types";

  let { data }: PageProps = $props();
</script>

<svelte:head><title>{data.title}</title></svelte:head>

<Landing hero={data.hero} body={data.body} locale={data.locale} />
