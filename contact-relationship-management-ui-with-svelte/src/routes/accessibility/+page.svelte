<svelte:head><title>Accessibility statement · CRM</title></svelte:head>

<!--
  Accessibility statement (CRM-T45). Written in English only: it is a
  statement of fact about this software's state, not interface copy, and it
  must not claim more than has been tested. Update it whenever an audit
  changes the facts, and set the review date.
-->
<article lang="en">
  <h1>Accessibility statement</h1>

  <p>
    This statement covers the Contact Relationship Management web application.
    It is a demonstration application and has not been deployed to production.
    An organisation that deploys it is responsible for publishing its own
    statement for its own service, and may adapt this one.
  </p>

  <h2>Compliance status</h2>
  <p>
    The target is the Web Content Accessibility Guidelines (WCAG) 2.2 at level AA.
    The application is <strong>not yet shown to comply</strong>. No full manual
    audit has been done, so we do not claim partial or full compliance.
  </p>

  <h2>What has been tested</h2>
  <ul>
    <li>
      Automated checks with axe-core against the WCAG 2.0, 2.1 and 2.2 A and AA
      rules run on every change, on the home, tour, sign-in, contacts, accounts,
      leads, tickets, articles, campaigns and emergency-access pages. They
      currently report no violations.
    </li>
  </ul>

  <h2>What has not been tested</h2>
  <ul>
    <li>
      Manual review with a keyboard only, with screen readers (NVDA, JAWS,
      VoiceOver), with browser zoom to 400%, or with speech-recognition software.
    </li>
    <li>
      Pages that need data to render: the deal board, forecast, dashboards,
      follow-up calendar, and detail pages.
    </li>
    <li>
      Automated tools find only part of the problems that affect people, so a
      clean automated result is not proof of accessibility.
    </li>
  </ul>

  <h2>Known or likely problems</h2>
  <ul>
    <li>
      The deal, lead and ticket boards are built around dragging cards between
      columns. We have not verified that every move can be made with a keyboard
      or assistive technology. Where a status can be changed another way, such as
      a button on the ticket queue, use that.
    </li>
    <li>
      Some third-party components (the data grid, kanban board and calendar) may
      not meet every success criterion. We have not audited them.
    </li>
    <li>
      The interface is translated into eight languages by machine-assisted
      translation that has not been reviewed by fluent speakers.
    </li>
  </ul>

  <h2>Feedback and contact</h2>
  <p>
    If you find an accessibility problem, or need information in a different
    format, email
    <a href="mailto:joel@joelparkerhenderson.com">joel@joelparkerhenderson.com</a>.
    Please say which page, which browser and assistive technology you used, and
    what went wrong. We aim to reply within 5 working days.
  </p>

  <h2>Enforcement</h2>
  <p>
    If you are in the United Kingdom and are not happy with our response, contact
    the Equality Advisory and Support Service (EASS). In Northern Ireland, contact
    the Equality Commission for Northern Ireland.
  </p>

  <h2>Preparation of this statement</h2>
  <p>
    This statement was prepared on 10 October 2026 from the automated test
    results and a review of the source. It will be reviewed when an audit is
    completed and at least once a year.
  </p>
</article>

<style>
  /* Links inside running text must not rely on colour alone (WCAG 1.4.1). */
  article p a,
  article li a {
    text-decoration: underline;
  }
</style>
