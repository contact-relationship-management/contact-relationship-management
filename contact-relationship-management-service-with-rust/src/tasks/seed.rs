//! `cargo run -- task seed` — create a synthetic book of business so
//! the CRM views demo instantly. **Synthetic data only** (spec
//! `regulatory.md`): person/organization/worker URNs are random
//! UUIDs, names are obviously fictional, no email is sent.
//!
//! The book is not in one shape (CRM-T176, CRM-R95): deals are in currencies with 0, 2 and 3
//! decimal places (JPY, KWD, GBP, EUR, INR, BRL), the exchange rates are not all from one
//! currency, and the contacts' names are written in more than one order and script, some with
//! the given and family names stored and some with the display name only.

use axa::prelude::*;
use axa::task::{TaskInfo, Vars};
use uuid::Uuid;

use crate::models::encrypted::EncryptedText;

/// The seeded deals' currency and amount in that currency's minor unit (CRM-R50): yen have no
/// decimal places, Kuwaiti dinars three, the rest two. Fictional amounts.
pub const DEAL_AMOUNTS: [(&str, i64); 12] = [
    ("GBP", 1_000_000),   // £10,000.00
    ("JPY", 1_850_000),   // ¥1,850,000
    ("KWD", 4_250_500),   // KWD 4,250.500
    ("EUR", 1_750_000),   // €17,500.00
    ("INR", 95_000_000),  // ₹950,000.00
    ("BRL", 6_400_000),   // R$64,000.00
    ("GBP", 2_500_000),   // £25,000.00
    ("JPY", 12_000_000),  // ¥12,000,000
    ("KWD", 980_250),     // KWD 980.250
    ("EUR", 3_000_000),   // €30,000.00
    ("INR", 410_000_000), // ₹4,100,000.00
    ("GBP", 3_250_000),   // £32,500.00
];

/// Exchange rates the seed sets for today, `(from, to, rate)`: several bases, not one.
pub const RATES: [(&str, &str, &str); 6] = [
    ("USD", "JPY", "151.2"),
    ("EUR", "KWD", "0.334"),
    ("INR", "BRL", "0.0652"),
    ("KWD", "GBP", "2.56"),
    ("EUR", "GBP", "0.855"),
    ("JPY", "INR", "0.553"),
];

/// Contact names `(display, given, family)`: given-name-first and family-name-first orders,
/// Latin, Han, Arabic, Devanagari, Hangul and Cyrillic scripts, and names with no parts stored
/// (CRM-R37: the parts are never derived from the display name). All fictional.
pub const CONTACT_NAMES: [(&str, Option<&str>, Option<&str>); 15] = [
    ("Imogen Hartley", Some("Imogen"), Some("Hartley")),
    ("王芳", Some("芳"), Some("王")),
    ("Nguyễn Thị Lan", Some("Lan"), Some("Nguyễn")),
    ("نور حداد", Some("نور"), Some("حداد")),
    ("田中 博", Some("博"), Some("田中")),
    ("प्रिया वेंकटरमन", None, None),
    ("김민준", Some("민준"), Some("김")),
    ("Rafael Okonkwo", None, None),
    ("Анна Соколова", Some("Анна"), Some("Соколова")),
    ("Tanaka Hiroshi", Some("Hiroshi"), Some("Tanaka")),
    (
        "María José García López",
        Some("María José"),
        Some("García López"),
    ),
    ("Aroha", None, None),
    ("Somchai Rattanakorn", Some("Somchai"), Some("Rattanakorn")),
    ("สมศรี ใจดี", Some("สมศรี"), Some("ใจดี")),
    ("Oluwaseun Adeyemi", Some("Oluwaseun"), Some("Adeyemi")),
];

/// The demo book-of-business seed task.
pub struct Seed;

