<!--
  Path to production (/<locale>/path-to-production/): what production means and an estimate of readiness for each
  part. Each estimate is shown as a Lily Slider, disabled because it is a reading, not a control, with the
  percentage also written beside it.
-->
<script lang="ts">
  import { Slider } from "@lilydesignsystem/svelte-headless";
  import { t } from "#lib/i18n.js";
  import type { PageProps } from "./$types";

  let { data }: PageProps = $props();
</script>

<svelte:head><title>{data.title} - CRM</title></svelte:head>

{@html data.before}

<ul class="readiness" data-testid="readiness">
  {#each data.items as item (item.name)}
    <li>
      <div class="estimate">
        <strong>{item.name}</strong>
        <Slider
          label={`${item.name}: ${t(data.locale, "readiness.estimate")}`}
          value={item.percent}
          min={0}
          max={100}
          step={1}
          disabled={true}
          aria-valuetext={`${item.percent}%`}
        />
        <output aria-hidden="true">{item.percent}%</output>
      </div>
      {@html item.html}
    </li>
  {/each}
</ul>

{@html data.after}

<style>
  .readiness {
    list-style: none;
    padding: 0;
  }
  .readiness li {
    margin-block: 1.5rem;
  }
  .estimate {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem 1rem;
  }
  .estimate strong {
    flex: 1 1 14rem;
  }
  .estimate :global(input[type="range"]) {
    flex: 1 1 10rem;
    max-inline-size: 16rem;
    accent-color: currentColor;
  }
  .estimate :global(input[type="range"]:disabled) {
    opacity: 1;
    cursor: default;
  }
  .estimate output {
    min-inline-size: 3.5ch;
    text-align: end;
    font-variant-numeric: tabular-nums;
  }
  .readiness li :global(p) {
    margin-block: 0.25rem 0;
  }
</style>
