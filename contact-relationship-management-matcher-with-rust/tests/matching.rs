//! Behaviour of the matcher on realistic pairs.

use contact_relationship_management_matcher::{
    Contact, Field, MatchConfig, Matcher, Outcome, Verdict,
};

fn matcher() -> Matcher {
    Matcher::default()
}

fn outcome(result: &contact_relationship_management_matcher::MatchResult, field: Field) -> Outcome {
    result
        .fields
        .iter()
        .find(|f| f.field == field)
        .unwrap()
        .outcome
}

fn ana() -> Contact {
    Contact::new()
        .given_name("Ana")
        .family_name("Ruiz")
        .email("ana.ruiz@example.com")
        .phone("+34 612 345 678")
        .organization("Acme Widgets Inc.")
        .job_title("VP Sales")
}

#[test]
fn identical_contacts_match_with_near_certainty() {
    let r = matcher().compare(&ana(), &ana());
    assert_eq!(r.verdict, Verdict::Match);
    assert!(r.probability > 0.999_999, "{}", r.probability);
    assert!(!r.deterministic);
}

#[test]
fn the_same_person_written_differently_matches() {
    let other = Contact::new()
        .display_name("Dr Ana Ruiz")
        .email("ANA.RUIZ@example.com")
        .organization("acme widgets")
        .job_title("Vice President, Sales");
    let r = matcher().compare(&ana(), &other);
    assert_eq!(r.verdict, Verdict::Match, "{r:?}");
    assert_eq!(outcome(&r, Field::Phone), Outcome::Missing);
}

#[test]
#[allow(clippy::many_single_char_names)]
fn nicknames_initials_and_accents() {
    let a = Contact::new()
        .given_name("Robert")
        .family_name("O'Brien")
        .organization("Acme");
    let b = Contact::new()
        .given_name("Bob")
        .family_name("OBrien")
        .organization("Acme");
    let r = matcher().compare(&a, &b);
    assert_eq!(outcome(&r, Field::GivenName), Outcome::Agree);
    assert_eq!(outcome(&r, Field::FamilyName), Outcome::Agree);

    let c = Contact::new().given_name("J").family_name("Smith");
    let d = Contact::new().given_name("John").family_name("Smith");
    // An initial is weaker than a nickname: a partial agreement, still in favour.
    let initial = matcher().compare(&c, &d);
    assert_eq!(outcome(&initial, Field::GivenName), Outcome::Partial);
    assert!(initial.fields[0].weight > 0.0);

    let e = Contact::new().given_name("José").family_name("García");
    let f = Contact::new().given_name("Jose").family_name("Garcia");
    assert_eq!(matcher().compare(&e, &f).fields[0].similarity, 1.0);
}

#[test]
fn a_name_alone_is_only_worth_a_review() {
    let a = Contact::new().given_name("John").family_name("Smith");
    let r = matcher().compare(&a, &a);
    assert_eq!(r.verdict, Verdict::Review, "{}", r.probability);
    assert!((0.5..0.9).contains(&r.probability));
}

#[test]
fn a_shared_name_at_different_organisations_does_not_match() {
    let a = Contact::new()
        .given_name("John")
        .family_name("Smith")
        .organization("Acme");
    let b = Contact::new()
        .given_name("John")
        .family_name("Smith")
        .organization("Globex");
    let r = matcher().compare(&a, &b);
    assert_ne!(r.verdict, Verdict::Match, "{}", r.probability);
    assert_eq!(outcome(&r, Field::Organization), Outcome::Disagree);
}

#[test]
fn different_people_do_not_match() {
    let b = Contact::new()
        .given_name("Ben")
        .family_name("Okafor")
        .email("ben@globex.test")
        .organization("Globex");
    let r = matcher().compare(&ana(), &b);
    assert_eq!(r.verdict, Verdict::NonMatch);
    assert!(r.probability < 0.001, "{}", r.probability);
}

