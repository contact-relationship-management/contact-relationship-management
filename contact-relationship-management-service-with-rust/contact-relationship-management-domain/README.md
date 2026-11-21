# Contact Relationship Management: domain

The business rules of the Contact Relationship Management service, and the dates and times they
reason about, with no web routes and no database storage (CRM-T260).

- `rules`: lifecycles (lead, deal, campaign, ticket, article), lead scoring with its breakdown,
  forecasts, ROI, CLV and win rates, attribution, SLA deadlines and breaches, segments with the
  consent gate, consent regimes, money and currency conversion, business calendars and periods,
  templates, webhooks, imports, reports and the rest. Each module is pure: no I/O, no SQL, no clock.
- `time`: `Timestamp` and `Date`, on `jiff`, which serialize as the API has always written them.
  With the `postgres` feature they also read and write Postgres's `timestamptz` and `date` (sqlx).

The service, `contact-relationship-management`, reads the database and the clock, calls these
rules, and stores and sends what they return. It re-exports both modules as `crate::rules` and
`crate::time`.

```bash
cargo test                      # the unit tests, without the service or a database
cargo bench --bench rules       # criterion benchmarks (BENCHMARKS.md)
```

Licence: MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR GPL-3.0-only OR OGL-UK-3.0 OR CC-BY-NC-SA-4.0, as the service.
