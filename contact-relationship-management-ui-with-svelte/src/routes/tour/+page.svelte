<!--
  Tour (/tour) — a public, in-depth walkthrough of the CRM. Copy lives in
  the i18n catalog (`tour.*`); this file only declares which app screen
  each workflow section links to, in order.
-->
<script lang="ts">
  import Tour from "#lib/components/Tour.svelte";
  import { t } from "#lib/i18n.svelte.js";

  const sections = [
    { href: "/contacts" },
    { href: "/leads" },
    { href: "/deals" },
    { href: "/campaigns" },
    { href: "/tickets" },
    { href: "/executive" },
  ];
</script>

<p class="panel" data-testid="tour-signin">
  {t("signin.demo")} <code>example@example.com</code> —
  <a href="/signin">{t("auth.signin")}</a>
</p>

<Tour {sections} />

<style>
  [data-testid="tour-signin"] a {
    text-decoration: underline;
  }
</style>
