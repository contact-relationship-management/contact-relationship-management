<!--
  Tour (/tour) — a public, in-depth walkthrough of the CRM. Copy lives in
  the i18n catalog (`tour.*`); this file only declares which app screen
  each workflow section links to, in order. With the tour on, the panel's
  button starts it in one click (a form POST to `?/start`, CRM-T233); the
  sign-in link is the other way in.
-->
<script lang="ts">
  import Tour from "#lib/components/Tour.svelte";
  import { t } from "#lib/i18n.svelte.js";
  import type { PageProps } from "./$types";

  let { data }: PageProps = $props();

  const sections = [
    { href: "/contacts" },
    { href: "/leads" },
    { href: "/deals" },
    { href: "/campaigns" },
    { href: "/tickets" },
    { href: "/executive" },
  ];
</script>

{#if data.demoTour}
  <div class="panel" data-testid="tour-signin">
    <form method="POST" action="?/start">
      <button class="primary" type="submit" data-testid="tour-begin"
        >{t("tour.begin")}</button
      >
    </form>
    <p>
      {t("signin.demo")} <code>example@example.com</code> —
      <a href="/signin">{t("auth.signin")}</a>
    </p>
  </div>
{/if}

<Tour {sections} />

<style>
  [data-testid="tour-signin"] {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem 1rem;
  }
  [data-testid="tour-signin"] p {
    margin: 0;
  }
  [data-testid="tour-signin"] a {
    text-decoration: underline;
  }
</style>
