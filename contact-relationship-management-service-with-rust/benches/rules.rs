//! Speed of the rules a request runs, at the sizes the service meets (CRM-T242).
//!
//! Every input is synthetic and made from a fixed seed, so each run measures the same work. The
//! rules are pure, so no database, network or clock is involved: these are the CPU cost of a
//! request's rule step, not a request's latency (the k6 tests in `tests/load/` measure that).
//! Run `cargo bench --bench rules`; see `BENCHMARKS.md` for results.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    missing_docs,
    reason = "a benchmark fails loudly on a bad setup"
)]

use std::collections::BTreeMap;
use std::hint::black_box;

use contact_relationship_management::rules::{
    analytics, attribution, audit_chain, fx, imports, reports, scoring, search, segment,
};
use contact_relationship_management::time::{Date, Timestamp};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use jiff::SignedDuration;
use uuid::Uuid;

/// A tiny deterministic generator, as in the matcher's benchmark, so the inputs need no extra
/// dependency.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, bound: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from(self.0 >> 33).unwrap_or(0) % bound
    }

    fn uuid(&mut self) -> Uuid {
        let hi = (self.next(1 << 30) as u128) << 98 | (self.next(1 << 30) as u128) << 68;
        let lo = (self.next(1 << 30) as u128) << 30 | self.next(1 << 30) as u128;
        Uuid::from_u128(hi | lo)
    }
}

const SIZES: [usize; 3] = [100, 1_000, 10_000];
const CURRENCIES: [&str; 5] = ["GBP", "EUR", "USD", "JPY", "KWD"];

fn start() -> Timestamp {
    "2026-06-01T09:00:00Z".parse().unwrap()
}

fn lead_facts(rng: &mut Lcg) -> scoring::LeadFacts {
    scoring::LeadFacts {
        source: ["web", "referral", "campaign", "event"][rng.next(4)].to_string(),
        campaign_attributed: rng.next(2) == 0,
        known_contact: rng.next(3) == 0,
        email_domain: Some(["example.com", "gmail.com", "example.org"][rng.next(3)].to_string()),
        days_since_last_activity: Some(i64::try_from(rng.next(45)).unwrap()),
        activity_count: i64::try_from(rng.next(8)).unwrap(),
        campaign_click: rng.next(4) == 0,
        unsubscribed: rng.next(20) == 0,
    }
}

/// Lead scoring (CRM-D5): one lead, and a page of leads as the list endpoint scores them.
fn bench_scoring(c: &mut Criterion) {
    let weights = scoring::Weights::default();
    let mut rng = Lcg(1);
    let one = lead_facts(&mut rng);
    let mut group = c.benchmark_group("scoring");
    group.bench_function("score/one", |b| {
        b.iter(|| scoring::score(black_box(&one), black_box(&weights)));
    });
    for size in SIZES {
        let leads: Vec<_> = (0..size).map(|_| lead_facts(&mut rng)).collect();
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("score", size), &leads, |b, leads| {
            b.iter(|| {
                leads
                    .iter()
                    .map(|l| scoring::score(l, &weights).score)
                    .sum::<i32>()
            });
        });
    }
    group.finish();
}

/// Segment evaluation with the consent gate, over a contact book.
fn bench_segment(c: &mut Criterion) {
    let filter = segment::Filter {
        statuses: vec!["active".into(), "prospect".into()],
        channels: vec!["email".into()],
        account_tiers: vec!["customer".into(), "partner".into()],
        tags: vec!["Newsletter".into(), "vip".into()],
    };
    let mut rng = Lcg(2);
    let mut group = c.benchmark_group("segment");
    for size in SIZES {
        let contacts: Vec<_> = (0..size)
            .map(|_| segment::ContactFacts {
                consent: ["granted", "never", "withdrawn"][rng.next(3)].to_string(),
                status: ["active", "prospect", "inactive"][rng.next(3)].to_string(),
                channel: ["email", "phone", "post"][rng.next(3)].to_string(),
                account_tier: (rng.next(4) != 0)
                    .then(|| ["prospect", "customer", "partner"][rng.next(3)].to_string()),
                tags: (0..rng.next(4))
                    .map(|_| ["newsletter", "VIP", "partner", "trial"][rng.next(4)].to_string())
                    .collect(),
                employee: rng.next(50) == 0,
            })
            .collect();
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("matches", size),
            &contacts,
            |b, contacts| {
                b.iter(|| {
                    contacts
                        .iter()
                        .filter(|f| segment::matches(&filter, f))
                        .count()
                });
            },
        );
    }
    group.finish();
}

