# Roadmap

Beyond the v1 queue ([tasks.md](tasks.md)):

- **Real ESP adapter** — the campaign/nurture send seam gains an
  actual email-service-provider implementation (bounces,
  deliverability, webhooks) behind the existing trait.
- **Event-driven merge repointing** — consume upstream `merged`
  events on the durable bus to repoint contact/account wrappers
  automatically (manual endpoint in v1).
- **More attribution models** — per-person open and click touches, and
  models with settings the caller can choose (CRM-R49 and CRM-R56 have
  first, last, linear, time-decay and position-based over e-mails sent,
  campaign leads and the recorded source).
- **SLA clock refinements** — business-hours calendars and holidays
  (CRM-R45) and a clock that pauses while a ticket waits on the
  customer (CRM-R55) are built; a limit on how long a ticket may wait,
  and pausing on a customer reply, are not.
- **Help centre refinements** — a public help centre and a deflection estimate are built (CRM-R57); suggesting articles as a ticket is typed, deflection tied to visits, and a limit shared by all clients are not.
- **Quotas & territories; commission views.**
- **CLV refinement** — margin, discounting, churn-adjusted models
  over the v1 revenue sum.
- **Cross-app bridges** — WPM (rep identity/teams from employee
  records), PPM (post-sale delivery projects opened from won
  deals).
- **FX beyond reports** — the report builder, attribution, campaign ROI,
  the live forecast and CLV convert from hand-entered rates (CRM-R50 to
  CRM-R54); the forecast snapshots and the trend still report per
  currency, and rates are not fetched
  from a provider or applied at each deal's close day.
