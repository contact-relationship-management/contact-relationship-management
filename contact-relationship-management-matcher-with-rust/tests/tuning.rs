//! Measures the matcher against a labelled set of synthetic contact pairs.
//!
//! `tests/fixtures/pairs.json` holds pairs, each labelled `same` or not. This
//! test scores them, prints a report (run with `-- --nocapture`), and pins the
//! precision and recall as floors so a later change cannot quietly make the
//! matcher worse. The pairs are synthetic and were written by the people who
//! wrote the matcher: see the README's "Tuning" section for what that does and
//! does not show.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers fail the test on bad fixtures"
)]
// Counts here are in the hundreds, so usize-to-f64 is exact; short names are the maths.
#![allow(clippy::cast_precision_loss, clippy::many_single_char_names)]

use contact_relationship_management_matcher::{Contact, Matcher, Verdict};
use serde::Deserialize;

#[derive(Deserialize)]
struct Pair {
    same: bool,
    category: String,
    a: Contact,
    b: Contact,
}

fn pairs() -> Vec<Pair> {
    serde_json::from_str(include_str!("fixtures/pairs.json")).expect("fixture parses")
}

/// The same pair as a service sees it before it fetches email: no emails or
/// phones.
fn without_contact_details(mut c: Contact) -> Contact {
    c.emails.clear();
    c.phones.clear();
    c
}

struct Scored {
    same: bool,
    category: String,
    probability: f64,
    verdict: Verdict,
}

fn score(pairs: &[Pair], strip: bool) -> Vec<Scored> {
    let matcher = Matcher::default();
    pairs
        .iter()
        .map(|p| {
            let (a, b) = if strip {
                (
                    without_contact_details(p.a.clone()),
                    without_contact_details(p.b.clone()),
                )
            } else {
                (p.a.clone(), p.b.clone())
            };
            let r = matcher.compare(&a, &b);
            Scored {
                same: p.same,
                category: p.category.clone(),
                probability: r.probability,
                verdict: r.verdict,
            }
        })
        .collect()
}

/// Precision and recall when everything at or above `threshold` is called a match.
fn at(scored: &[Scored], threshold: f64) -> (f64, f64) {
    let tp = scored
        .iter()
        .filter(|s| s.same && s.probability >= threshold)
        .count() as f64;
    let fp = scored
        .iter()
        .filter(|s| !s.same && s.probability >= threshold)
        .count() as f64;
    let fn_ = scored
        .iter()
        .filter(|s| s.same && s.probability < threshold)
        .count() as f64;
    let precision = if tp + fp == 0.0 { 1.0 } else { tp / (tp + fp) };
    let recall = if tp + fn_ == 0.0 {
        1.0
    } else {
        tp / (tp + fn_)
    };
    (precision, recall)
}

fn report(title: &str, scored: &[Scored]) {
    println!("\n== {title} ({} pairs) ==", scored.len());
    println!("threshold  precision  recall");
    for t in [0.3, 0.5, 0.7, 0.8, 0.9, 0.95, 0.99, 0.999] {
        let (p, r) = at(scored, t);
        println!("{t:>9}  {p:>9.3}  {r:>6.3}");
    }
    println!("calibration: stated probability vs observed share that were the same person");
    let edges = [0.0, 0.1, 0.3, 0.5, 0.7, 0.9, 0.99, 1.000_001];
    for w in edges.windows(2) {
        let bin: Vec<_> = scored
            .iter()
            .filter(|s| s.probability >= w[0] && s.probability < w[1])
            .collect();
        if bin.is_empty() {
            continue;
        }
        let same = bin.iter().filter(|s| s.same).count();
        let mean = bin.iter().map(|s| s.probability).sum::<f64>() / bin.len() as f64;
        println!(
            "[{:.2}, {:.2})  n={:>3}  stated {:.3}  observed {:.3}",
            w[0],
            w[1].min(1.0),
            bin.len(),
            mean,
            same as f64 / bin.len() as f64
        );
    }
    println!("verdicts: Match / Review / NonMatch by truth");
    for (label, truth) in [("same", true), ("different", false)] {
        let count = |v: Verdict| {
            scored
                .iter()
                .filter(|s| s.same == truth && s.verdict == v)
                .count()
        };
        println!(
            "{label:>9}: {} / {} / {}",
            count(Verdict::Match),
            count(Verdict::Review),
            count(Verdict::NonMatch)
        );
    }
    println!("misjudged (same below review, or different at match):");
    for s in scored {
        let bad =
            (s.same && s.verdict == Verdict::NonMatch) || (!s.same && s.verdict == Verdict::Match);
        if bad {
            println!("  {:.4} {} (same={})", s.probability, s.category, s.same);
        }
    }
}

