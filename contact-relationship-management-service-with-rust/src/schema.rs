//! The schema, on `sqlx`'s migrator (CRM-D24, CRM-T229).
//!
//! `migrations/` starts with one baseline: the schema the 59 `SeaORM` migrations built, as one
//! file. A new database runs it. A database those migrations built already has that schema, so
//! [`migrate`] records the baseline as applied without running it (`Migrator::skip`), then runs
//! what comes after. A database with only part of that history is refused: the baseline would
//! skip the migrations it lacks. Its `seaql_migrations` table is left where it is.

use axa::prelude::*;
use sqlx::PgPool;
use sqlx::migrate::Migrator;

/// Every migration in `migrations/`, built in.
pub static MIGRATOR: Migrator = sqlx::migrate!();

/// The baseline's version: the schema [`HISTORY`] built.
pub const BASELINE: i64 = 20_261_110_000_000;

/// The `SeaORM` migrations the baseline stands for, in order, as `seaql_migrations` names them.
pub const HISTORY: [&str; 59] = [
    "m20220101_000001_relationships",
    "m20220101_000002_sales",
    "m20220101_000003_marketing",
    "m20220101_000004_support",
    "m20220101_000005_audit_logs",
    "m20220101_000006_event_outbox",
    "m20260720_000007_engagement",
    "m20261010_000008_audit_chain",
    "m20261010_000009_break_glass",
    "m20261011_000010_idempotency_keys",
    "m20261011_000011_email_deliveries",
    "m20261011_000012_email_events",
    "m20261012_000013_nurture_step_content",
    "m20261013_000014_delivery_provider_id",
    "m20261017_000015_board_position",
    "m20261018_000016_touch_updated_at",
    "m20261019_000017_contact_merges",
    "m20261020_000018_contact_unmerge",
    "m20261020_000019_external_links",
    "m20261024_000020_webhooks",
    "m20261024_000021_consent_purposes",
    "m20261024_000022_web_forms",
    "m20261024_000023_notifications",
    "m20261024_000024_search",
    "m20261025_000025_lead_imports",
    "m20261025_000026_views",
    "m20261025_000027_email_templates",
    "m20261025_000028_contact_family_name",
    "m20261025_000029_contact_given_name",
    "m20261025_000030_workflows",
    "m20261025_000031_custom_fields",
    "m20261025_000032_attachments",
    "m20261025_000033_record_imports",
    "m20261025_000034_lead_owner",
    "m20261025_000035_business_calendars",
    "m20261025_000036_tags",
    "m20261025_000037_reports",
    "m20261026_000038_fx_rates",
    "m20261026_000039_ticket_pauses",
    "m20261026_000040_kb_daily",
    "m20261026_000041_email_engagement",
    "m20261026_000042_quotas",
    "m20261027_000043_wait_limit",
    "m20261027_000044_deal_splits",
    "m20261027_000045_teams",
    "m20261028_000046_territories",
    "m20261028_000047_wait_limit_action",
    "m20261028_000048_clawbacks",
    "m20261028_000049_deal_deliveries",
    "m20261028_000050_link_tokens",
    "m20261028_000051_auto_close_days",
    "m20261030_000052_wpm_employments",
    "m20261030_000053_employment_last_events",
    "m20261030_000054_person_user_refs",
    "m20261030_000055_search_ngrams",
    "m20261030_000056_template_language",
    "m20261030_000057_organization_settings",
    "m20261031_000058_deal_stalled",
    "m20261031_000059_consent_profiles",
];

fn failed(why: impl std::fmt::Display) -> Error {
    Error::string(&format!("migration failed: {why}"))
}

