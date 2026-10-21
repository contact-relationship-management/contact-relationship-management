//! Speed of the matcher: one pair, and one contact against many.
//!
//! The contacts are synthetic and generated from a fixed seed, so every run
//! compares the same data. Run `cargo bench`; see `BENCHMARKS.md` for results.

#![allow(
    clippy::expect_used,
    reason = "a benchmark fails loudly on a bad setup"
)]

use std::hint::black_box;

use contact_relationship_management_matcher::{Contact, MatchConfig, Matcher};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

const GIVEN: [&str; 12] = [
    "Imogen",
    "Robert",
    "Aisha",
    "Wei",
    "Katarzyna",
    "Liam",
    "Fatima",
    "Jose",
    "Priya",
    "Oluwaseun",
    "Chloe",
    "Mateo",
];
const FAMILY: [&str; 12] = [
    "Hartley",
    "Okafor",
    "Nguyen",
    "Kowalski",
    "Haddad",
    "Fernandez",
    "Singh",
    "Muller",
    "Rossi",
    "Tanaka",
    "Adeyemi",
    "Lindqvist",
];
const ORG: [&str; 6] = [
    "Northwind Traders",
    "Contoso Ltd",
    "Fabrikam",
    "Adventure Works",
    "Tailspin Toys",
    "Wingtip",
];
const TITLE: [&str; 5] = [
    "Head of Procurement",
    "Account Manager",
    "Chief Financial Officer",
    "Operations Lead",
    "Sales Director",
];

/// A tiny deterministic generator, so the benchmark needs no extra dependency.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, bound: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from(self.0 >> 33).unwrap_or(0) % bound
    }
}

fn contact(rng: &mut Lcg, serial: usize) -> Contact {
    let given = GIVEN[rng.next(GIVEN.len())];
    let family = FAMILY[rng.next(FAMILY.len())];
    Contact::new()
        .given_name(given)
        .family_name(family)
        .email(format!(
            "{}.{}{serial}@example.com",
            given.to_lowercase(),
            family.to_lowercase()
        ))
        .phone(format!("+4477009{:05}", serial % 100_000))
        .organization(ORG[rng.next(ORG.len())])
        .job_title(TITLE[rng.next(TITLE.len())])
}

fn bench(c: &mut Criterion) {
    let matcher = Matcher::new(MatchConfig::default()).expect("the default configuration is valid");
    let mut rng = Lcg(42);
    let query = contact(&mut rng, 0);

    let other = contact(&mut rng, 1);
    c.bench_function("compare one pair", |b| {
        b.iter(|| matcher.compare(black_box(&query), black_box(&other)));
    });

    let mut group = c.benchmark_group("compare_one_to_many");
    group.sample_size(20);
    for size in [1_000_usize, 5_000, 50_000] {
        let candidates: Vec<Contact> = (0..size).map(|i| contact(&mut rng, i + 2)).collect();
        group.bench_with_input(BenchmarkId::from_parameter(size), &candidates, |b, cs| {
            b.iter(|| matcher.compare_one_to_many(black_box(&query), black_box(cs)));
        });
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
