//! # Contact matcher
//!
//! The probability that two contact records are the same person.
//!
//! The method is Fellegi–Sunter record linkage. Each field (given name,
//! family name, email, phone, organisation, job title) either agrees,
//! partly agrees, disagrees, or is missing. Each outcome adds or subtracts
//! evidence, measured as log-odds, to a prior; the sum becomes a
//! probability. Because every field's contribution is kept, a result can
//! always say *why*.
//!
//! - Names are folded (`Siân` is `sian`, `O'Brien` is `obrien`), compared
//!   with Jaro–Winkler, and lifted for nicknames (`Bob` and `Robert`) and
//!   initials. Given and family names that match better swapped are
//!   swapped, and the result says so.
//! - Emails agree on a shared address; Gmail dots and `+tags` are ignored.
//! - Phones agree on a shared number, across `+44 20…`, `0044 20…` and
//!   `020…` formats.
//! - Organisations ignore `Inc`, `Ltd`, `GmbH` and the like; job titles
//!   spell out `VP`, `CTO`, `Sr`.
//! - A shared `person_ref` means the same person, with certainty.
//! - A field that is empty on either side says nothing, so sparse records
//!   are not punished.
//!
//! ```
//! use contact_relationship_management_matcher::{Contact, Matcher, Verdict};
//!
//! let matcher = Matcher::default();
//! let a = Contact::new().given_name("Robert").family_name("Smith")
//!     .email("rob.smith@example.com").organization("Acme Inc.");
//! let b = Contact::new().display_name("Bob Smith")
//!     .email("Rob.Smith@Example.com").organization("ACME");
//!
//! let result = matcher.compare(&a, &b);
//! assert_eq!(result.verdict, Verdict::Match);
//! assert!(result.probability > 0.99);
//! ```
//!
//! The tuning lives in [`MatchConfig`], which loads from JSON. The
//! defaults are reasoned guesses, not fitted to your data: tune `prior`
//! and the per-field `m` and `u` against contacts you have already
//! reviewed before acting on a [`Verdict::Match`] automatically.
//!
//! The crate stores nothing and does no I/O. Contact data is personal
//! data; hand the matcher what it needs for the comparison and drop it.

mod config;
mod contact;
mod matcher;
mod nicknames;
mod normalize;
mod words;

pub use config::{ConfigError, FieldParameters, MatchConfig};
pub use contact::Contact;
pub use matcher::{Field, FieldScore, MatchResult, Matcher, Outcome, Ranked, Verdict};
