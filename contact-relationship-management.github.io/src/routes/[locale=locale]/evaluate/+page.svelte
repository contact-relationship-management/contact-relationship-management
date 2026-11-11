<script lang="ts">
  import type { PageProps } from "./$types";

  let { data }: PageProps = $props();
</script>

<svelte:head><title>{data.title} - CRM</title></svelte:head>

{@html data.hero}

{@html data.body}
