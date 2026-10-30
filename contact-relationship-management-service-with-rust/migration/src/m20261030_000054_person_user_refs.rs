//! Migration: a CRM user is named by `person:` URN, not `worker:` (CRM-D18).
//!
//! The CRM wrote its signed-in user as `worker:<sign-in subject>`, and took `worker:<id>` for every
//! owner, assignee, recipient, quota holder and team member. The family's sign-in subject is a
//! person id (WPM matches it as `person:<sub>`; PPM addresses it so), so each of those held a person
//! id under the worker prefix. This rewrites `worker:<uuid>` to `person:<uuid>`, keeping the id, in
//! every column that names a CRM user, in the user references inside workflow actions, and in the
//! members of teams set by hand.
//!
//! **Left alone, on purpose:**
//! - members of a team whose latest `set` was the WPM import (`via: wpm` in its audit): an import
//!   before CRM-T161 stored `worker:<WPM worker pid>`, a real worker id that is not a person id, and
//!   the next import replaces them (CRM-D17);
//! - history: the audit log (hash-chained, append-only), the outbox, workflow run logs, stored
//!   idempotent answers;
//! - WPM's own `worker:` subjects in `wpm_events` (they do name WPM workers);
//! - a row whose rewrite would collide with a `person:` row already holding the same unique key
//!   (none can exist from the API, which took `worker:` only, but a hand-made one is not broken).
//!
//! `updated_at` is not moved: the reference is spelt differently, nobody edited the record.
//! `down` turns `person:<uuid>` back into `worker:<uuid>` in the same places, except team members a
//! WPM import set (those are `person_ref`s from WPM and were `person:` before this migration too).

use std::fmt::Write as _;

use sea_orm_migration::prelude::*;

/// A UUID, as a Postgres regular expression.
const UUID: &str = "[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}";

/// The tables with the `crm_touch_updated_at` trigger among those rewritten.
const TOUCHED: [&str; 7] = [
    "accounts",
    "activities",
    "contacts",
    "deals",
    "leads",
    "sla_policies",
    "tickets",
];

/// `(table, column, the other parts of a unique key the column is in)`. A key part is an
/// expression in which `{a}` stands for the table's alias.
const COLUMNS: [(&str, &str, &[&str]); 23] = [
    ("contacts", "owner_ref", &[]),
    ("accounts", "owner_ref", &[]),
    ("leads", "owner_ref", &[]),
    ("deals", "owner_ref", &[]),
    ("activities", "actor_ref", &[]),
    ("activities", "assignee_ref", &[]),
    ("tickets", "assignee_ref", &[]),
    ("sla_policies", "escalate_to", &[]),
    (
        "notifications",
        "recipient_ref",
        &["{a}.kind", "{a}.source_pid", "{a}.source_key"],
    ),
    ("saved_views", "owner_ref", &["{a}.kind", "lower({a}.name)"]),
    ("reports", "owner_ref", &["lower({a}.name)"]),
    (
        "quotas",
        "owner_ref",
        &["{a}.period_from", "{a}.period_to", "{a}.currency"],
    ),
    ("quotas", "set_by", &[]),
    ("commission_plans", "set_by", &[]),
    ("deal_splits", "owner_ref", &["{a}.deal_pid"]),
    ("deal_splits", "set_by", &[]),
    ("deal_clawbacks", "owner_ref", &["{a}.deal_pid"]),
    ("deal_clawbacks", "set_by", &[]),
    ("territories", "owner_ref", &[]),
    ("territories", "set_by", &[]),
    ("fx_rates", "set_by", &[]),
    ("teams", "set_by", &[]),
    ("deal_deliveries", "opened_by", &[]),
];

/// The rewrite of one column from prefix `from` to prefix `to` (both seven characters, so the id
/// starts at the eighth).
fn column_sql(table: &str, column: &str, key: &[&str], from: &str, to: &str) -> String {
    let mut sql = format!(
        "UPDATE {table} AS t SET {column} = '{to}' || substr(t.{column}, 8) \
         WHERE t.{column} ~ '^{from}{UUID}$'"
    );
    if !key.is_empty() {
        let same: Vec<String> = key
            .iter()
            .map(|part| {
                format!(
                    "{} IS NOT DISTINCT FROM {}",
                    part.replace("{a}", "o"),
                    part.replace("{a}", "t")
                )
            })
            .collect();
        let _ = write!(
            sql,
            " AND NOT EXISTS (SELECT 1 FROM {table} AS o WHERE o.{column} = '{to}' || \
             substr(t.{column}, 8) AND {})",
            same.join(" AND ")
        );
    }
    sql
}

/// A JSON string value `v` rewritten when it is `<from><uuid>`.
fn json_ref(v: &str, from: &str, to: &str) -> String {
    format!(
        "CASE WHEN ({v}) #>> '{{}}' ~ '^{from}{UUID}$' \
         THEN to_jsonb('{to}' || substr(({v}) #>> '{{}}', 8)) ELSE ({v}) END"
    )
}

/// A JSON array of references rewritten entry by entry, in order.
fn json_refs(array: &str, from: &str, to: &str) -> String {
    format!(
        "(SELECT coalesce(jsonb_agg({} ORDER BY i), '[]'::jsonb) \
         FROM jsonb_array_elements({array}) WITH ORDINALITY AS x(m, i))",
        json_ref("m", from, to)
    )
}