#[test]
fn the_fixture_has_at_least_as_many_hard_cases_as_easy_ones() {
    let pairs = pairs();
    assert!(pairs.len() >= 100);
    let easy = ["exact", "different-names"];
    let easy_n = pairs
        .iter()
        .filter(|p| easy.iter().any(|e| p.category.starts_with(e)))
        .count();
    assert!(
        easy_n * 2 <= pairs.len(),
        "{easy_n} easy of {}",
        pairs.len()
    );
}

#[test]
fn measure() {
    let pairs = pairs();
    let full = score(&pairs, false);
    let names = score(&pairs, true);
    report("full records", &full);
    report("names, employer and title only", &names);

    // Floors, set a little below what was measured, so a change that makes
    // the matcher worse fails here. Raise them when the matcher improves.
    let config = Matcher::default().config().clone();
    let (precision, recall) = at(&full, config.match_threshold);
    assert!(precision >= 0.85, "match precision {precision:.3}");
    assert!(recall >= 0.85, "match recall {recall:.3}");
    let (_, review_recall) = at(&full, config.review_threshold);
    assert!(review_recall >= 0.95, "review recall {review_recall:.3}");
    let (_, names_recall) = at(&names, config.review_threshold);
    assert!(
        names_recall >= 0.85,
        "names-only review recall {names_recall:.3}"
    );
    let wrong_matches = full
        .iter()
        .filter(|s| !s.same && s.verdict == Verdict::Match)
        .count();
    assert!(
        wrong_matches <= 6,
        "{wrong_matches} different pairs called Match"
    );
}

fn f1_at(scored: &[Scored], t: f64) -> f64 {
    let (p, r) = at(scored, t);
    if p + r == 0.0 {
        0.0
    } else {
        2.0 * p * r / (p + r)
    }
}

fn score_with(
    pairs: &[Pair],
    config: contact_relationship_management_matcher::MatchConfig,
) -> Vec<Scored> {
    let matcher = Matcher::new(config).unwrap();
    pairs
        .iter()
        .map(|p| {
            let r = matcher.compare(&p.a, &p.b);
            Scored {
                same: p.same,
                category: p.category.clone(),
                probability: r.probability,
                verdict: r.verdict,
            }
        })
        .collect()
}

#[test]
#[ignore = "exploration: prints a parameter sweep"]
#[allow(clippy::field_reassign_with_default)]
fn sweep() {
    use contact_relationship_management_matcher::MatchConfig;
    let pairs = pairs();
    let mut rows = vec![];
    for agree in [0.92, 0.94, 0.96, 0.98] {
        for dis in [0.7, 0.8, 0.85, 0.9] {
            if dis >= agree {
                continue;
            }
            for prior in [0.001, 0.01, 0.05] {
                for (gu, fu) in [(0.03, 0.01), (0.05, 0.02), (0.08, 0.04)] {
                    for (ou, tu) in [(0.02, 0.05), (0.05, 0.1), (0.1, 0.2)] {
                        let mut c = MatchConfig::default();
                        c.agree_at = agree;
                        c.disagree_at = dis;
                        c.prior = prior;
                        c.given_name.u = gu;
                        c.family_name.u = fu;
                        c.organization.u = ou;
                        c.job_title.u = tu;
                        let s = score_with(&pairs, c);
                        let best = [0.5, 0.7, 0.9, 0.95, 0.99]
                            .iter()
                            .map(|&t| (f1_at(&s, t), t))
                            .fold((0.0, 0.0), |a, b| if b.0 > a.0 { b } else { a });
                        rows.push((best.0, best.1, agree, dis, prior, gu, fu, ou, tu));
                    }
                }
            }
        }
    }
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    for r in rows.iter().take(25) {
        println!(
            "F1 {:.3} @{} agree {} dis {} prior {} g.u {} f.u {} o.u {} t.u {}",
            r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8
        );
    }
}
