# Load tests

[k6](https://k6.io/) scripts that put the CRM API under load: smoke, basic, stress, spike, soak and
breakpoint, a seed that fills a database first, and an endpoint benchmark that measures each group of
routes on its own. They call the API's real routes with its real request bodies, through one module,
`lib/crm.js`. The CPU cost of the rules behind those routes is measured apart, with criterion
([`BENCHMARKS.md`](../../BENCHMARKS.md)).

For a run of many hours that compares the start with the end and probes health on its own, use
[`tests/long-running/`](../long-running/README.md).

## Before you run them

- **Use a throwaway database.** The tests write synthetic contacts, leads, deals, activities and a
  pipeline, and delete nothing: the API has no contact delete, as erasure is a privacy operation.
  From the repository root, `podman compose up --build` starts the demo at `http://localhost:1241`
  with sign-in off.
- **Never point them at production**, or at a service that holds real personal data.
- The data is synthetic. A contact holds a made-up `person:<uuid>` reference and a display name such
  as `Load 1a2b3c4d`, never a copied name, email, phone or address ([scope](../../spec/scope.md)).

## Running

```bash
cd tests/load
make smoke K6=podman-k6           # one user, each call once; run this first
make basic K6=podman-k6           # 10 users for 5 minutes
make stress VUS=5 DURATION=1m     # steps of 5, 10 and 20 users, 1 minute each
make quick                        # smoke, then one minute of basic
make seed SEED=2000               # fill the database first, so lists and rollups have rows
make endpoints RATE=20            # each group of routes on its own, a table per group
make bench                        # seed, then endpoints
make breakpoint MAX_RATE=300      # a rising rate until the service fails its limits
make help
```

`K6=podman-k6` runs the pinned k6 image with Podman, so k6 need not be installed; without it, the
`k6` on your `PATH` runs. `TEST_URL` names another service and `AUTH_TOKEN` gives a bearer token for
one with sign-in on. From `tests/`, `make load-smoke` and `make load-quick` do the same.

Each run writes `results/<test>-summary.json` and, for runs longer than a few seconds, k6's HTML
report `results/<test>.html`. The endpoint benchmark also writes its table to `results/endpoints.md`.

GitHub Actions has a manual workflow, **Load Test** (`.github/workflows/load-test.yml`), that runs one
script against a URL you give it and uploads the results.

## The tests

| Test | Script | Load | Defaults | Fails when |
| --- | --- | --- | --- | --- |
| Smoke | `smoke.test.js` | One user makes every call in `lib/crm.js` once per iteration | 3 iterations (`ITERATIONS`) | any call fails, or p95 is 1 s or more |
| Basic | `basic-load.test.js` | A steady number of users on the mixed workload | `VUS=10`, `DURATION=5m` | over 1% of requests or checks fail, p95 ≥ 500 ms, p99 ≥ 1.5 s |
| Stress | `stress.test.js` | Steps of one, two and four times `VUS`, less pause | `VUS=10`, `DURATION=2m` per step | over 5% fail, p95 ≥ 2 s |
| Spike | `spike.test.js` | `VUS`, then ten times as many within 10 s, then back | `VUS=5`, `DURATION=1m` of spike | over 5% fail, p95 ≥ 3 s |
| Soak | `soak.test.js` | The mixed workload for hours | `VUS=10`, `DURATION=2h` | as basic |
| Breakpoint | `breakpoint.test.js` | The mixed workload, no pause, at a rate rising from 1 a second to `MAX_RATE` | `MAX_RATE=200`, `DURATION=10m` | stops the run once over 5% fail or p95 ≥ 2 s; the rate reached is the result |
| Seed | `seed.test.js` | `SEED` contacts, each with an activity and tags, and accounts, leads, deals and tickets in proportion | `SEED=1000`, `VUS=5` | over 1% fail (a setup step, not a test of speed) |
| Endpoints | `endpoints.test.js` | Eleven groups of routes, one after another, each at `RATE` requests a second | `RATE=20`, `DURATION=30s` per group | a group's p95 is over its limit, or over 1% of its requests fail |

The mixed workload (`WORKLOAD` in `lib/crm.js`) is mostly reads, as a CRM's use is:

| Share | Call |
| --- | --- |
| 40% | a list of contacts, accounts, leads, deals, activities or tickets |
| 8% | `/api/search` |
| 8% | a rollup: the forecast, attribution, a dashboard, an insight, the pipeline's funnel or quota attainment |
| 7% | one contact this user made |
| 3% | one account this user made |
| 2% | an ad hoc report (`/api/reports/run`), one in five converted to GBP |
| 2% | the run's segment, previewed through the consent gate |
| 1% | the duplicate check on one contact |
| 6% | a new contact |
| 4% | an update to one of this user's contacts |
| 4% | an activity on one of them |
| 2% | new tags on one of them |
| 3% | a new lead |
| 2% | a new account |
| 3% | a new deal in the run's pipeline, moved to its next stage |
| 4% | a new ticket, moved to pending or resolved |
| 1% | a lead-import preview of 100 rows (read and judged, not saved) |

Each user's first iteration makes a contact, and each pause is between one and two times the
script's think time, so a user behaves like a person, not a loop. `setup()` checks
`/_health/ready` and stops the run if the service is not ready, then makes the one pipeline the deals
use and the one segment the previews read.

## The endpoint benchmark

`endpoints.test.js` runs these groups one after another, each at a constant arrival rate, so each
group's figures are its own: lists; search; contact reads; contact writes (creates and updates);
other writes (activities, leads, accounts, tickets); the rollups (each path in `ANALYTICS`);
reports; segment previews; duplicate checks; lead-import previews of 200 rows; and the audit-chain
check. Its limits are p95 under 300 ms for a contact read, 500 ms for lists, search and writes,
1 s for rollups, reports, previews and imports, and 2 s for duplicates and the audit check, which
grow with the data. Run `make seed` first: an empty database answers every list fast.

Two of its groups grow with the data, not the load: the duplicate check compares with up to 5,000
contacts, and the audit check hashes every audit row, so a long-lived database makes it slower
whatever the rate.

## Adding a call

Add a function to `lib/crm.js` that sends what the API accepts. Each request body refuses a field
it does not know, so a wrong name fails as a `422`, which the check reports. The types are in the
service's OpenAPI document and in the UI's generated `src/lib/api/schema.d.ts`. k6 reads settings from
`__ENV` (`-e NAME=value`); it has no `process.env` and no `require`.

## Not covered

- Sign-in. With `CONTACT_RELATIONSHIP_MANAGEMENT_REQUIRE_AUTH=1` the tests need a token in `AUTH_TOKEN`; they do not sign in.
- The bridges to project-portfolio and workforce-planning services, which are off unless configured.
- Many users from many machines. k6 runs from one machine here; nothing in these scripts stops you
  running several copies at once.
- Campaign runs, nurture, webhooks, the MCP endpoint and web forms: each sends to or answers
  someone outside the service, and a load test should not.
- Erasure, merges and imports that save: they change or remove records in ways a shared demo
  database would not want repeated thousands of times. The import preview reads and judges a file
  without saving it.
