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
| SCORE-1 | A lead score (CRM-D5) | The rule weights total below 0 or above 100 | 0 or 100 | The score is defined on 0 to 100; the breakdown still lists every rule. |

## Input out of range or malformed is an error

A declaration MUST NOT name a fallback, default or bound for an input that is out of range or malformed
(RCA-DECL-6, RCA-IN-2). A default applies only to an input that is absent (RCA-IN-3). Such an input is rejected
where it enters, with an error that names the figure:

- **A `?days=` window** (1 to 365, default per endpoint when absent): a `422` naming `days` and the range
  (`arith::out_of_range`). `days=0`, `days=9999` and `days=` are refused.
- **A page size** (`limit`, 1 to 100, 20 when absent) on the duplicates list: a `422`.
- **A card's position in a column**: outside the column, `board::new_order` returns `None` and the request is a `422`.
- **A numeric setting** (`MIN_CELL_SIZE`, `MAIL_BATCH`, `ATTACHMENT_MAX_BYTES`, the three client limits,
  `PASETO_KEYS_REFRESH_SECS`) that is blank, not a number or out of range: `Settings::unusable` names it and
  start-up refuses (RCA-IN-4). The three retention settings are refused when below their floors (30, 7 and 400
  days) and are no longer raised to them; `rules::privacy::retention_days` and its two siblings return `None` for
  an invalid value, and the retention endpoints then answer with an error that names the setting.
- **The rate-limit numbers** (`RATE_LIMIT_PER_MIN`, `AUTH_FAIL_LIMIT`, `AUTH_FAIL_WINDOW_SECS`) that are set but not a
  whole number or too large: `Guard::from_env` returns an error that names the setting, and start-up stops. A value
  that is not valid text is read lossily and fails the same way, never as unset.
- **A stored organization calendar** (time zone, fiscal start month, stalled-after days) that is no longer valid is
  corrupt data (RCA-IN-6): an error that names the field, not the starting values.
- **An `Accept-Language` header** with a `q` value that is not 0 to 1 with up to three decimals, or that is not
  text: a `422` naming the header (`public_text::negotiate`, `controllers::accept_language`). Decided by the owner
  on 2026-11-22; it is not read as 0 and the range is not passed over. The address's `?lang=`, when the pages have
  it, is used before the header is read.
- **A `From` value** whose brackets cannot be read: `address_in` returns `None`, and the reply is an
  `unverified_sender`.
- **A fiscal start month** outside 1 to 12: no period SQL (`fiscal_year_expr` returns `None`), and a report plan
  refuses with `fiscal_year_start_month`.
- **A deal probability** outside 0 to 100: the forecast returns an error.

The declarations above are for a computed result, never an input. SCORE-1 stays because the lead score is a total
of validated rule weights, a computed result.

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
- No `saturating_*`, `wrapping_*`, `overflowing_*` or `clamp` in production code of the service and domain crates.
  `rules::bounds::within(n, lo..=hi)` tests a range; out of range is an error.
- `clippy.toml` bans `Iterator::sum` and `product`, `Duration::from_secs_f*`, `Instant::duration_since`,
  `Instant::saturating_duration_since`, `Ord::clamp`, `Saturating` and `Wrapping` (RCA-ENF-2). Tests, the benchmark
  and the integration tests allow `disallowed_methods`.
- The release profile sets `overflow-checks = true` (RCA-ENF-4).
- `scripts/check-checked-arithmetic.py` runs in the `clippy` stage and fails on `saturating_*`, `wrapping_*`,
  `overflowing_*`, `clamp` and an unwrapped `checked_*` result in production code (RCA-ENF-5).
- Not yet at the rule:
  - **`Option::unwrap_or_default`** is still used in 40-odd places, which RCA-P-3 forbids and `clippy.toml` does not
    yet ban. Done: stored JSON that read as empty is an error (`json::stored`, RCA-IN-6); a mail request body that
    cannot be serialized is a permanent send error and is no longer signed as empty; audit-chain snapshots use
    `Value::to_string`; the chain's first `prev_hash` is the named `GENESIS_PREV_HASH` (AUDIT-GENESIS-1: the first
    row has no predecessor, so its chain starts from the empty hash); the client key reads its digest by pattern;
    contact erasure returns exactly 13 counts or an error; the segment filter, the forecast totals and a ticket are
    checked when serialized. Left, to be decided or excepted one by one: the insight cache's generation stamp
    (`insight_cache.rs`, 0 after the year 2262), `Decision::record` in `consent_profiles.rs` (writes `null` if a value
    will not serialize), and the absent-means-empty map entries and optional texts.
  - `as` conversions and the other lints of RCA-ENF-1 (`as_conversions`, `cast_*`, `integer_division`,
    `float_arithmetic`, `string_slice`, `allow_attributes`).
  - The matcher crate has no `clippy.toml`, and its `clamp` of the log-odds is not converted.
  - `try_from(..).unwrap_or(..)` on conversions; the integration tests in `tests/` still use plain operators
    (RCA-EXC-5 allows it).