/// Whether a database whose `seaql_migrations` lists `applied` can take the baseline as applied:
/// it must list every migration in [`HISTORY`] and nothing else.
///
/// # Errors
///
/// Why it cannot, to show the operator.
pub fn adoptable(applied: &[String]) -> std::result::Result<(), String> {
    let unknown: Vec<&str> = applied
        .iter()
        .map(String::as_str)
        .filter(|name| !HISTORY.contains(name))
        .collect();
    if !unknown.is_empty() {
        return Err(format!(
            "seaql_migrations lists migrations this build does not know: {}",
            unknown.join(", ")
        ));
    }
    let missing = HISTORY
        .iter()
        .filter(|name| !applied.iter().any(|done| done == *name))
        .count();
    if missing > 0 {
        return Err(format!(
            "seaql_migrations lacks {missing} of the {} SeaORM migrations; migrate this database \
             with an image from before CRM-T229 first",
            HISTORY.len()
        ));
    }
    Ok(())
}

/// Apply pending migrations. A database the `SeaORM` history built takes the baseline as applied
/// first, and runs none of it. `sqlx` holds a lock while it migrates, so replicas starting together
/// take turns, and a second one finds the baseline recorded.
///
/// # Errors
///
/// When the history is partial or unknown, a migration fails, or an applied migration has changed.
pub async fn migrate(pool: &PgPool) -> Result<()> {
    let history: bool =
        sqlx::query_scalar("SELECT to_regclass('public.seaql_migrations') IS NOT NULL")
            .fetch_one(pool)
            .await?;
    if history {
        // Not a checked query: a database `sqlx` built has no such table.
        let applied: Vec<String> = sqlx::query_scalar("SELECT version FROM seaql_migrations")
            .fetch_all(pool)
            .await?;
        adoptable(&applied).map_err(failed)?;
        MIGRATOR.skip(pool, Some(BASELINE)).await.map_err(failed)?;
    }
    MIGRATOR.run(pool).await.map_err(failed)
}

/// Drop everything in `public`, `seaql_migrations` too, and migrate from nothing. **This destroys
/// all data**; `db reset` refuses in production, and tests boot with it.
///
/// # Errors
///
/// A database error, or a failed migration.
pub async fn reset(pool: &PgPool) -> Result<()> {
    sqlx::raw_sql("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
        .execute(pool)
        .await?;
    migrate(pool).await
}

/// Each migration and whether it has run, oldest first, as `applied 20261110000000 baseline`.
///
/// # Errors
///
/// A database error.
pub async fn status(pool: &PgPool) -> Result<Vec<String>> {
    let recorded: bool =
        sqlx::query_scalar("SELECT to_regclass('public._sqlx_migrations') IS NOT NULL")
            .fetch_one(pool)
            .await?;
    let applied: Vec<i64> = if recorded {
        sqlx::query_scalar("SELECT version FROM _sqlx_migrations WHERE success")
            .fetch_all(pool)
            .await?
    } else {
        Vec::new()
    };
    Ok(MIGRATOR
        .iter()
        .filter(|migration| !migration.migration_type.is_down_migration())
        .map(|migration| {
            let state = if applied.contains(&migration.version) {
                "applied"
            } else {
                "pending"
            };
            format!("{state} {} {}", migration.version, migration.description)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn the_history_is_the_seaorm_migrator_s_list() {
        use migration::MigratorTrait as _;
        let seaorm: Vec<String> = migration::Migrator::migrations()
            .iter()
            .map(|m| m.name().to_string())
            .collect();
        assert_eq!(seaorm, names(&HISTORY));
    }

    #[test]
    fn the_baseline_is_the_first_migration() {
        let first = MIGRATOR.iter().next().expect("a migration");
        assert_eq!(first.version, BASELINE);
        assert_eq!(first.description, "baseline");
    }

    #[test]
    fn only_the_whole_history_takes_the_baseline() {
        assert_eq!(adoptable(&names(&HISTORY)), Ok(()));
        let mut reversed = names(&HISTORY);
        reversed.reverse();
        assert_eq!(adoptable(&reversed), Ok(()), "order is not compared");
        let partial = names(&HISTORY[..58]);
        assert!(
            adoptable(&partial)
                .unwrap_err()
                .contains("lacks 1 of the 59")
        );
        let mut extra = names(&HISTORY);
        extra.push("m20261101_000060_unknown".to_string());
        assert!(
            adoptable(&extra)
                .unwrap_err()
                .contains("m20261101_000060_unknown")
        );
        assert!(adoptable(&[]).is_err(), "an empty table is not the history");
    }
}
