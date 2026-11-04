<script lang="ts">
  import Doc from "#lib/Doc.svelte";
  import type { PageProps } from "./$types";

  let { data }: PageProps = $props();
</script>

<svelte:head><title>{data.title} · Contact Relationship Management</title></svelte:head>

<Doc html={data.html} path={data.path} />