#[async_trait]
impl Task for Seed {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "seed".to_string(),
            detail: "Seed a synthetic book of business (accounts, contacts, pipeline, deals, leads, SLAs)"
                .to_string(),
        }
    }

    /// Made-up records never reach a production database: axa refuses the task outside
    /// development and test (CRM-T218).
    fn development_only(&self) -> bool {
        true
    }

    #[allow(clippy::too_many_lines)] // one linear walk building the demo book
    async fn run(&self, ctx: &AppContext, _vars: &Vars) -> Result<()> {
        // One transaction, so a seed that fails leaves nothing half made (CRM-T216).
        let mut txn = crate::db::pool(ctx).begin().await?;
        // Accounts + contacts.
        let tiers = ["customer", "customer", "prospect", "partner", "prospect"];
        let mut account_pids = Vec::new();
        for (index, tier) in tiers.iter().enumerate() {
            let account = Uuid::new_v4();
            sqlx::query!(
                "INSERT INTO accounts (pid, organization_ref, owner_ref, display_name, tier, \
                 industry) VALUES ($1, $2, $3, $4, $5, 'technology')",
                account,
                format!("organization:{}", Uuid::new_v4()),
                format!("person:{}", Uuid::new_v4()),
                format!("Demo Account {:02}", index + 1),
                tier,
            )
            .execute(&mut *txn)
            .await?;
            account_pids.push(account);
            for c in 0..3 {
                let (display, given, family) = CONTACT_NAMES[(index * 3 + c) % CONTACT_NAMES.len()];
                // Two of three consented, one never — the segment gate is visible in the demo.
                // `preferred_channel` is sealed: binding `EncryptedText` seals it (CRM-T41).
                sqlx::query!(
                    "INSERT INTO contacts (pid, person_ref, account_pid, display_name, given_name, \
                     family_name, status, job_title, preferred_channel, marketing_consent) \
                     VALUES ($1, $2, $3, $4, $5, $6, 'active', 'Buyer', $7, $8)",
                    Uuid::new_v4(),
                    format!("person:{}", Uuid::new_v4()),
                    account,
                    display,
                    given,
                    family,
                    EncryptedText::from("email") as _,
                    if c == 2 { "never" } else { "granted" },
                )
                .execute(&mut *txn)
                .await?;
            }
        }
        // One pipeline with five stages.
        let pipeline = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO pipelines (pid, name) VALUES ($1, 'New Business')",
            pipeline,
        )
        .execute(&mut *txn)
        .await?;
        let stage_defs = [
            ("Qualification", 10, false, false),
            ("Discovery", 25, false, false),
            ("Proposal", 55, false, false),
            ("Won", 100, true, false),
            ("Lost", 0, false, true),
        ];
        let mut stage_pids = Vec::new();
        for (position, (name, probability, is_won, is_lost)) in stage_defs.iter().enumerate() {
            let stage = Uuid::new_v4();
            sqlx::query!(
                "INSERT INTO pipeline_stages (pid, pipeline_pid, name, position, \
                 probability_percent, is_won, is_lost) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                stage,
                pipeline,
                name,
                i32::try_from(position).unwrap_or(0),
                probability,
                is_won,
                is_lost,
            )
            .execute(&mut *txn)
            .await?;
            stage_pids.push(stage);
        }
        // Deals across the stages (some closed).
        for (i, (currency, amount_minor)) in DEAL_AMOUNTS.iter().enumerate() {
            let stage_index = i % 5;
            let closed = stage_index >= 3;
            sqlx::query!(
                "INSERT INTO deals (pid, account_pid, owner_ref, pipeline_pid, stage_pid, name, \
                 amount_minor, currency, expected_close_on, kanban_position, closed_at, won, \
                 lost_reason) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
                Uuid::new_v4(),
                account_pids[i % account_pids.len()],
                format!("person:{}", Uuid::new_v4()),
                pipeline,
                stage_pids[stage_index],
                format!("Demo Deal {:02}", i + 1),
                amount_minor,
                currency,
                chrono::NaiveDate::from_ymd_opt(2026, 9, 30),
                i32::try_from(i).unwrap_or(0),
                closed.then(crate::clock::now),
                stage_index == 3,
                (stage_index == 4).then_some("budget cut"),
            )
            .execute(&mut *txn)
            .await?;
        }
        // Exchange rates between several pairs, not all from one currency (CRM-R50).
        // The organization's "today", as the rate routes' (CRM-T179).
        let today = crate::controllers::organization::organization_today(&mut *txn).await?;
        for (from, to, rate) in RATES {
            let ppb = crate::rules::fx::parse_rate(rate).map_err(|why| Error::string(&why))?;
            sqlx::query!(
                "INSERT INTO fx_rates (pid, from_currency, to_currency, rate_ppb, effective_on, \
                                       source, set_by) \
                 VALUES ($1, $2, $3, $4, $5, 'synthetic demo rate', 'person:seed') \
                 ON CONFLICT (from_currency, to_currency, effective_on) DO NOTHING",
                Uuid::new_v4(),
                from,
                to,
                ppb,
                today,
            )
            .execute(&mut *txn)
            .await?;
        }
        // Leads in every status.
        for (i, status) in ["new", "new", "contacted", "qualified", "disqualified"]
            .iter()
            .enumerate()
        {
            sqlx::query!(
                "INSERT INTO leads (pid, source, display_name, email, email_domain, score, \
                 campaign_click, unsubscribed, status) \
                 VALUES ($1, $2, $3, $4, 'initech.example', $5, false, false, $6)",
                Uuid::new_v4(),
                if i % 2 == 0 { "web" } else { "referral" },
                format!("Demo Lead {:02}", i + 1),
                format!("lead{i}@initech.example"),
                if i % 2 == 0 { 10 } else { 30 },
                status,
            )
            .execute(&mut *txn)
            .await?;
        }
        // SLA policies for every priority.
        for (priority, first, resolution) in [
            ("low", 480, 4320),
            ("normal", 240, 1440),
            ("high", 60, 480),
            ("urgent", 15, 240),
        ] {
            sqlx::query!(
                "INSERT INTO sla_policies (pid, priority, first_response_minutes, \
                 resolution_minutes) VALUES ($1, $2, $3, $4)",
                Uuid::new_v4(),
                priority,
                first,
                resolution,
            )
            .execute(&mut *txn)
            .await?;
        }
        txn.commit().await?;
        tracing::info!("seeded synthetic book of business");
        println!(
            "seeded: 5 accounts, 15 contacts, 1 pipeline, {} deals, {} exchange rates, 5 leads, \
             4 SLA policies",
            DEAL_AMOUNTS.len(),
            RATES.len()
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{CONTACT_NAMES, DEAL_AMOUNTS, RATES};
    use crate::rules::currency;

    /// Basic Latin to Latin Extended-B, and Latin Extended Additional (Vietnamese).
    fn is_latin(c: char) -> bool {
        matches!(u32::from(c), 0..=0x024F | 0x1E00..=0x1EFF)
    }

    /// axa refuses a development-only task in production, so this is what keeps the synthetic
    /// book out of a real database (CRM-T218).
    #[test]
    fn the_seed_runs_only_in_development_or_test() {
        use axa::task::Task;
        assert!(super::Seed.development_only());
    }

    #[test]
    fn the_seed_has_more_than_one_exponent_and_every_code_is_known() {
        let exponents: Vec<u32> = DEAL_AMOUNTS
            .iter()
            .map(|(code, _)| currency::exponent(code).expect("an ISO 4217 code"))
            .collect();
        assert!(exponents.contains(&0), "a zero-exponent currency");
        assert!(exponents.contains(&2));
        assert!(exponents.contains(&3), "a three-exponent currency");
        for (from, to, rate) in RATES {
            assert!(currency::exponent(from).is_some() && currency::exponent(to).is_some());
            assert!(crate::rules::fx::parse_rate(rate).is_ok(), "{rate}");
        }
    }

    #[test]
    fn the_seed_rates_are_not_all_from_one_currency() {
        let mut bases: Vec<&str> = RATES.iter().map(|(from, _, _)| *from).collect();
        bases.sort_unstable();
        bases.dedup();
        assert!(bases.len() >= 3, "{bases:?}");
        assert!(RATES.iter().all(|(from, to, _)| from != to));
    }

    #[test]
    fn the_seed_names_are_in_more_than_one_order_and_script() {
        let non_latin = CONTACT_NAMES
            .iter()
            .filter(|(display, _, _)| display.chars().any(|c| c.is_alphabetic() && !is_latin(c)))
            .count();
        assert!(non_latin >= 5, "{non_latin}");
        // Family name first: the display name starts with the stored family name.
        assert!(CONTACT_NAMES.iter().any(|(display, given, family)| {
            matches!((given, family), (Some(_), Some(f)) if display.starts_with(f))
        }));
        // No parts stored: never derived from the display name (CRM-R37).
        assert!(
            CONTACT_NAMES
                .iter()
                .any(|(_, given, family)| given.is_none() && family.is_none())
        );
    }
}