#[test]
fn a_shared_email_outweighs_a_different_name() {
    // A married name: same mailbox, same phone, new family name.
    let a = Contact::new()
        .given_name("Ana")
        .family_name("Ruiz")
        .email("ana@example.com")
        .phone("+34 612 345 678");
    let b = Contact::new()
        .given_name("Ana")
        .family_name("Berg")
        .email("ana@example.com")
        .phone("0034 612 345 678");
    let r = matcher().compare(&a, &b);
    assert_eq!(outcome(&r, Field::FamilyName), Outcome::Disagree);
    assert_eq!(r.verdict, Verdict::Match, "{}", r.probability);
}

#[test]
fn swapped_given_and_family_names_are_noticed() {
    let a = Contact::new()
        .given_name("Ana")
        .family_name("Ruiz")
        .email("a@example.com");
    let b = Contact::new()
        .given_name("Ruiz")
        .family_name("Ana")
        .email("a@example.com");
    let r = matcher().compare(&a, &b);
    assert!(r.names_swapped);
    assert_eq!(r.verdict, Verdict::Match);
    assert!(!matcher().compare(&a, &a).names_swapped);
}

#[test]
fn a_shared_person_ref_is_certain_even_when_nothing_else_agrees() {
    let a = Contact::new()
        .person_ref("person:123")
        .given_name("Ana")
        .family_name("Ruiz");
    let b = Contact::new()
        .person_ref("person:123")
        .given_name("Ben")
        .family_name("Okafor");
    let r = matcher().compare(&a, &b);
    assert!(r.deterministic);
    assert_eq!(r.probability, 1.0);
    assert_eq!(r.verdict, Verdict::Match);
    // Different references say nothing by themselves.
    let c = Contact::new()
        .person_ref("person:999")
        .given_name("Ana")
        .family_name("Ruiz");
    let r = matcher().compare(&a, &c);
    assert!(!r.deterministic);
    // Blank references are not references.
    let blank = Contact::new().person_ref("  ");
    assert!(!matcher().compare(&blank, &blank).deterministic);
}

#[test]
fn empty_contacts_carry_no_evidence() {
    let r = matcher().compare(&Contact::new(), &Contact::new());
    assert!(
        r.fields
            .iter()
            .all(|f| f.outcome == Outcome::Missing && f.weight == 0.0)
    );
    assert_eq!(r.verdict, Verdict::NonMatch);
    assert!((r.probability - MatchConfig::default().prior).abs() < 1e-9);
}

#[test]
fn comparison_is_symmetric() {
    let people = [
        ana(),
        Contact::new()
            .display_name("Ruiz, Ana")
            .email("ana@example.com"),
        Contact::new().given_name("Ruiz").family_name("Ana"),
        Contact::new()
            .given_name("Robert")
            .family_name("Smith")
            .phone("020 7946 0958"),
        Contact::new()
            .given_name("Bob")
            .family_name("Smith")
            .phone("+44 20 7946 0958"),
        Contact::new().person_ref("p1"),
        Contact::new().organization("Acme").job_title("CTO"),
        Contact::new(),
    ];
    for a in &people {
        for b in &people {
            assert_eq!(
                matcher().compare(a, b),
                matcher().compare(b, a),
                "{a:?} / {b:?}"
            );
        }
    }
}

#[test]
fn probabilities_stay_in_range_and_serialise() {
    let r = matcher().compare(&ana(), &ana());
    assert!((0.0..=1.0).contains(&r.probability));
    let json = serde_json::to_string(&r).unwrap();
    assert_eq!(
        serde_json::from_str::<contact_relationship_management_matcher::MatchResult>(&json)
            .unwrap(),
        r
    );
    let deterministic = Contact::new().person_ref("x");
    let r = matcher().compare(&deterministic, &deterministic);
    assert!(serde_json::to_string(&r).unwrap().contains("40.0"));
}

