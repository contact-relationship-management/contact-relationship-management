# Benchmarks

Four sets, each with synthetic data only ([spec/testing.md](spec/testing.md), "Performance"):
the matcher and the service's rules (criterion, `scripts/ci-check.sh bench`), the API under load (k6), and the UI's browser
code. Each records its commit, machine, data size and command. They are readings from one
machine, not limits, and none is a CI gate.

## Matcher (CRM-T88)

`compare` is one pair; `compare_one_to_many` scores one contact against a list.
The contacts are synthetic, from a fixed seed, with every field filled in, so a
real table with sparse records will be a little faster.

| Case | Time (criterion's low, estimate, high) |
|---|---|
| `compare`, one pair | 2.59 µs, 2.68 µs, 2.80 µs |
| `compare_one_to_many`, 1,000 candidates | 1.62 ms, 1.79 ms, 2.05 ms |
| `compare_one_to_many`, 5,000 candidates | 7.28 ms, 7.31 ms, 7.35 ms |
| `compare_one_to_many`, 50,000 candidates | 73.4 ms, 74.0 ms, 74.8 ms |

It scales linearly, about 1.5 µs a candidate. The service's duplicates endpoint
compares at most 5,000 contacts (CRM-T61), which is about 7 ms; its time is
dominated by the person-service calls (up to 21 a request), not by the matcher.

- **Commit:** `512b5f5` plus the benchmark file.
- **Hardware:** Apple M4 Max, macOS 27.0.1, rustc 1.99.1, release profile, one run.
- **Command:** `cd contact-relationship-management-matcher-with-rust && cargo bench`
  (criterion 0.8; the benchmark is `benches/matching.rs`).
- **Limits:** one run on one machine, not a trend; the 1,000-candidate figure has
  the widest spread. No allocation or memory figures. Names are drawn from small
  lists, so many candidates share a name, which does not change the work per pair
  but is not a real distribution. Nothing here is a service-level figure.

## Service rules (CRM-T242)

The pure rules a request runs, in `contact-relationship-management-service-with-rust/contact-relationship-management-domain/benches/rules.rs` (in the service crate's `benches/` until CRM-T260):
no database, network or clock, so these are a request's rule step, not its latency. Times are
criterion's estimate (its low and high in brackets where the spread is wide).

| Case | 100 | 1,000 | 10,000 |
|---|---|---|---|
| `scoring::score`, a page of leads (one lead: 102 ns) | 19.1 µs (16.8–21.6) | 212 µs | 1.22 ms |
| `segment::matches`, a contact book, with the consent gate | 3.0 µs | 27.9 µs | 291 µs |
| `analytics::forecast_by_currency`, open deals in five currencies | 2.7 µs | 26.9 µs | 350 µs (311–396) |
| `audit_chain::verify`, the whole chain | 90.5 µs | 557 µs | 6.60 ms |
| `fx::convert_rows`, report rows into GBP, a year of daily rates | 61.2 µs | 179 µs | 1.22 ms |

| Case | Time |
|---|---|
| `imports`: parse a 500-row CSV, map it and judge every row | 203 µs |
| `attribution::credit`, 5 / 50 / 500 touches, first touch | 187 ns / 1.50 µs / 15.1 µs |
| the same, linear | 233 ns / 1.87 µs / 20.5 µs |
| the same, time decay | 345 ns / 5.58 µs / 78.3 µs |
| the same, position | 226 ns / 3.27 µs / 22.6 µs |
| `reports::plan`, a deal sum by state with a filter and dates | 1.67 µs |
| `reports::suppress`, 1,000 cells | 129 µs |
| `search` words and `tsquery`, Latin / Chinese | 747 ns / 911 ns |

What they say: every rule is linear in its input, and none costs more than a few milliseconds at
10,000 items. The audit check is the one that grows without bound, since the log only grows: at
about 0.66 µs a row it hashes a million rows in some 0.7 s of CPU before any database time. The
currency conversion's fixed cost (61 µs at 100 rows) is the search for each currency's rate in a
year of rates, done once per currency. The import figure is the CSV step only: the preview also
compares each row with known records using the matcher, which the k6 figures below include.

- **Commit:** `28c39a4` plus the benchmark file.
- **Hardware:** Apple M4 Max, macOS 27.2.1, rustc 1.100.0, the bench profile, one run.
- **Command:** `cd contact-relationship-management-service-with-rust/contact-relationship-management-domain && cargo bench --bench rules`
  (criterion 0.8; `-- segment` runs one group).
- **Limits:** one run on a laptop that was also running other work, so spreads are wide (the
  segment group was 21–31% faster on a second run minutes later; the figures above are the
  second run's); no allocation or memory figures; the inputs are drawn from short lists.

### More rules (CRM-T262)

Business calendars, templates, webhooks and the time types, which had no benchmark, in the same
file. Criterion's low, estimate and high.

| Case | Time |
| --- | --- |
| `calendar::add_working_minutes`, an SLA deadline 4 working hours on (London, two windows a weekday, eight holidays) | 159 ns, 161 ns, 164 ns |
| the same, 5 working days on | 977 ns, 986 ns, 996 ns |
| the same, 40 working days on | 6.69 µs, 6.74 µs, 6.79 µs |
| `calendar::working_seconds_between`, 1 day | 295 ns, 297 ns, 299 ns |
| the same, 30 days | 5.13 µs, 5.86 µs, 6.65 µs |
| the same, 365 days | 44.9 µs, 50.7 µs, 57.5 µs |
| `templates::parse`, a 1 KB mail with five tokens | 62 ns, 64 ns, 66 ns |
| `templates::render`, the same for one recipient (subject, text and HTML) | 1.65 µs, 1.84 µs, 2.08 µs |
| `webhooks::sign`, a 2 KB body (HMAC-SHA256) | 1.29 µs, 1.30 µs, 1.31 µs |
| `webhooks::check_target`, one https address | 230 ns, 244 ns, 260 ns |
| `webhooks::is_public_ip`, four addresses | 8.2 ns, 8.3 ns, 8.5 ns |
| `time::Timestamp` read as `chrono` read it, with an offset | 66 ns, 76 ns, 88 ns |
| the same, strict RFC 3339 | 30 ns, 35 ns, 41 ns |
| `Timestamp` written as `chrono` wrote it | 164 ns, 176 ns, 197 ns |
| `time::Date` read | 17.2 ns, 17.3 ns, 17.4 ns |

What they say: a calendar walks day by day, so its cost grows with the span, about 170 ns a
working day for a deadline and 140 ns a calendar day for a sum; an SLA deadline of a few days is
about a microsecond. A campaign of 10,000 recipients spends about 18 ms rendering. Reading and
writing times, which every request and response does, costs well under a microsecond each.

- **Commit:** `a1d9825` plus the benchmark additions; the bench is now in the domain crate (CRM-T260).
- **Hardware:** Apple M4 Max, macOS 27.2.1, rustc 1.100.0, the bench profile, one run, with a load
  average of 26 to 33 from other work on the machine.
- **Command:** `cd contact-relationship-management-service-with-rust/contact-relationship-management-domain && cargo bench --bench rules -- '^(calendar|templates|webhooks|time)/'`.
- **Limits:** one run on a shared machine; the first run of `templates/parse` read 172 ns and the
  second 64 ns, so its spread is wide; one calendar shape only.

## Rerun of both criterion suites (CRM-T266)

Every criterion case, rerun through `scripts/ci-check.sh bench` after the move to axa 0.20 and jiff
1.0 (CRM-T264, CRM-T265), and kept as the baseline `crm-t266`. "Earlier" is the estimate in the
tables above, from the commits they name; "now" is criterion's estimate, with its low and high.

| Case | Earlier | Now (low – high) |
| --- | --- | --- |
| `compare one pair` | 2.68 µs | 5.9769 µs (4.8939 µs – 7.1518 µs) |
| `compare_one_to_many/1000` | 1.79 ms | 2.6893 ms (2.2447 ms – 3.4456 ms) |
| `compare_one_to_many/5000` | 7.31 ms | 16.351 ms (11.934 ms – 23.138 ms) |
| `compare_one_to_many/50000` | 74.0 ms | 94.877 ms (81.768 ms – 114.02 ms) |
| `scoring/score/one` | 102 ns | 328.90 ns (239.63 ns – 427.81 ns) |
| `scoring/score/100` | 19.1 µs | 16.566 µs (13.745 µs – 20.022 µs) |
| `scoring/score/1000` | 212 µs | 159.27 µs (129.73 µs – 192.33 µs) |
| `scoring/score/10000` | 1.22 ms | 1.1051 ms (1.0580 ms – 1.1575 ms) |
| `segment/matches/100` | 3.0 µs | 5.7906 µs (5.0038 µs – 6.5998 µs) |
| `segment/matches/1000` | 27.9 µs | 30.849 µs (28.401 µs – 33.566 µs) |
| `segment/matches/10000` | 291 µs | 423.29 µs (368.71 µs – 484.44 µs) |
| `forecast/by_currency/100` | 2.7 µs | 2.5555 µs (2.5391 µs – 2.5756 µs) |
| `forecast/by_currency/1000` | 26.9 µs | 26.621 µs (26.427 µs – 26.828 µs) |
| `forecast/by_currency/10000` | 350 µs | 277.25 µs (271.63 µs – 283.54 µs) |
| `audit_chain/verify/100` | 90.5 µs | 52.568 µs (52.255 µs – 52.893 µs) |
| `audit_chain/verify/1000` | 557 µs | 559.18 µs (551.55 µs – 568.41 µs) |
| `audit_chain/verify/10000` | 6.60 ms | 6.0205 ms (5.7630 ms – 6.3175 ms) |
| `imports/parse_and_judge/500` | 203 µs | 198.68 µs (191.66 µs – 207.64 µs) |
| `attribution/credit/first/5` | 187 ns | 254.88 ns (229.54 ns – 284.68 ns) |
| `attribution/credit/linear/5` | 233 ns | 540.04 ns (430.03 ns – 665.83 ns) |
| `attribution/credit/time_decay/5` | 345 ns | 354.52 ns (315.83 ns – 395.51 ns) |
| `attribution/credit/position/5` | 226 ns | 191.82 ns (188.82 ns – 195.73 ns) |
| `attribution/credit/first/50` | 1.50 µs | 1.2571 µs (1.2199 µs – 1.3078 µs) |
| `attribution/credit/linear/50` | 1.87 µs | 1.7377 µs (1.6037 µs – 1.9105 µs) |
| `attribution/credit/time_decay/50` | 5.58 µs | 5.7017 µs (5.1977 µs – 6.3627 µs) |
| `attribution/credit/position/50` | 3.27 µs | 2.8712 µs (2.8400 µs – 2.9068 µs) |
| `attribution/credit/first/500` | 15.1 µs | 11.152 µs (10.863 µs – 11.520 µs) |
| `attribution/credit/linear/500` | 20.5 µs | 12.858 µs (12.817 µs – 12.907 µs) |
| `attribution/credit/time_decay/500` | 78.3 µs | 52.143 µs (51.946 µs – 52.378 µs) |
| `attribution/credit/position/500` | 22.6 µs | 16.821 µs (16.651 µs – 17.015 µs) |
| `fx/convert_rows/100` | 61.2 µs | 69.345 µs (63.731 µs – 76.309 µs) |
| `fx/convert_rows/1000` | 179 µs | 153.01 µs (151.28 µs – 155.39 µs) |
| `fx/convert_rows/10000` | 1.22 ms | 1.1403 ms (1.1132 ms – 1.1754 ms) |
| `reports/plan` | 1.67 µs | 1.0599 µs (1.0530 µs – 1.0673 µs) |
| `reports/suppress/1000` | 129 µs | 104.81 µs (103.72 µs – 106.75 µs) |
| `search/tsquery/latin` | 747 ns | 656.71 ns (647.60 ns – 669.18 ns) |
| `search/tsquery/cjk` | 911 ns | 758.13 ns (751.53 ns – 767.23 ns) |
| `calendar/add_working_minutes/4h` | 161 ns | 154.75 ns (153.00 ns – 157.11 ns) |
| `calendar/add_working_minutes/5d` | 986 ns | 1.1199 µs (1.0387 µs – 1.2223 µs) |
| `calendar/add_working_minutes/40d` | 6.74 µs | 6.6972 µs (6.5770 µs – 6.8883 µs) |
| `calendar/working_seconds_between/1d` | 297 ns | 423.32 ns (364.78 ns – 486.57 ns) |
| `calendar/working_seconds_between/30d` | 5.86 µs | 4.8108 µs (4.2148 µs – 5.4928 µs) |
| `calendar/working_seconds_between/365d` | 50.7 µs | 76.714 µs (62.510 µs – 91.591 µs) |
| `templates/parse` | 64 ns | 91.512 ns (84.069 ns – 99.691 ns) |
| `templates/render` | 1.84 µs | 1.4690 µs (1.4298 µs – 1.5171 µs) |
| `webhooks/sign/2KB` | 1.30 µs | 1.6310 µs (1.5392 µs – 1.7277 µs) |
| `webhooks/check_target` | 244 ns | 463.32 ns (403.32 ns – 532.49 ns) |
| `webhooks/is_public_ip/4` | 8.3 ns | 9.9538 ns (9.2540 ns – 10.849 ns) |
| `time/timestamp/read` | 76 ns | 62.770 ns (56.030 ns – 70.476 ns) |
| `time/timestamp/read_rfc3339` | 35 ns | 18.608 ns (18.484 ns – 18.750 ns) |
| `time/timestamp/write` | 176 ns | 160.94 ns (159.16 ns – 163.09 ns) |
| `time/date/read` | 17.3 ns | 17.342 ns (16.959 ns – 17.900 ns) |

What they say: little that can be trusted to a few tens of percent. The machine's load average was
31 to 37 from other work for the whole run, against 26 to 33 for CRM-T262's and less for the
earlier ones, and the cases with the widest spreads (the matcher's, one lead's score, linear
attribution at 5 touches, `webhooks::check_target`) are the ones that read slower. Cases with
narrow spreads read about the same or faster: the forecast, the audit check, reports, search and
time decay at 500 touches (52 µs, from 78 µs). Strict RFC 3339 reads in 19 ns, from 35 ns, which
may be jiff 1.0 or the quieter moment; this run cannot say which. The matcher, which neither
move touches, reads about twice as slow, which shows the size of the load's effect. Nothing reads
as a regression from the dependency moves.

- **Commit:** `f30de40` (jiff 1.0.0) plus the `bench` stage; criterion 0.8.2.
- **Hardware:** Apple M4 Max, macOS 27.2.1, rustc 1.100.0, the bench profile, one run, load average
  31 to 37.
- **Command:** `CONTACT_RELATIONSHIP_MANAGEMENT_BENCH_SAVE=crm-t266 scripts/ci-check.sh bench` (615 s). A later run compares with
  it by `CONTACT_RELATIONSHIP_MANAGEMENT_BENCH_BASELINE=crm-t266 scripts/ci-check.sh bench`, on this machine only: the baseline
  is in each crate's `target/criterion`, which is not committed.
- **Limits:** one run on a loaded machine; the earlier figures came from different commits and loads,
  so a difference under about 30% is not a finding.

## API under load (CRM-T242)

k6 1.4.2 (`tests/load/`), from a container on the demo stack's own network, against
`podman compose up --build` at `28c39a4` (sign-in off, simulated mail, PostgreSQL 18 and Redis 8 in
the same Podman VM, 8 CPUs and 32 GiB), after `seed.test.js` made 1,000 contacts with an activity
and tags each, and accounts, leads, deals and tickets around them (4,742 requests, none failed).
The database started empty; each endpoint run adds about 320 contacts of its own (half of its
600 contact writes, and 20 in its setup), and the figures below are from the second run, so it
held roughly 1,350 contacts at its start and 1,650 at its end. They were not counted: the demo keeps its data in an anonymous
volume, which `podman compose down` removes. k6, the service, the database and the laptop's
other work shared the machine.

**Endpoints** (`endpoints.test.js`, each group alone at 20 requests a second for 30 s):

| Group | p50 ms | p95 ms | p99 ms | max ms | Failed | p95 limit ms | |
| --- | --- | --- | --- | --- | --- | --- | --- |
| lists | 2.7 | 4.5 | 6.2 | 39.8 | 0% | 500 | pass |
| search | 78.9 | 141.0 | 145.4 | 153.2 | 0% | 500 | pass |
| contact reads | 2.6 | 3.6 | 4.4 | 25.7 | 0% | 300 | pass |
| contact writes | 5.5 | 13.2 | 19.7 | 26.1 | 0% | 500 | pass |
| other writes | 5.6 | 16.3 | 24.7 | 35.3 | 0% | 500 | pass |
| rollups (forecast, attribution, dashboards, insights, funnel, quotas) | 4.6 | 12.1 | 19.7 | 36.3 | 0% | 1,000 | pass |
| reports | 6.6 | 15.7 | 27.6 | 70.4 | 0% | 1,000 | pass |
| **segment previews** | **7,019** | **9,722** | **10,226** | **11,308** | **87.7%** | 1,000 | **fail** |
| duplicate checks | 7.0 | 19.2 | 29.0 | 117.1 | 0% | 2,000 | pass |
| lead-import previews, 200 rows | 9.1 | 10.6 | 15.6 | 47.7 | 0% | 1,000 | pass |
| audit-chain check | 25.0 | 30.9 | 38.7 | 57.2 | 0% | 2,000 | pass |

**Segment previews fail.** One preview on an idle service took 0.35–0.44 s with roughly 1,300
contacts, though it matched none (no synthetic contact has granted marketing consent): it reads
every contact, then makes four queries for each on one connection (`evaluate_segment`). Twenty a
second need more connections than the pool holds, so requests wait 5 s for one and are refused
with `503` ("pool timed out while waiting for an open connection"). The cost grows with the number
of contacts, and a campaign run evaluates its segment the same way. Fixed in CRM-T243: see below.

**Lead-import previews** depend on the names: with made-up pairs from short lists, as the script
now sends, a 200-row preview takes about 10 ms. In a first run every name shared the word "Load"
with each other and with every seeded record, so each row was compared with its 200-candidate
cap: a single such preview took about 60 ms, and at 20 a second they queued to a p50 of 3.7 s.

**Search** is the slowest of the plain reads (p50 79 ms, up from 67 ms in the first run with
about 340 fewer contacts), and is the read to watch as the data grows.

**Mixed workload:** `basic-load.test.js`, 10 users for 1 minute: 427 requests, none failed, p50
2.6 ms, p95 116 ms, p99 423 ms. **Breakpoint:** `breakpoint.test.js` with `MAX_RATE=100` over 3
minutes stopped at about 72 iterations a second, when p95 passed 2 s (2.23 s; no request failed).
Near the end every route was slow, not one, so the limit is the service as a whole on this machine;
segment previews (2% of the mix) were the slowest.

**After CRM-T243** (segment facts in four set queries, not four a contact), at the commit that
fixed it, on a fresh demo database seeded with 1,000 contacts (4,764 requests, none failed); the
endpoint run then held roughly 1,000 to 1,320 contacts. The laptop was quieter than in the run
above, so every group was faster, not only segments:

| Group | p50 ms | p95 ms | p99 ms | max ms | Failed | p95 limit ms | |
| --- | --- | --- | --- | --- | --- | --- | --- |
| lists | 1.5 | 2.3 | 8.4 | 20.0 | 0% | 500 | pass |
| search | 29.9 | 84.6 | 103.7 | 129.7 | 0% | 500 | pass |
| contact reads | 2.3 | 3.2 | 8.5 | 46.8 | 0% | 300 | pass |
| contact writes | 1.8 | 2.1 | 2.5 | 29.9 | 0% | 500 | pass |
| other writes | 1.6 | 2.2 | 2.5 | 3.3 | 0% | 500 | pass |
| rollups | 1.4 | 2.3 | 3.0 | 6.0 | 0% | 1,000 | pass |
| reports | 1.8 | 2.3 | 2.9 | 49.0 | 0% | 1,000 | pass |
| **segment previews** | **4.6** | **5.8** | **13.1** | **26.2** | **0%** | 1,000 | **pass** |
| duplicate checks | 4.8 | 5.5 | 7.8 | 51.0 | 0% | 2,000 | pass |
| lead-import previews, 200 rows | 6.9 | 7.9 | 8.7 | 40.9 | 0% | 1,000 | pass |
| audit-chain check | 16.4 | 19.2 | 21.0 | 50.8 | 0% | 2,000 | pass |

One preview on an idle service: 6 ms, against 0.35–0.44 s before. The breakpoint test to 200 a
second over 3 minutes stopped at about 140 iterations a second (p95 2.15 s; none of 8,805
requests failed), against about 72 before; near the end every route was slow together again, so
the limit is the service as a whole on this machine.

**The other load tests**, on a fresh demo seeded with 500 contacts at `fcbd054`, at small sizes:
stress (`VUS=3`, 30 s steps) 1,725 requests, p95 40 ms; spike (`VUS=2`, a 15 s spike) 643
requests, p95 31 ms; soak (`VUS=3`, 2 minutes) 263 requests, p95 38 ms. None failed a request or
a check.

**A campaign run**, simulated mail, to 1,000 consented synthetic contacts in one segment: one
`POST /api/campaigns/{pid}/run` of 2.76 s sent all 1,000 (none left). The segment step is 8.6 ms
of it; the rest is the per-contact work done on purpose: consent read again from the live contact
just before each send, the delivery row claimed and the touch recorded (CRM-T39), about 2.8 ms a
contact. With real mail the run is a background job, batch by batch, and the vendor's time comes
on top. A one-off script, not kept in `tests/load/`, as it grants consent to synthetic contacts.

- **Commands:** from `tests/load/`, `podman run --rm -i --network contact-relationship-management_default
  -v "$PWD:/work:Z" -w /work <K6_IMAGE> run -e TEST_URL=http://api:1241 scripts/<test>.test.js`,
  with `-e DURATION=1m` for basic and `-e MAX_RATE=100 -e DURATION=3m` for breakpoint. The
  container network was used because another service held port 1241 on the host; with the port
  free, `make <test> K6=podman-k6` does the same.
- **Limits:** one run of each; nothing with sign-in on; stress, spike and soak were checked with
  `k6 inspect`, not run; no run at the scripts' default sizes for basic or breakpoint.

## UI browser code (CRM-T242)

`pnpm build && pnpm size` in `contact-relationship-management-ui-with-svelte` at `28c39a4`
(`scripts/bundle-size.mjs`, gzip level 9):

| What | Files | Raw KiB | Gzip KiB |
| --- | --- | --- | --- |
| JavaScript, all files | 83 | 1,108.1 | 372.0 |
| CSS, all files | 17 | 25.2 | 7.3 |
| Every page (SvelteKit's start and app entries and the root layout) | 3 | 65.5 | 19.8 |

The largest single file is 20.5 KiB gzipped. A page's own node and the chunks it imports come on
top of the 19.8 KiB every page loads; the script does not trace those, and no load time was
measured in a browser.

## Page load in a browser (CRM-T262)

`pnpm perf:pages` in `contact-relationship-management-ui-with-svelte` (`tests/perf/page-load.spec.ts`,
its own Playwright config, not part of the e2e suite): eleven pages of the tour, signed in as the demo
user, against the production build served by `vite preview` with the BFF's demo data. Each page is
loaded three times in a new browser context (no cache), keeping the median, and once more warm (a
reload). Times are milliseconds from the start of the navigation; LCP is the largest contentful paint.

**Plain** (loopback, no throttling):

| Page | TTFB | DOM ready | Load | LCP | Warm load | Requests | JS KiB |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `/` | 2 | 43 | 48 | 88 | 15 | 37 | 88.1 |
| `/contacts` | 2 | 40 | 46 | 84 | 20 | 38 | 130.5 |
| `/contacts/con-1` | 2 | 39 | 45 | 104 | 12 | 41 | 134.3 |
| `/accounts` | 1 | 39 | 47 | 84 | 10 | 38 | 130.3 |
| `/leads` | 2 | 41 | 45 | 88 | 18 | 38 | 130.7 |
| `/leads/board` | 2 | 40 | 45 | 88 | 11 | 33 | 123.1 |
| `/deals` | 2 | 46 | 53 | 92 | 13 | 42 | 150.6 |
| `/campaigns` | 2 | 40 | 48 | 108 | 10 | 39 | 156.7 |
| `/tickets` | 2 | 40 | 45 | 88 | 10 | 38 | 130.9 |
| `/executive` | 1 | 40 | 49 | 92 | 11 | 36 | 127.7 |
| `/search?q=a` | 2 | 41 | 45 | 84 | 8 | 29 | 85.0 |

**A mid-range phone** (Chrome's CPU slowed 4 times; 9 Mbit/s down, 1.5 Mbit/s up, 170 ms round trip):

| Page | TTFB | DOM ready | Load | LCP | Warm load | Requests | JS KiB |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `/` | 171 | 574 | 1,165 | 1,412 | 213 | 37 | 88.1 |
| `/contacts` | 171 | 577 | 1,316 | 1,572 | 224 | 38 | 130.5 |
| `/contacts/con-1` | 171 | 574 | 1,300 | 1,668 | 215 | 41 | 134.3 |
| `/accounts` | 171 | 585 | 1,320 | 1,596 | 216 | 38 | 130.3 |
| `/leads` | 171 | 598 | 1,329 | 1,604 | 222 | 38 | 130.7 |
| `/leads/board` | 171 | 605 | 1,184 | 1,452 | 224 | 33 | 123.1 |
| `/deals` | 171 | 599 | 1,369 | 1,644 | 447 | 40 | 150.6 |
| `/campaigns` | 171 | 575 | 1,317 | 1,804 | 216 | 39 | 156.7 |
| `/tickets` | 171 | 576 | 1,316 | 1,572 | 215 | 38 | 130.9 |
| `/executive` | 171 | 581 | 1,159 | 1,460 | 216 | 35 | 127.7 |
| `/search?q=a` | 171 | 587 | 1,124 | 1,420 | 204 | 29 | 85.0 |

What they say: on the throttled phone every page paints its largest element in 1.4 to 1.8 s, under
the 2.5 s that Core Web Vitals counts as good, and a warm reload takes about 0.2 s. About 30 to 40
requests and 85 to 157 KiB of JavaScript (as sent: `vite preview` does not compress) load before
the page is usable; the campaigns and deals pages carry the most. The server's own time is a
millisecond or two here, because the demo data is in memory: with a real service, its API time
(see above) comes before the first byte.

- **Commit:** `a1d9825` plus the benchmark script.
- **Hardware:** Apple M4 Max, macOS 27.2.1, Node 26.8.1, Playwright 1.65.1 (Chromium), load average
  about 26 to 30 from other work.
- **Command:** `cd contact-relationship-management-ui-with-svelte && pnpm perf:pages`; the figures are
  also written to `test-results/page-load.json`.
- **Limits:** loopback, so the plain run has no network; the phone profile is Chrome's throttling of
  this machine, not a phone; demo data in the UI's server, not the service; the transfer sizes are
  uncompressed, which a deployment behind a compressing proxy would cut; three cold runs a page.

## Not yet measured

- Request latency against a database of realistic size (tens of thousands of contacts). The demo
  stack's `podman compose` does not start on this machine's Podman 6.2.0 (CRM-T254), so it was not
  run here.
- Page load against a real service, and with a deployment's compression.
- Anything with sign-in on, or with real mail.

## Method

Each result should record the commit, hardware, dataset size (synthetic), tool
and command used, and the raw output, so that it can be reproduced.
