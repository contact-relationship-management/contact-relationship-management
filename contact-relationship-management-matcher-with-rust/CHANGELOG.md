# Changelog

## Unreleased

- Licence expression: `CC-BY-SA-4.0` replaces `CC-BY-NC-SA-4.0` (2026-11-24, at the owner's request). Releases already published keep the expression they were published with.
- Phone comparisons preserve all 770 Unicode 18.0 decimal digits, including Adlam and mixed-script numbers (CRM-T270).
- Two national phone numbers compared without a default calling code must agree on every digit, including a leading zero (CRM-T271).
- A national number matching only the suffix of an international number is partial evidence, not
  full agreement, unless a configured calling code normalizes both into the same international
  number. A labelled twelve-pair corpus records why and the effect on verdicts (CRM-T273).
- Mutation testing (CRM-T259): `.cargo/mutants.toml`, with one equivalent mutant named and why.
  New unit tests pin the probability as the prior plus the weights, a partial field's weight, a field
  on one side only, the single-letter initial, a tie that is not a swap, a title's word overlap, the
  family name of a three-word name, a name that is only a title or a suffix, the ideographic space,
  and the open bounds of `MatchConfig::validate`. `email` drops two checks that the rest of it
  already made (an empty domain has no dot; an empty local part is refused after Gmail's folding).

## 0.2.1

No change in behaviour or public API.

- `rust-version` is 1.98, down from 1.99 (CRM-T225; 1.97 until the toolchain moved to 1.100.0, `62dc845`). `cargo +1.98
  check --all-targets` passes.
- The dependency minimums are the versions in use: `strsim` 0.11.1, `unicode-normalization` 0.1.26 (and `criterion`
  0.8.2 for the benchmarks).
- rustfmt 1.100.0 layout, and `map_or_default` where clippy 1.100.0 asks for it.

## 0.2.0

Breaking. Published to crates.io on 2026-10-31 by the maintainer; the service depends on it from the same day.

- Words by language (CRM-T169): honorifics, name suffixes and legal-form words are per-language data in a new `words`
  module (English and a dozen other languages, in Latin, Cyrillic, Arabic and Devanagari), and a script without spaces
  has its joined honorifics (`女士`, `様`, `님`) and legal forms (`有限公司`, `株式会社`, `주식회사`) stripped from a word's end
  or start. The nickname table adds German, Spanish, French, Italian, Portuguese, Russian (Latin and Cyrillic), Polish
  and Dutch classes and the romanisations of some Arabic names. All lists apply to every contact. Drafted without a
  speaker of each language checking them. Behaviour change, not an API change.
- Phones in every script (CRM-T170): digits typed in ASCII, full width or any of 31 Unicode decimal digit sets (the
  workforce planning service's table, WPM-T382) are read, so the same line typed in Arabic-Indic and in ASCII digits
  agrees. With `default_calling_code`, a national number's trunk prefix is dropped only where the code's plan has one
  (`TRUNK_PREFIXES`: `0`, `8` or `06`, or none for Italy, Spain, Portugal, Greece, Poland, Czechia, Denmark, Norway and
  the other closed plans listed); a code not in the table drops nothing (it dropped a leading `0` for every code). The
  table is from public summaries of national plans and was not checked against each regulator. Behaviour change, not an
  API change.
- `ConfigError` is now a `#[non_exhaustive]` enum (`OutOfRange`, `ThresholdsOutOfOrder`,
  `SimilarityBandsOutOfOrder`, `NotDiscriminating`, `CallingCode`), where it was a tuple struct holding a
  `String`. Its `Display` text is unchanged.
- `Field`, `Outcome`, `Verdict`, `MatchResult` and `FieldScore` are `#[non_exhaustive]`, so later
  additions are not breaking. Code that matches them exhaustively needs a wildcard arm; nothing else changes
  for code that reads them.
- `Contact`, `MatchConfig`, `FieldParameters` and `Ranked` stay constructible (see the README, "Changing the API").
- A criterion benchmark (`cargo bench`, `benches/matching.rs`); results are in the repository's `BENCHMARKS.md`.
- Lints: `clippy::pedantic`, `unwrap_used` and `expect_used` are on; the library needed no change.

## 0.1.2

- `Matcher::config` is now a `const fn`. No behaviour change.
  Published to crates.io on 2026-10-20.

## 0.1.1

- Default `match_threshold` raised from 0.9 to 0.99, from a labelled set of
  105 synthetic pairs (`tests/fixtures/pairs.json`): different pairs called
  `Match` fell from 17 to 5 at the cost of one true match. Other defaults
  unchanged. Probabilities are over-confident; see the README's "Tuning".
  Behaviour change: contacts scoring 0.90 to 0.99 are now `Review`, not `Match`.
  Published to crates.io on 2026-10-20.

## 0.1.0

- First version: Fellegi–Sunter matching of two contacts (given and family
  name, email, phone, organisation, job title), nicknames, initials, swapped
  names, a certain match on a shared `person_ref`, batch scoring and ranking,
  and a validated configuration that loads from JSON. Published to crates.io
  on 2026-10-19; used by the service's duplicates endpoint.