#[test]
fn ranking_orders_by_probability_then_position() {
    let query = ana();
    let candidates = vec![
        Contact::new().given_name("Ben").family_name("Okafor"), // 0: unrelated
        Contact::new().given_name("Ana").family_name("Ruiz"),   // 1: name only
        ana(),                                                  // 2: everything
        ana(),                                                  // 3: tie with 2
    ];
    let ranked = matcher().rank_one_to_many(&query, &candidates);
    let order: Vec<usize> = ranked.iter().map(|r| r.index).collect();
    assert_eq!(order, vec![2, 3, 1]); // the non-match is left out
    assert_eq!(matcher().compare_one_to_many(&query, &candidates).len(), 4);
}

#[test]
fn configuration_is_validated() {
    let bad = |edit: fn(&mut MatchConfig)| {
        let mut c = MatchConfig::default();
        edit(&mut c);
        Matcher::new(c).unwrap_err().to_string()
    };
    assert!(bad(|c| c.prior = 0.0).contains("prior"));
    assert!(bad(|c| c.prior = f64::NAN).contains("prior"));
    assert!(bad(|c| c.review_threshold = 0.995).contains("review_threshold"));
    assert!(bad(|c| c.disagree_at = 0.95).contains("disagree_at"));
    assert!(bad(|c| c.email.m = 0.0001).contains("email.m"));
    assert!(bad(|c| c.phone.u = 1.0).contains("phone.u"));
    assert!(bad(|c| c.default_calling_code = Some("+44".into())).contains("calling"));
    assert!(Matcher::new(MatchConfig::default()).is_ok());
}

#[test]
fn configuration_loads_from_partial_json() {
    let config: MatchConfig =
        serde_json::from_str(r#"{ "prior": 0.05, "default_calling_code": "44" }"#).unwrap();
    assert_eq!(config.prior, 0.05);
    assert_eq!(
        config.match_threshold,
        MatchConfig::default().match_threshold
    );
    let matcher = Matcher::new(config).unwrap();
    // National and international forms now agree because the code is known.
    let a = Contact::new().phone("020 7946 0958");
    let b = Contact::new().phone("+44 20 7946 0958");
    assert_eq!(
        outcome(&matcher.compare(&a, &b), Field::Phone),
        Outcome::Agree
    );
}

#[test]
fn custom_nicknames_count() {
    let a = Contact::new().given_name("Zbigniew").family_name("Nowak");
    let b = Contact::new().given_name("Zbyszek").family_name("Nowak");
    assert_ne!(
        outcome(&matcher().compare(&a, &b), Field::GivenName),
        Outcome::Agree
    );
    let config = MatchConfig {
        nicknames: vec![vec!["Zbigniew".into(), "Zbyszek".into()]],
        ..MatchConfig::default()
    };
    let custom = Matcher::new(config).unwrap();
    assert_eq!(
        outcome(&custom.compare(&a, &b), Field::GivenName),
        Outcome::Agree
    );
}

#[test]
fn more_agreement_never_lowers_the_probability() {
    let base = Contact::new().given_name("Ana").family_name("Ruiz");
    let mut other = Contact::new().given_name("Ana").family_name("Ruiz");
    let mut last = matcher().compare(&base, &other).probability;
    for step in 0..3 {
        match step {
            0 => other = other.organization("Acme"),
            1 => other = other.phone("+34 612 345 678"),
            _ => other = other.email("ana@example.com"),
        }
        let base_step = match step {
            0 => base.clone().organization("Acme"),
            1 => base.clone().organization("Acme").phone("+34 612 345 678"),
            _ => base
                .clone()
                .organization("Acme")
                .phone("+34 612 345 678")
                .email("ana@example.com"),
        };
        let p = matcher().compare(&base_step, &other).probability;
        assert!(p >= last, "step {step}: {p} < {last}");
        last = p;
    }
}
