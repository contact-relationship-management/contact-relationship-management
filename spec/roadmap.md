# Roadmap

Beyond the v1 queue ([tasks.md](tasks.md)):

- **Real ESP adapter** — the campaign/nurture send seam gains an
  actual email-service-provider implementation (bounces,
  deliverability, webhooks) behind the existing trait.
- **Event-driven merge repointing** — consume upstream `merged`
  events on the durable bus to repoint contact/account wrappers
  automatically (manual endpoint in v1).
- **More attribution models** — models with settings the caller can
  choose, and ways to discount machine opens (CRM-R49, CRM-R56 and
  CRM-R58 have first, last, linear, time-decay and position-based over
  e-mails sent, opened and clicked as a provider reports them, campaign
  leads and the recorded source).
- **SLA clock refinements** — business-hours calendars and holidays
  (CRM-R45) and a clock that pauses while a ticket waits on the
  customer (CRM-R55) are built; a limit on how long a ticket may wait,
  and pausing on a customer reply, are not.
- **Help centre refinements** — a public help centre and a deflection estimate are built (CRM-R57); suggesting articles as a ticket is typed, deflection tied to visits, and a limit shared by all clients are not.
- **Territories; quota refinements** — quotas, tiered commission plans and attainment are built (CRM-R59); territories, team roll-ups, splitting a deal and clawbacks are not.
- **CLV refinement** — margin and a discounted, retention-based look-ahead
  are built (CRM-R60); a retention taken from history, converting them,
  and a page to show them are not.
- **Cross-app bridges** — WPM (rep identity/teams from employee
  records), PPM (post-sale delivery projects opened from won
  deals).
- **FX beyond reports** — the report builder, attribution, campaign ROI,
  the live forecast and CLV convert from hand-entered rates (CRM-R50 to
  CRM-R54); the forecast snapshots and the trend still report per
  currency, and rates are not fetched
  from a provider or applied at each deal's close day.
