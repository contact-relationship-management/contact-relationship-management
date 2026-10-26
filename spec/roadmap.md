# Roadmap

Beyond the v1 queue ([tasks.md](tasks.md)):

- **Real ESP adapter** — the campaign/nurture send seam gains an
  actual email-service-provider implementation (bounces,
  deliverability, webhooks) behind the existing trait.
- **Event-driven merge repointing** — consume upstream `merged`
  events on the durable bus to repoint contact/account wrappers
  automatically (manual endpoint in v1).
- **More attribution models** — time-decay and position-based, and
  per-person open and click touches (CRM-R49 has first, last and linear
  over e-mails sent, campaign leads and the recorded source).
- **Pause-on-pending SLA clocks** — business-hours calendars and
  holidays are built (CRM-R45); a clock that pauses while a ticket waits
  on the customer is not.
- **Public KB portal** + ticket deflection metrics.
- **Quotas & territories; commission views.**
- **CLV refinement** — margin, discounting, churn-adjusted models
  over the v1 revenue sum.
- **Cross-app bridges** — WPM (rep identity/teams from employee
  records), PPM (post-sale delivery projects opened from won
  deals).
- **FX beyond reports** — the report builder converts deal sums from
  hand-entered rates (CRM-R50); the forecast, campaign ROI, CLV and
  attribution still report per currency, and rates are not fetched
  from a provider or applied at each deal's close day.
