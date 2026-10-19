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
of order).

**The defaults are reasoned guesses, not fitted to your data.** Tune them on
pairs you have already reviewed before you act on a `Match` automatically.

## Limits

- Fields are treated as independent, which over-states confidence when they
  are correlated (the same name and the same employer, say).
- Names are English-centred: the nickname table, honorifics and name particles
  are. Jaro–Winkler suits Latin scripts best.
- Phones are compared as digits; there is no country-code database.
- No date of birth or address.
- It is advice. It merges and deletes nothing.

## Privacy

Contact data is personal data. The crate stores nothing, logs nothing and does
no input or output: hand it what the comparison needs and drop it afterwards.
In the CRM, email addresses live on the person record and are fetched at
comparison time.

## Checks

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

Traces to CRM-R23 and CRM-T60 in the repository's [spec](../spec/).