/// The user references in workflow actions: a task's assignee, a notification's recipient, an
/// owner pool, and an `owner_ref` a `set_field` sets. Nothing else in an action is touched.
fn workflows_sql(from: &str, to: &str) -> String {
    let field = |name: &str| {
        format!(
            "jsonb_set(a, '{{{name}}}', {})",
            json_ref(&format!("a->'{name}'"), from, to)
        )
    };
    format!(
        "UPDATE workflows AS w SET actions = (\
           SELECT jsonb_agg(CASE \
             WHEN a->>'type' = 'create_task' AND a ? 'assignee_ref' \
               AND jsonb_typeof(a->'assignee_ref') = 'string' THEN {task} \
             WHEN a->>'type' = 'notify' AND jsonb_typeof(a->'recipient_ref') = 'string' THEN {notify} \
             WHEN a->>'type' = 'set_field' AND a->>'field' = 'owner_ref' \
               AND jsonb_typeof(a->'value') = 'string' THEN {set} \
             WHEN a->>'type' = 'assign_owner' AND jsonb_typeof(a->'pool') = 'array' \
               THEN jsonb_set(a, '{{pool}}', {pool}) \
             ELSE a END ORDER BY n) \
           FROM jsonb_array_elements(w.actions) WITH ORDINALITY AS e(a, n)) \
         WHERE jsonb_typeof(w.actions) = 'array' AND jsonb_array_length(w.actions) > 0 \
           AND w.actions::text LIKE '%\"{from}%'",
        task = field("assignee_ref"),
        notify = field("recipient_ref"),
        set = field("value"),
        pool = json_refs("a->'pool'", from, to),
    )
}

/// Team members, in teams whose latest `set` was not the WPM import.
fn teams_sql(from: &str, to: &str) -> String {
    format!(
        "UPDATE teams AS t SET members = {} \
         WHERE jsonb_typeof(t.members) = 'array' AND t.members::text LIKE '%\"{from}%' \
           AND coalesce((SELECT l.snapshot->>'via' FROM audit_logs AS l \
                         WHERE l.entity = 'team' AND l.entity_pid = t.pid AND l.action = 'set' \
                         ORDER BY l.id DESC LIMIT 1), '') <> 'wpm'",
        json_refs("t.members", from, to)
    )
}

/// Every statement of the rewrite, in order.
fn statements(from: &str, to: &str) -> Vec<String> {
    let mut out: Vec<String> = TOUCHED
        .iter()
        .map(|t| format!("ALTER TABLE {t} DISABLE TRIGGER crm_touch_updated_at"))
        .collect();
    out.extend(
        COLUMNS
            .iter()
            .map(|(table, column, key)| column_sql(table, column, key, from, to)),
    );
    out.push(workflows_sql(from, to));
    out.push(teams_sql(from, to));
    out.extend(
        TOUCHED
            .iter()
            .map(|t| format!("ALTER TABLE {t} ENABLE TRIGGER crm_touch_updated_at")),
    );
    out
}

/// The user-reference migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Rewrite `worker:<uuid>` user references to `person:<uuid>`.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        for sql in statements("worker:", "person:") {
            conn.execute_unprepared(&sql).await?;
        }
        Ok(())
    }

    /// Put `worker:<uuid>` back, except in teams the WPM import set.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        for sql in statements("person:", "worker:") {
            conn.execute_unprepared(&sql).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_column_keeps_the_id_and_only_matches_a_whole_uuid() {
        let sql = column_sql("deals", "owner_ref", &[], "worker:", "person:");
        assert!(
            sql.starts_with(
                "UPDATE deals AS t SET owner_ref = 'person:' || substr(t.owner_ref, 8)"
            )
        );
        assert!(sql.contains(&format!("~ '^worker:{UUID}$'")), "{sql}");
        assert!(!sql.contains("NOT EXISTS"), "{sql}");
    }

    #[test]
    fn a_unique_column_skips_a_row_that_would_collide() {
        let sql = column_sql(
            "saved_views",
            "owner_ref",
            &["{a}.kind", "lower({a}.name)"],
            "worker:",
            "person:",
        );
        assert!(sql.contains("NOT EXISTS (SELECT 1 FROM saved_views AS o WHERE o.owner_ref = 'person:' || substr(t.owner_ref, 8)"), "{sql}");
        assert!(sql.contains("o.kind IS NOT DISTINCT FROM t.kind"), "{sql}");
        assert!(
            sql.contains("lower(o.name) IS NOT DISTINCT FROM lower(t.name)"),
            "{sql}"
        );
    }

    #[test]
    fn the_prefixes_are_the_same_length_so_the_id_starts_at_the_eighth_character() {
        assert_eq!("worker:".len(), 7);
        assert_eq!("person:".len(), 7);
    }

    #[test]
    fn every_column_is_rewritten_once_and_the_triggers_are_put_back() {
        let all = statements("worker:", "person:");
        let mut seen = std::collections::BTreeSet::new();
        for (table, column, _) in COLUMNS {
            assert!(seen.insert((table, column)), "{table}.{column} twice");
        }
        assert_eq!(all.len(), TOUCHED.len() * 2 + COLUMNS.len() + 2);
        for t in TOUCHED {
            let off = all.iter().position(|s| {
                s == &format!("ALTER TABLE {t} DISABLE TRIGGER crm_touch_updated_at")
            });
            let on = all
                .iter()
                .position(|s| s == &format!("ALTER TABLE {t} ENABLE TRIGGER crm_touch_updated_at"));
            assert!(off < on, "{t}");
        }
    }

    #[test]
    fn teams_set_by_the_import_and_wpm_subjects_are_left_alone() {
        let teams = teams_sql("worker:", "person:");
        assert!(teams.contains("<> 'wpm'"), "{teams}");
        let all = statements("worker:", "person:").join("\n");
        for untouched in [
            "wpm_events",
            "audit_logs SET",
            "event_outbox",
            "workflow_runs",
        ] {
            assert!(!all.contains(&format!("UPDATE {untouched}")), "{untouched}");
        }
    }
}
