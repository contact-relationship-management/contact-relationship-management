# Load tests

[k6](https://k6.io/) scripts that put the CRM API under load: smoke, basic, stress, spike and soak.
They call the API's real routes with its real request bodies, through one module, `lib/crm.js`.

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
make help
```

`K6=podman-k6` runs the pinned k6 image with Podman, so k6 need not be installed; without it, the
`k6` on your `PATH` runs. `TEST_URL` names another service and `AUTH_TOKEN` gives a bearer token for
one with sign-in on. From `tests/`, `make load-smoke` and `make load-quick` do the same.

Each run writes `results/<test>-summary.json` and, for runs longer than a few seconds, k6's HTML
report `results/<test>.html`.

GitHub Actions has a manual workflow, **Load Test** (`.github/workflows/load-test.yml`), that runs one
script against a URL you give it and uploads the results.

## The tests

| Test | Script | Load | Defaults | Fails when |
| --- | --- | --- | --- | --- |
| Smoke | `smoke.test.js` | One user makes every call once per iteration | 3 iterations (`ITERATIONS`) | any call fails, or p95 is 1 s or more |
| Basic | `basic-load.test.js` | A steady number of users on the mixed workload | `VUS=10`, `DURATION=5m` | over 1% of requests or checks fail, p95 ≥ 500 ms, p99 ≥ 1.5 s |
| Stress | `stress.test.js` | Steps of one, two and four times `VUS`, less pause | `VUS=10`, `DURATION=2m` per step | over 5% fail, p95 ≥ 2 s |
| Spike | `spike.test.js` | `VUS`, then ten times as many within 10 s, then back | `VUS=5`, `DURATION=1m` of spike | over 5% fail, p95 ≥ 3 s |
| Soak | `soak.test.js` | The mixed workload for hours | `VUS=10`, `DURATION=2h` | as basic |

The mixed workload (`mixed` in `lib/crm.js`) is mostly reads, as a CRM's use is:

| Share | Call |
| --- | --- |
| 60% | a list of contacts, accounts, leads, deals, activities or tickets |
| 10% | `/api/search` |
| 10% | one contact this user made |
| 6% | a new contact |
| 4% | an update to one of this user's contacts |
| 4% | an activity on one of them |
| 3% | a new lead |
| 3% | a new deal in the run's pipeline, moved to its next stage |

Each user's first iteration makes a contact, and each pause is between one and two times the
script's think time, so a user behaves like a person, not a loop. `setup()` checks
`/_health/ready` and stops the run if the service is not ready, then makes the one pipeline the deals
use.

## Adding a call

Add a function to `lib/crm.js` that sends what the API accepts. Each request body refuses a field
it does not know, so a wrong name fails as a `422`, which the check reports. The types are in the
service's OpenAPI document and in the UI's generated `src/lib/api/schema.d.ts`. k6 reads settings from
`__ENV` (`-e NAME=value`); it has no `process.env` and no `require`.

## Not covered

- Sign-in. With `CRM_REQUIRE_AUTH=1` the tests need a token in `AUTH_TOKEN`; they do not sign in.
- The bridges to project-portfolio and workforce-planning services, which are off unless configured.
- Many users from many machines. k6 runs from one machine here; nothing in these scripts stops you
  running several copies at once.