/// The stage-weighted forecast, per currency, over the open pipeline.
fn bench_forecast(c: &mut Criterion) {
    let mut rng = Lcg(3);
    let mut group = c.benchmark_group("forecast");
    for size in SIZES {
        let deals: Vec<_> = (0..size)
            .map(|_| analytics::OpenDeal {
                amount_minor: i64::try_from(rng.next(100_000_000)).unwrap(),
                currency: CURRENCIES[rng.next(CURRENCIES.len())].to_string(),
                probability_percent: [10, 20, 40, 60, 80][rng.next(5)],
            })
            .collect();
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("by_currency", size), &deals, |b, deals| {
            b.iter(|| analytics::forecast_by_currency(black_box(deals)).unwrap());
        });
    }
    group.finish();
}

/// One audit row's fields, with the snapshot as the JSON text a contact change stores.
struct AuditRow {
    id: i64,
    pid: String,
    action: &'static str,
    snapshot: String,
    micros: i64,
}

/// `GET /api/audits/verify` walks the whole chain, hashing every row: the cost grows with the
/// audit log, which only grows.
fn bench_audit_chain(c: &mut Criterion) {
    let mut rng = Lcg(4);
    let mut group = c.benchmark_group("audit_chain");
    for size in SIZES {
        let rows: Vec<AuditRow> = (0..size)
            .map(|i| {
                let pid = rng.uuid().to_string();
                AuditRow {
                    id: i64::try_from(i).unwrap() + 1,
                    snapshot: format!(
                        r#"{{"pid":"{pid}","display_name":"Contact {i}","status":"active","job_title":"Operations Lead"}}"#
                    ),
                    pid,
                    action: ["create", "update", "consent"][rng.next(3)],
                    micros: 1_780_000_000_000_000 + i64::try_from(i).unwrap() * 1_000,
                }
            })
            .collect();
        // Chain them as the service writes them, so the verifier walks to the end.
        let mut hashes: Vec<String> = Vec::with_capacity(size);
        for row in &rows {
            let prev = hashes.last().map_or("", String::as_str).to_string();
            let link = audit_chain::Link {
                id: row.id,
                prev_hash: &prev,
                entity: "contact",
                entity_pid: &row.pid,
                action: row.action,
                actor: Some("person:00000000-0000-4000-8000-000000000001"),
                snapshot: Some(&row.snapshot),
                created_micros: row.micros,
            };
            hashes.push(audit_chain::hash(&link));
        }
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("verify", size), &rows, |b, rows| {
            b.iter(|| {
                let verdict = audit_chain::verify(rows.iter().enumerate().map(|(i, row)| {
                    audit_chain::Stored {
                        link: audit_chain::Link {
                            id: row.id,
                            prev_hash: if i == 0 { "" } else { &hashes[i - 1] },
                            entity: "contact",
                            entity_pid: &row.pid,
                            action: row.action,
                            actor: Some("person:00000000-0000-4000-8000-000000000001"),
                            snapshot: Some(&row.snapshot),
                            created_micros: row.micros,
                        },
                        hash: Some(&hashes[i]),
                    }
                }));
                assert!(verdict.ok());
                verdict.rows
            });
        });
    }
    group.finish();
}

/// A lead import (CRM-T97): read the CSV, map its columns and judge each row, at the row limit.
fn bench_imports(c: &mut Criterion) {
    let mut rng = Lcg(5);
    let mut csv = String::from("Full Name,E-mail,Organisation,Notes\n");
    for i in 0..imports::MAX_ROWS {
        let org = ["Northwind Traders", "Contoso Ltd", "Fabrikam"][rng.next(3)];
        csv.push_str(&format!(
            "Lead {i},lead{i}@example.com,\"{org}\",Synthetic row {i}\n"
        ));
    }
    let mut group = c.benchmark_group("imports");
    group.throughput(Throughput::Elements(imports::MAX_ROWS as u64));
    group.bench_function(
        BenchmarkId::new("parse_and_judge", imports::MAX_ROWS),
        |b| {
            b.iter(|| {
                let table = imports::parse(black_box(&csv)).unwrap();
                let mapping = imports::suggest(&table.headers);
                let columns = imports::resolve(&mapping, &table.headers).unwrap();
                let width = table.headers.len();
                table
                    .rows
                    .iter()
                    .filter(|cells| imports::judge(cells, &columns, width).is_ok())
                    .count()
            });
        },
    );
    group.finish();
}

