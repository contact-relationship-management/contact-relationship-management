# Arithmetic

The rules are in `spec/rust-checked-arithmetic/index.md` (requirement IDs `RCA-*`). That file is the source of
truth. This page holds what that file asks each project to declare (RCA-DECL): the figures that have a declared
fallback, so the code can recover at the source with `checked_*` plus `unwrap_or` (RCA-P-7), and what the code does
for every other figure.

Math must not panic and must not wrap. Integer arithmetic on an external figure uses `checked_*`. A `None` is
recovered with a fallback named below, or becomes an error that names the figure, or stays `None` as "unknown". Every
other figure has no fallback (RCA-DECL-5). The example is `255_u8.checked_add(1)`, which is `None`.

## Figures with a declared fallback

Each call site cites its ID in a comment. A fallback is a valid value of its figure and says why it is correct
(RCA-DECL-4).

| ID | Figure | Failure | Declared fallback | Why it is correct |
| --- | --- | --- | --- | --- |
| TALLY-1 | A count of rows, events or attempts (`arith::Tally`) | The count passes its type's largest value | The type's `MAX` | A tally is how many things were seen; past the type it stays at the largest value. No money rests on it. |
| CLOCK-1 | The corrected time (`clock::now`) | Host time plus the database offset is out of range | The host's time | The correction is unknown, so the uncorrected time is used, not a made-up one. |
| CLOCK-2 | The database clock offset | A gap too large for `i64` milliseconds | The offset in use is kept | A reading that cannot be held is no measurement. |
| CLOCK-3 | The gap measured from one reading | The midpoint or the difference does not fit | A gap of zero | Zero says "no correction", the same as before any reading. |
| RETRY-NEXT-1 | A webhook's next attempt time | `now` plus a delay of at most a day is out of range | `now` | The delay table is bounded, so this is `now` at the end of time, and the attempt is tried at once. |
| SALES-WINDOW-1 | The start of a sales window | `to` minus 30 days is out of range | `to` | An empty window, not a date nobody asked for. |
| TERRITORY-WINDOW-1 | The start of a territory period | `to` minus the default days is out of range | `to` | An empty period. |
| CACHE-1 | The lifetime of the insight cache's generation key | Four times the entry lifetime overflows | One entry lifetime | The key outlives an entry by at least that. |
| CACHE-2 | The age of a cached worker name | The entry is stamped after now | The entry is stale | An age that cannot be measured is not fresh. |
| RATE-1 | A client's remaining request budget | The count is above the limit | Zero | A client past its limit has no budget left. |
| HEADROOM-1 | The rows fetched before the in-app audit filter | Ten times the page overflows | The page itself | No headroom, which is correct if less efficient. |
| CLAWBACK-1 | A worker's credit on a deal after a clawback (CRM-R68) | Never below nothing | Zero (applied as `max(0)` on a checked result) | A clawback takes from the credit and cannot make it negative. |
| QUOTA-REMAINING-1 | What is still to win to meet a quota (CRM-R59) | The quota is met | Zero | Nothing is left to win once it is met. |
| PERIOD-1 | Months a fiscal year starts after January | The start month is 0 | Zero months | January starts the year. |
| ADDR-1 | The mailbox in a `From` value | A bracket pair that cannot be read | The whole value | The whole value is trimmed and read as the mailbox. |
| DAYS-1 | A `?days=` window on a request | Outside 1 to 365 | The endpoint's default window | An out-of-range window is not read as the nearest bound; the caller gets the window the endpoint documents. |
| LIMIT-1 | A page size on the duplicates list | Outside 1 to 100 | 20 | The default page. |
| SETTING-1 | A numeric setting (`MIN_CELL_SIZE`, `MAIL_BATCH`, `ATTACHMENT_MAX_BYTES`) | Outside its range, or not a number | The setting's default (5, 200, 10 MiB) | A mistyped setting is the default the documentation names, not a bound nobody chose. |
| SCORE-1 | A lead score (CRM-D5) | The rule weights total below 0 or above 100 | 0 or 100 | The score is defined on 0 to 100; the breakdown still lists every rule. |
| BOARD-1 | A card's position in a column | Below 0 or past the end | The start or the end | A card dropped past the column is last. |

## Figures with no fallback

These return `None` (unknown) or an error that names the figure and shows no number (RCA-P-4, RCA-ERR-2,
RCA-ERR-4). A `422` says "`<figure>` could not be calculated" (`arith::overflow`).

- **Money and quantities in minor units:** exchange conversion (`fx::convert*`), commission and bands
  (`quota::bands`, `commission`, `split`), margin, retention and projection (`analytics`), attributed credit
  (`attribution::credit*`), revenue by currency, team and campaign totals, win rate and ROI. A sum or product that does
  not fit is an error, not `i64::MAX`.
- **Instants and dates:** a deadline, a window start, an expiry or a due date that is out of range is an error. SLA
  deadlines and paused time (`sla::deadlines*`, `pause_counted`) return `None`.
- **Small-cell suppression (CRM-R78):** a group size that does not fit is an error, so no cell is shown unchecked.
- **Lead score:** `scoring::score` returns `None` when the rule weights do not add.
- **Audit chain:** `audit_chain::verify` returns `None` when it cannot count the rows.

## Where it stands (2026-11-22)

- Both crates warn on `arithmetic_side_effects`, `indexing_slicing`, `panic`, `unreachable`, `todo`, `unimplemented`,
  `unwrap_used` and `expect_used` outside tests (CI runs clippy with `-D warnings`).
- `Timestamp` and `Date` have no operators in production: `checked_add`, `checked_sub`, `checked_add_days`,
  `checked_sub_days` and `duration_since`. The operators exist only for tests (`time::test_operators`).
- No `saturating_*`, `wrapping_*` or `overflowing_*`, and no `clamp`, in production code of the service and domain crates. A bound is `rules::bounds::within(n, lo..=hi)` plus `unwrap_or` with a declared fallback above. A deal probability outside 0 to 100 has no forecast (an error, no fallback). The matcher crate's `clamp` of its log-odds (a float bound of its own model) is not yet converted.
- Not yet at the rule: `as` conversions and the other lints of RCA-ENF-1 (`as_conversions`, `cast_*`,
  `integer_division`, `float_arithmetic`, `string_slice`, `allow_attributes`); `clippy.toml` `disallowed-methods`
  (RCA-ENF-2); `overflow-checks` in the release profile (RCA-ENF-4); the CI grep (RCA-ENF-5); `try_fold` in place of
  `sum` (RCA-AGG-1); `try_from(..).unwrap_or(..)` on conversions, which is a different `Option` from a checked
  operation but has the same fault; the integration tests in `tests/` still use plain operators (RCA-EXC-5 allows it).
