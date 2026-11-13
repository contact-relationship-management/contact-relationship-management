# Long-running tests

Tests that run for hours to show whether the service stays stable: no rising error rate, no latency drift, no
failed health checks. They use [k6](https://grafana.com/docs/k6/latest/).

There is one test today, `scripts/stability.test.js`. Memory, connection-pool, background-job, calendar, nurture
and SLA soak tests are **not built**.

## What the stability test does

- **Workload.** A ramp up, a steady phase and a ramp down. Each virtual user does list reads (75%), single-contact
  reads (10%), contact creates (6%), contact updates (5%) and activity creates (4%), with a 1 to 3 second pause.
- **Health probe.** A separate scenario calls `/_health/live` and `/_health/ready` twice a minute.
- **Drift.** Requests in the first and last tenth of the workload are tagged `phase:early` and `phase:late`. The
  summary shows both latency series, so a slowdown over the run is visible side by side.

| Threshold | Limit |
| --- | --- |
| Failed requests (`http_req_failed`) | under 1% |
| Workload latency | p95 under 1 s, p99 under 3 s |
| Latency in the last tenth of the run | p95 under 1 s |
| Checks | over 99% pass |
| Health probe failures | none |

It writes synthetic contacts and activities and does not delete them: the API has no contact delete, and erasure
is a privacy operation. **Run it against a throwaway database only.**

## Running it

Start a service with a throwaway database, for example the demo stack from the repository root:

```bash
podman compose up --build
```

Then, from this directory:

```bash
make quick                        # 10 minutes: check the script and the service
make stability                    # 6 hours, 5 VUs
make stability DURATION=24h VUS=10
make stability K6=podman-k6       # k6 from the pinned container image, no local install
AUTH_TOKEN=... make stability     # a service with sign-in on
```

`make` writes a JSON summary to `results/`. With `K6=podman-k6`, a `localhost` URL is rewritten to
`host.containers.internal` so the container can reach the host.

| Variable | Default | Meaning |
| --- | --- | --- |
| `TEST_URL` | `http://localhost:1241` | Service base URL |
| `AUTH_TOKEN` | empty | Bearer token, passed as `CRM_AUTH_TOKEN` |
| `VUS` | `5` | Virtual users in the steady phase |
| `DURATION` | `6h` | Length of the steady phase |
| `RAMP` | `5m` | Length of the ramp up and of the ramp down |

## What has been verified

- A two-minute run (5 VUs, 15 s ramps) against the demo stack on 2026-11-13: all seven checks and every threshold
  passed, with workload p95 at about 6 ms.
- `make stability K6=podman-k6 DURATION=1m RAMP=10s` against `podman compose up --build` on the same day: every
  check and threshold passed, and `results/stability-summary.json` was written.
- **Not yet run:** a full 6-hour run, a run with sign-in on, `make quick`, and k6 installed on the host.