/// Campaign attribution: one won deal's credit under each model, with few and many touches.
fn bench_attribution(c: &mut Criterion) {
    let mut rng = Lcg(6);
    let campaigns: Vec<Uuid> = (0..12).map(|_| rng.uuid()).collect();
    let closed = start() + contact_relationship_management::time::days(90);
    let mut group = c.benchmark_group("attribution");
    for touches in [5_usize, 50, 500] {
        let list: Vec<_> = (0..touches)
            .map(|_| attribution::Touch {
                campaign: campaigns[rng.next(campaigns.len())],
                at: start() + SignedDuration::from_hours(i64::try_from(rng.next(90 * 24)).unwrap()),
            })
            .collect();
        for (name, model) in [
            ("first", attribution::Model::First),
            ("linear", attribution::Model::Linear),
            ("time_decay", attribution::Model::TimeDecay),
            ("position", attribution::Model::Position),
        ] {
            group.bench_with_input(
                BenchmarkId::new(format!("credit/{name}"), touches),
                &list,
                |b, list| {
                    b.iter(|| attribution::credit(model, 12_345_678, black_box(list), closed));
                },
            );
        }
    }
    group.finish();
}

/// A report's money rows converted into one currency (CRM-R50), with a year of daily rates.
fn bench_fx(c: &mut Criterion) {
    let mut rng = Lcg(7);
    let first_day = Date::from_ymd(2025, 6, 1).unwrap();
    let mut rates = Vec::new();
    for day in 0..365 {
        for from in ["EUR", "USD", "JPY", "KWD"] {
            rates.push(fx::Rate {
                from: from.to_string(),
                to: "GBP".to_string(),
                ppb: 500_000_000 + i64::try_from(rng.next(1_000_000_000)).unwrap(),
                effective_on: first_day + contact_relationship_management::time::days(day),
                source: "bench".to_string(),
            });
        }
    }
    let day = Date::from_ymd(2026, 5, 31).unwrap();
    let mut group = c.benchmark_group("fx");
    for size in SIZES {
        let rows: Vec<(String, String, i64)> = (0..size)
            .map(|i| {
                (
                    format!("group {}", i % 50),
                    CURRENCIES[rng.next(CURRENCIES.len())].to_string(),
                    i64::try_from(rng.next(10_000_000)).unwrap(),
                )
            })
            .collect();
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("convert_rows", size), &rows, |b, rows| {
            b.iter(|| fx::convert_rows(black_box(rows), "GBP", day, &rates));
        });
    }
    group.finish();
}

/// Planning a report: the spec checked and turned into SQL with its bound values.
fn bench_reports(c: &mut Criterion) {
    let spec = reports::Spec {
        kind: "deal".to_string(),
        group_by: "state".to_string(),
        measure: "sum_amount".to_string(),
        filter: BTreeMap::from([(
            "currency".to_string(),
            vec!["GBP".to_string(), "EUR".to_string()],
        )]),
        from: Date::from_ymd(2026, 1, 1),
        to: Date::from_ymd(2026, 12, 31),
        convert_to: None,
        rate_date: None,
        rate_basis: None,
    };
    let mut cells = Vec::new();
    let mut rng = Lcg(8);
    for i in 0..1_000 {
        let n = i64::try_from(rng.next(40)).unwrap();
        cells.push(reports::Cell {
            group: format!("group {i}"),
            currency: None,
            value: n,
            n,
            day: None,
        });
    }
    let mut group = c.benchmark_group("reports");
    group.bench_function("plan", |b| {
        b.iter(|| reports::plan(black_box(&spec)).unwrap());
    });
    group.bench_function(BenchmarkId::new("suppress", cells.len()), |b| {
        b.iter(|| reports::suppress(black_box(cells.clone()), 5, false));
    });
    group.finish();
}

/// The search box: a typed query broken into words and turned into a `tsquery`, in Latin and in
/// a script without spaces.
fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");
    for (name, query) in [
        ("latin", "Northwind procurement Hartley renewal"),
        ("cjk", "北京分公司采购部王芳"),
    ] {
        group.bench_function(BenchmarkId::new("tsquery", name), |b| {
            b.iter(|| {
                let words = search::words(black_box(query));
                search::tsquery(&words)
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_scoring,
    bench_segment,
    bench_forecast,
    bench_audit_chain,
    bench_imports,
    bench_attribution,
    bench_fx,
    bench_reports,
    bench_search,
);
criterion_main!(benches);
