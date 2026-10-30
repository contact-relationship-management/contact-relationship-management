# Contact matcher

The probability that two contact records are the same person, with the reasons.

```rust
use contact_relationship_management_matcher::{Contact, Matcher, Verdict};

let matcher = Matcher::default();
let a = Contact::new().given_name("Robert").family_name("Smith")
    .email("rob.smith@example.com").organization("Acme Inc.");
let b = Contact::new().display_name("Bob Smith")
    .email("Rob.Smith@Example.com").organization("ACME");

let result = matcher.compare(&a, &b);
assert_eq!(result.verdict, Verdict::Match);
println!("{:.4}", result.probability);
for f in &result.fields {
    println!("{:?}: {:?} (weight {:+.2})", f.field, f.outcome, f.weight);
}
```

## How it works

Fellegi–Sunter record linkage. Every field is `Agree`, `Partial`, `Disagree` or
`Missing`. An agreement adds `ln(m / u)` to the log-odds of a match, a
disagreement adds `ln((1 - m) / (1 - u))`, a partial agreement lands in
between, and a missing field adds nothing. `m` is how often the field agrees
for the same person; `u` how often it agrees by coincidence. The log-odds start
from a `prior` and become the probability. Two thresholds turn that into a
`Verdict`: `Match`, `Review` or `NonMatch`.

| Field | Agrees when |
|---|---|
| Given name | equal, a known nickname (`Bob`/`Robert`), or close by Jaro–Winkler; an initial is a partial agreement. Only the first word counts. |
| Family name | equal or close, ignoring case, accents and apostrophes. |
| Email | any shared address; Gmail dots and `+tags` ignored. |
| Phone | any shared number across `+44…`, `0044…` and national formats. |
| Organisation | equal after dropping `Inc`, `Ltd`, `GmbH` and the like. |
| Job title | close, or the same words in any order, with `VP`, `CTO`, `Sr` spelled out. |

Also: given and family names that match better swapped are swapped, and
`names_swapped` says so; a display name fills in a missing given or family name
(`Dr Ana Ruiz`, `Ruiz, Ana`, `Ana van der Berg`); a shared `person_ref` is a
certain match (`deterministic`); `compare(a, b)` equals `compare(b, a)`.

`compare_one_to_many` scores a query against candidates, and
`rank_one_to_many` returns them best first in a stable order, leaving out
non-matches.

## Configuration

`MatchConfig` holds the prior, thresholds, each field's `m` and `u`, and a few
options (`fold_gmail`, `default_calling_code`, extra `nicknames`). It loads from
JSON, every key is optional, and `Matcher::new` refuses values that make no
sense (a probability outside `(0, 1)`, an `m` not above its `u`, thresholds out
of order). The refusal is a `ConfigError`, an enum that says which setting.

**The defaults are checked against synthetic pairs, not fitted to your data.**
Tune them on pairs you have already reviewed before you act on a `Match`
automatically.

## Tuning

`tests/fixtures/pairs.json` holds 105 labelled pairs (44 the same person, 61
not): nicknames, initials, typos, accents, transliteration, swapped and
compound names, a changed surname, a job change, sparse records, a shared
mailbox or phone, the same name at different employers, look-alike names, and
families. About half are the hard look-alikes. `tests/tuning.rs` scores them
(`cargo test --test tuning -- --nocapture`), prints precision, recall and a
calibration table, and pins floors so a later change cannot make it worse.
`cargo test --test tuning -- --ignored --nocapture sweep` searches the
parameters.

Measured on full records (all fields):

| Match at or above | Precision | Recall |
|---|---|---|
| 0.5 | 0.61 | 0.98 |
| 0.9 | 0.70 | 0.91 |
| 0.99 | 0.89 | 0.89 |
| 0.999 | 0.94 | 0.66 |

What this changed and what it did not:

- **`match_threshold` went from 0.9 to 0.99.** Between 0.9 and 0.99, 13 pairs
  scored and only one was the same person; at 0.9, 17 different pairs were
  called `Match`, at 0.99, 5. That cost one true match.
- **The other defaults stayed.** A sweep of `agree_at`, `disagree_at`, the
  prior and the `u` of names, organisation and title found nothing better than
  the defaults by more than a rounding error, so no change is justified.
- **`review_threshold` stayed at 0.5.** Below it, 1 of 35 pairs was the same
  person; lowering it to 0.3 gains no recall.
- **The probabilities are over-confident.** The 13 pairs the matcher put
  between 0.90 and 0.99 (stated about 0.97) were the same person 8% of the
  time; the 44 above 0.99 (stated 0.998) 89% of the time. Treat the number as a
  ranking, not a chance. The independence assumption is the cause: a name, an
  employer and a title that agree together are less independent than the maths
  assumes.
- **Some errors cannot be fixed by tuning.** Family members who share a phone
  and an employer, a contractor with two clients, and look-alike names at one
  employer look the same as one person written twice. They stay `Match` and
  need a person.
- **Without email and phone** (what the service has before it fetches an
  address), recall at `Review` falls from 0.98 to 0.89 and changed surnames and
  job changes are the misses.

**Limits of this evidence.** The pairs are synthetic and were written by the
people who wrote the matcher, which flatters it even with the hard cases in. They
show how the matcher treats the cases someone thought of, not how it performs on
real contacts. Real tuning needs pairs a person has reviewed, drawn from your
own data, under the same data-protection rules as that data.

## Limits

- Fields are treated as independent, which over-states confidence when they
  are correlated (the same name and the same employer, say).
- The nickname classes, honorifics, name suffixes and legal-form words are
  per-language data (`src/nicknames.rs`, `src/words.rs`): English and a dozen
  other languages, including Arabic, Hindi, Russian, Chinese, Japanese and Korean
  forms, all applied to every contact. They were drafted without a speaker of
  each language checking them. The name particles are still European. A name
  written without spaces (Chinese, Japanese, Korean) is not split into given and
  family names, and Jaro–Winkler suits Latin scripts best.
- Phones are compared as digits (typed in any Unicode decimal digit set or in full width); there is no country-code database beyond a table of which calling codes' plans have a trunk prefix (`0`, `8` or `06`) to drop from a national number, and a code not in it drops nothing.
- No date of birth or address.
- It is advice. It merges and deletes nothing.

## Changing the API

`Field`, `Outcome`, `Verdict` and `ConfigError` are `#[non_exhaustive]`, so a new
variant is not a breaking change; match them with a wildcard arm. So are the result
types `MatchResult` and `FieldScore`, which you read and do not build.
`Contact`, `MatchConfig` and `FieldParameters` are not: you build them with
`..Default::default()`, which `#[non_exhaustive]` would forbid. `Ranked` is not
either: the service builds it to re-sort and re-score candidates. Adding a field to
one of those is therefore a breaking change, and goes out in a new minor version
while the crate is 0.x.

## Privacy

Contact data is personal data. The crate stores nothing, logs nothing and does
no input or output: hand it what the comparison needs and drop it afterwards.
In the CRM, email addresses live on the person record and are fetched at
comparison time.

## Checks

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cargo semver-checks   # against the last published version
```

Traces to CRM-R23 and CRM-T60 in the repository's [spec](../spec/).
