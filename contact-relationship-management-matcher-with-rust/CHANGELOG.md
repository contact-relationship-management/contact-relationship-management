# Changelog

## Unreleased

- `Matcher::config` is now a `const fn`. No behaviour change.

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
