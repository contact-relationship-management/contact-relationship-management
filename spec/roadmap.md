# Roadmap

Beyond the v1 queue ([tasks.md](tasks.md)):

- **Real ESP adapter** — the campaign/nurture send seam gains an
  actual email-service-provider implementation (bounces,
  deliverability, webhooks) behind the existing trait.
- **Event-driven merge repointing** — consume upstream `merged`
  events on the durable bus to repoint contact/account wrappers
  automatically (manual endpoint in v1).
- **More attribution models** — CRM-R49, CRM-R56 and CRM-R58 have first,
  last, linear, time-decay and position-based over e-mails sent, opened
  and clicked as a provider reports them, campaign leads and the recorded
  source; the time-decay half-life can be chosen and opens that come
  seconds after sending can be left out (CRM-R89); each end of the
  position model's share can be chosen (CRM-R90). Weights for the touches
  between, discounting rather than leaving out, and a delay for clicks are
  not built.
- **SLA clock refinements** — business-hours calendars and holidays
  (CRM-R45) and a clock that pauses while a ticket waits on the
  customer (CRM-R55) are built, with a customer reply resuming it
  (CRM-R63); a limit on how long a ticket may wait (CRM-R64) is built, with resolving or
  escalating on it (CRM-R69); closing a ticket left resolved is built (CRM-R82); a reply by e-mail
  resumes a waiting ticket (CRM-R88, only for a message sent in the service's
  own two-member shape, checked against no mail service); a vendor's own
  inbound format, and a reply on a channel other than e-mail, are not.
- **Help centre refinements** — a public help centre and a deflection estimate are built (CRM-R57); articles suggested for an existing ticket are built (CRM-R66); suggesting as a ticket is typed, deflection tied to visits, and a limit shared by all clients are not.
- **Territories; quota refinements** — quotas, tiered commission plans and attainment are built (CRM-R59); splitting a deal between workers is built (CRM-R65); team roll-ups are built (CRM-R67); territories of named accounts are built (CRM-R68); clawing credit back from a worker is built (CRM-R71); clawing back commission directly is not.
- **CLV refinement** — margin and a discounted, retention-based look-ahead
  are built (CRM-R60) and shown on the account page, with the retention
  readable from the account's own history (CRM-R61) and conversion to
  one currency (CRM-R62), at the rate of the day each deal closed if asked
  (CRM-R70, for the total alone).
- **Cross-app bridges** — a won deal opens a delivery idea in PPM
  (CRM-R72) and WPM's departments become teams (CRM-R73), both stub-first
  and checked against stand-ins only; projects in PPM (it has plans and
  ideas), teams by manager, rep identity beyond ids, and running either
  on a schedule or on an event are not built.
- **FX beyond reports** — the report builder, attribution, campaign ROI,
  the live forecast and CLV convert from hand-entered rates (CRM-R50 to
  CRM-R54); the forecast snapshots and the trend still report per
  currency, and rates are not fetched
  from a provider. Each deal's close day is applied to an account's CLV
  (CRM-R70), to the report builder (CRM-R83), to attribution (CRM-R85) and
  to a campaign's ROI (CRM-R86), not yet to the forecast.
