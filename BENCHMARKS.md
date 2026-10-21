# Benchmarks

Only the matcher has benchmarks so far. The endpoints, scoring, segments and
front end have none yet.

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

## Intended coverage

- Request latency for list, search, and forecast endpoints against PostgreSQL 18.
- Throughput of lead scoring and segment evaluation.
- Front-end bundle size and load time.

## Method

Each result should record the commit, hardware, dataset size (synthetic), tool
and command used, and the raw output, so that it can be reproduced.
