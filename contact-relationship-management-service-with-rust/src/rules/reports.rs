//! The report builder (CRM-R48): what a report may ask, and the query that answers it.
//!
//! A report is one kind of record, one thing to group by, one measure, and some filters. Every part
//! is chosen from a closed list, and the SQL is put together from fixed pieces of text with the
//! caller's values bound, never written into it. Pure and DB-free: [`plan`] returns the query and
//! what to bind to it, and the controller runs it.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::tokens;

/// The records a report can be about.
pub const KINDS: &[&str] = &["contact", "account", "lead", "deal", "ticket"];
/// The measures: a count, and for deals the sum of the amounts, always per currency.
pub const MEASURES: &[&str] = &["count", "sum_amount"];
/// Most groups a report returns; more is flagged, not hidden.
pub const MAX_GROUPS: usize = 200;
/// Most reports one worker keeps.
pub const MAX_PER_OWNER: i64 = 50;
/// Longest report name.
pub const MAX_NAME: usize = 80;
/// Most values in one filter.
const MAX_VALUES: usize = 20;

/// What a report asks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "ReportSpec")]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// `contact`, `account`, `lead`, `deal` or `ticket`.
    pub kind: String,
    /// What to group by; see [`fields`] for each kind.
    pub group_by: String,
    /// `count`, or `sum_amount` (deals only).
    #[serde(default = "count")]
    pub measure: String,
    /// Field to the values to keep: alternatives within a field, AND across fields.
    #[serde(default)]
    pub filter: BTreeMap<String, Vec<String>>,
    /// Only records created (tickets: opened) on or after this day.
    #[serde(default)]
    pub from: Option<NaiveDate>,
    /// Only records created (tickets: opened) on or before this day.
    #[serde(default)]
    pub to: Option<NaiveDate>,
}

fn count() -> String {
    "count".to_string()
}

/// A value to bind, in the order the placeholders appear.
#[derive(Debug, Clone, PartialEq)]
pub enum Bind {
    /// A list of texts, bound as JSON and unpacked in SQL.
    List(Vec<String>),
    /// A day.
    Day(NaiveDate),
}

/// The query for a report.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    /// SQL with `$1`, `$2`… placeholders; it selects `grp`, optionally `currency`, and `value`.
    pub sql: String,
    /// What to bind, in order.
    pub binds: Vec<Bind>,
    /// Whether rows carry a currency (a sum of amounts is per currency, never merged).
    pub per_currency: bool,
    /// Whether the report looks at tags, which a masked reader may not.
    pub uses_tags: bool,
    /// Whether the measure is money, which a masked reader may not.
    pub uses_money: bool,
}

/// How one kind of record is read.
struct Table {
    /// `FROM` clause with its alias.
    from: &'static str,
    /// The alias's primary key column.
    pid: &'static str,
    /// A condition that keeps only live records.
    live: &'static str,
    /// The column a date range applies to.
    created: &'static str,
    /// Field name to the SQL expression for it.
    fields: &'static [(&'static str, &'static str)],
    /// Whether records can carry tags.
    taggable: bool,
    /// `(amount column, currency column)`, for a kind with money.
    money: Option<(&'static str, &'static str)>,
}

#[allow(clippy::too_many_lines)] // one table of what each kind offers
fn table(kind: &str) -> Option<Table> {
    Some(match kind {
        "contact" => Table {
            from: "contacts c",
            pid: "c.pid",
            live: "c.deleted_at IS NULL AND c.merged_into_pid IS NULL",
            created: "c.created_at",
            fields: &[
                ("status", "c.status"),
                ("marketing_consent", "c.marketing_consent"),
                (
                    "account_tier",
                    "coalesce((SELECT a.tier FROM accounts a WHERE a.pid = c.account_pid), '(no account)')",
                ),
                ("owner", "coalesce(c.owner_ref, '(none)')"),
                (
                    "created_month",
                    "to_char(date_trunc('month', c.created_at), 'YYYY-MM')",
                ),
            ],
            taggable: true,
            money: None,
        },
        "account" => Table {
            from: "accounts a",
            pid: "a.pid",
            live: "a.deleted_at IS NULL",
            created: "a.created_at",
            fields: &[
                ("tier", "a.tier"),
                ("industry", "coalesce(a.industry, '(none)')"),
                ("owner", "coalesce(a.owner_ref, '(none)')"),
                (
                    "created_month",
                    "to_char(date_trunc('month', a.created_at), 'YYYY-MM')",
                ),
            ],
            taggable: true,
            money: None,
        },
        "lead" => Table {
            from: "leads l",
            pid: "l.pid",
            live: "l.deleted_at IS NULL",
            created: "l.created_at",
            fields: &[
                ("source", "l.source"),
                ("status", "l.status"),
                ("owner", "coalesce(l.owner_ref, '(none)')"),
                (
                    "created_month",
                    "to_char(date_trunc('month', l.created_at), 'YYYY-MM')",
                ),
            ],
            taggable: true,
            money: None,
        },
        "deal" => Table {
            from: "deals d",
            pid: "d.pid",
            live: "d.deleted_at IS NULL",
            created: "d.created_at",
            fields: &[
                (
                    "stage",
                    "coalesce((SELECT s.name FROM pipeline_stages s WHERE s.pid = d.stage_pid), '(unknown)')",
                ),
                (
                    "state",
                    "CASE WHEN d.closed_at IS NULL THEN 'open' WHEN d.won THEN 'won' ELSE 'lost' END",
                ),
                ("currency", "d.currency"),
                (
                    "pipeline",
                    "coalesce((SELECT p.name FROM pipelines p WHERE p.pid = d.pipeline_pid), '(unknown)')",
                ),
                ("owner", "coalesce(d.owner_ref, '(none)')"),
                (
                    "created_month",
                    "to_char(date_trunc('month', d.created_at), 'YYYY-MM')",
                ),
                (
                    "closed_month",
                    "coalesce(to_char(date_trunc('month', d.closed_at), 'YYYY-MM'), '(open)')",
                ),
            ],
            taggable: true,
            money: Some(("d.amount_minor", "d.currency")),
        },
        "ticket" => Table {
            from: "tickets k",
            pid: "k.pid",
            live: "k.deleted_at IS NULL",
            created: "k.opened_at",
            fields: &[
                ("status", "k.status"),
                ("priority", "k.priority"),
                ("channel", "k.channel"),
                ("assignee", "coalesce(k.assignee_ref, '(none)')"),
                (
                    "opened_month",
                    "to_char(date_trunc('month', k.opened_at), 'YYYY-MM')",
                ),
            ],
            taggable: false,
            money: None,
        },
        _ => return None,
    })
}

/// The fields a report on `kind` can group or filter by (and `tag`, where records carry tags).
#[must_use]
pub fn fields(kind: &str) -> Vec<&'static str> {
    let Some(t) = table(kind) else {
        return Vec::new();
    };
    let mut out: Vec<&'static str> = t.fields.iter().map(|(name, _)| *name).collect();
    if t.taggable {
        out.push("tag");
    }
    out
}

/// Whether `value` can be a value of the filterable `field` of `kind`; `None` when the field takes
/// free text (an owner, an industry, a stage name) and any short text will do.
fn token_set(kind: &str, field: &str) -> Option<&'static [&'static str]> {
    match (kind, field) {
        ("contact" | "lead" | "ticket", "status") => Some(match kind {
            "contact" => tokens::CONTACT_STATUSES,
            "lead" => tokens::LEAD_STATUSES,
            _ => tokens::TICKET_STATUSES,
        }),
        ("contact", "marketing_consent") => Some(tokens::CONSENT_STATES),
        ("account", "tier") => Some(tokens::ACCOUNT_TIERS),
        ("lead", "source") => Some(tokens::LEAD_SOURCES),
        ("deal", "state") => Some(&["open", "won", "lost"]),
        ("ticket", "priority") => Some(tokens::PRIORITIES),
        ("ticket", "channel") => Some(tokens::TICKET_CHANNELS),
        _ => None,
    }
}

fn is_month(field: &str) -> bool {
    field.ends_with("_month")
}

/// Everything wrong with a specification, as `(member, message)`.
fn check(spec: &Spec, table: &Table) -> Vec<(String, String)> {
    let mut problems: Vec<(String, String)> = Vec::new();
    let mut add = |member: &str, message: String| problems.push((member.to_string(), message));
    let allowed = fields(&spec.kind);
    if !allowed.contains(&spec.group_by.as_str()) {
        add(
            "group_by",
            format!(
                "a {} report groups by one of {}",
                spec.kind,
                allowed.join(", ")
            ),
        );
    }
    if !MEASURES.contains(&spec.measure.as_str()) {
        add("measure", format!("must be one of {}", MEASURES.join(", ")));
    }
    if spec.measure == "sum_amount" && table.money.is_none() {
        add("measure", "only a deal report can sum amounts".to_string());
    }
    for (field, values) in &spec.filter {
        if is_month(field) || !allowed.contains(&field.as_str()) {
            add(
                "filter",
                format!("a {} report cannot filter by {field}", spec.kind),
            );
            continue;
        }
        if values.is_empty() || values.len() > MAX_VALUES {
            add("filter", format!("{field} needs 1 to {MAX_VALUES} values"));
            continue;
        }
        if let Some(set) = token_set(&spec.kind, field)
            && let Some(bad) = values.iter().find(|v| !tokens::is_token(set, v))
        {
            add("filter", format!("{bad:?} is not a value of {field}"));
        } else if values
            .iter()
            .any(|v| v.trim().is_empty() || v.chars().count() > 100)
        {
            add("filter", format!("a value of {field} is empty or too long"));
        }
    }
    if let (Some(from), Some(to)) = (spec.from, spec.to)
        && from > to
    {
        add("from", "is after the end of the range".to_string());
    }
    problems
}

/// Check a specification and build its query, or say everything that is wrong, as
/// `(member, message)`.
///
/// # Errors
///
/// Every problem found.
pub fn plan(spec: &Spec) -> Result<Plan, Vec<(String, String)>> {
    let Some(table) = table(&spec.kind) else {
        return Err(vec![(
            "kind".to_string(),
            format!("must be one of {}", KINDS.join(", ")),
        )]);
    };
    let problems = check(spec, &table);
    if !problems.is_empty() {
        return Err(problems);
    }

    let mut binds = Vec::new();
    let mut conditions = vec![table.live.to_string()];
    for (field, values) in &spec.filter {
        binds.push(Bind::List(values.clone()));
        let n = binds.len();
        if field == "tag" {
            conditions.push(format!(
                "EXISTS (SELECT 1 FROM taggings g2 JOIN tags t2 ON t2.pid = g2.tag_pid \
                 WHERE g2.subject_pid = {} AND g2.subject_kind = '{}' AND lower(t2.name) IN \
                 (SELECT lower(jsonb_array_elements_text(${n}::jsonb))))",
                table.pid, spec.kind
            ));
        } else if let Some((_, expr)) = table.fields.iter().find(|(name, _)| name == field) {
            conditions.push(format!(
                "{expr} IN (SELECT jsonb_array_elements_text(${n}::jsonb))"
            ));
        }
    }
    if let Some(from) = spec.from {
        binds.push(Bind::Day(from));
        conditions.push(format!("{}::date >= ${}", table.created, binds.len()));
    }
    if let Some(to) = spec.to {
        binds.push(Bind::Day(to));
        conditions.push(format!("{}::date <= ${}", table.created, binds.len()));
    }

    let by_tag = spec.group_by == "tag";
    let (join, group) = if by_tag {
        (
            format!(
                " LEFT JOIN taggings g ON g.subject_pid = {} AND g.subject_kind = '{}' \
                 LEFT JOIN tags t ON t.pid = g.tag_pid",
                table.pid, spec.kind
            ),
            "coalesce(t.name, '(no tag)')".to_string(),
        )
    } else {
        let expr = table
            .fields
            .iter()
            .find(|(name, _)| *name == spec.group_by)
            .map_or("''", |(_, expr)| *expr);
        (String::new(), expr.to_string())
    };
    let sum = spec.measure == "sum_amount";
    let (select, group_by, order) = match (sum, table.money) {
        (true, Some((amount, currency))) => (
            format!(
                "{group} AS grp, {currency} AS currency, coalesce(sum({amount}), 0)::bigint AS value"
            ),
            "grp, currency".to_string(),
            "value DESC, grp, currency".to_string(),
        ),
        _ => (
            format!("{group} AS grp, count(*)::bigint AS value"),
            "grp".to_string(),
            "value DESC, grp".to_string(),
        ),
    };
    let sql = format!(
        "SELECT {select} FROM {from}{join} WHERE {where_} GROUP BY {group_by} ORDER BY {order} LIMIT {limit}",
        from = table.from,
        where_ = conditions.join(" AND "),
        limit = MAX_GROUPS + 1,
    );
    Ok(Plan {
        sql,
        binds,
        per_currency: sum,
        uses_tags: by_tag || spec.filter.contains_key("tag"),
        uses_money: sum,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(kind: &str, group_by: &str) -> Spec {
        Spec {
            kind: kind.to_string(),
            group_by: group_by.to_string(),
            measure: "count".to_string(),
            filter: BTreeMap::new(),
            from: None,
            to: None,
        }
    }

    #[test]
    fn a_count_by_a_field_is_one_grouped_query() {
        let p = plan(&spec("lead", "source")).unwrap();
        assert!(
            p.sql.starts_with(
                "SELECT l.source AS grp, count(*)::bigint AS value FROM leads l WHERE"
            )
        );
        assert!(
            p.sql
                .contains("GROUP BY grp ORDER BY value DESC, grp LIMIT 201")
        );
        assert!(p.binds.is_empty());
        assert!(!p.per_currency && !p.uses_tags && !p.uses_money);
    }

    #[test]
    fn a_sum_of_deals_is_always_per_currency() {
        let mut s = spec("deal", "state");
        s.measure = "sum_amount".into();
        let p = plan(&s).unwrap();
        assert!(p.sql.contains("d.currency AS currency"));
        assert!(p.sql.contains("GROUP BY grp, currency"));
        assert!(p.per_currency && p.uses_money);
    }

    #[test]
    fn filters_and_dates_are_bound_never_written_in() {
        let mut s = spec("contact", "status");
        s.filter
            .insert("marketing_consent".into(), vec!["granted".into()]);
        s.filter.insert(
            "tag".into(),
            vec!["VIP".into(), "x'; DROP TABLE contacts; --".into()],
        );
        s.from = NaiveDate::from_ymd_opt(2026, 1, 1);
        s.to = NaiveDate::from_ymd_opt(2026, 12, 31);
        let p = plan(&s).unwrap();
        assert!(!p.sql.contains("DROP"));
        assert!(!p.sql.contains("VIP"));
        assert_eq!(p.binds.len(), 4);
        assert!(p.sql.contains("$1::jsonb") && p.sql.contains("$2::jsonb"));
        assert!(
            p.sql.contains("c.created_at::date >= $3")
                && p.sql.contains("c.created_at::date <= $4")
        );
        assert!(p.uses_tags);
    }

    #[test]
    fn grouping_by_tag_joins_the_taggings() {
        let p = plan(&spec("contact", "tag")).unwrap();
        assert!(
            p.sql
                .contains("LEFT JOIN taggings g ON g.subject_pid = c.pid")
        );
        assert!(p.sql.contains("coalesce(t.name, '(no tag)') AS grp"));
        assert!(p.uses_tags);
    }

    #[test]
    fn what_cannot_be_asked_is_refused_with_the_member() {
        let members = |s: &Spec| -> Vec<String> {
            plan(s).unwrap_err().into_iter().map(|(m, _)| m).collect()
        };
        assert_eq!(members(&spec("nonsense", "x")), ["kind"]);
        assert_eq!(members(&spec("lead", "score")), ["group_by"]);
        assert_eq!(
            members(&spec("ticket", "tag")),
            ["group_by"],
            "tickets have no tags"
        );
        let mut sum = spec("lead", "source");
        sum.measure = "sum_amount".into();
        assert_eq!(members(&sum), ["measure"]);
        let mut bad_measure = spec("lead", "source");
        bad_measure.measure = "avg".into();
        assert_eq!(members(&bad_measure), ["measure"]);
        let mut month = spec("lead", "source");
        month
            .filter
            .insert("created_month".into(), vec!["2026-01".into()]);
        assert_eq!(members(&month), ["filter"]);
        let mut value = spec("lead", "source");
        value
            .filter
            .insert("source".into(), vec!["carrier pigeon".into()]);
        assert_eq!(members(&value), ["filter"]);
        let mut empty = spec("lead", "source");
        empty.filter.insert("status".into(), vec![]);
        assert_eq!(members(&empty), ["filter"]);
        let mut range = spec("lead", "source");
        range.from = NaiveDate::from_ymd_opt(2026, 2, 1);
        range.to = NaiveDate::from_ymd_opt(2026, 1, 1);
        assert_eq!(members(&range), ["from"]);
    }

    #[test]
    fn every_kinds_fields_build() {
        for kind in KINDS {
            for field in fields(kind) {
                assert!(plan(&spec(kind, field)).is_ok(), "{kind} by {field}");
            }
            // A kind with no fields would pass the loop above by doing nothing.
            assert!(!fields(kind).is_empty(), "{kind} has fields");
        }
        assert!(fields("deal").contains(&"tag") && !fields("ticket").contains(&"tag"));
    }

    #[test]
    fn the_measure_defaults_to_a_count() {
        let parsed: Spec = serde_json::from_str(r#"{"kind":"lead","group_by":"source"}"#).unwrap();
        assert_eq!(parsed.measure, "count");
        assert!(
            serde_json::from_str::<Spec>(r#"{"kind":"lead","group_by":"source","x":1}"#).is_err()
        );
    }

    #[test]
    fn each_fixed_filter_takes_its_own_values_and_no_others() {
        let cases = [
            ("contact", "status", "active", "gone"),
            ("contact", "marketing_consent", "granted", "maybe"),
            ("account", "tier", "customer", "vip"),
            ("lead", "status", "new", "lost"),
            ("lead", "source", "web", "pigeon"),
            ("deal", "state", "won", "drawn"),
            ("ticket", "status", "open", "stuck"),
            ("ticket", "priority", "urgent", "whenever"),
            ("ticket", "channel", "chat", "pigeon"),
        ];
        for (kind, field, good, bad) in cases {
            let mut s = spec(kind, field);
            s.filter.insert(field.into(), vec![good.into()]);
            assert!(plan(&s).is_ok(), "{kind}.{field} takes {good}");
            s.filter.insert(field.into(), vec![bad.into()]);
            assert!(plan(&s).is_err(), "{kind}.{field} refuses {bad}");
        }
    }

    #[test]
    fn every_filter_field_uses_its_own_expression() {
        let mut s = spec("contact", "owner");
        s.filter.insert("status".into(), vec!["active".into()]);
        s.filter
            .insert("marketing_consent".into(), vec!["granted".into()]);
        let sql = plan(&s).unwrap().sql;
        // Filters go in key order, so the first placeholder belongs to `marketing_consent`.
        assert!(
            sql.contains("c.marketing_consent IN (SELECT jsonb_array_elements_text($1::jsonb))"),
            "{sql}"
        );
        assert!(
            sql.contains("c.status IN (SELECT jsonb_array_elements_text($2::jsonb))"),
            "{sql}"
        );
    }

    #[test]
    fn the_limits_on_values_and_dates_are_inclusive() {
        let values = |n: usize| -> Vec<String> { (0..n).map(|i| format!("o{i}")).collect() };
        let mut s = spec("lead", "source");
        s.filter.insert("owner".into(), values(MAX_VALUES));
        assert!(plan(&s).is_ok(), "twenty values are allowed");
        s.filter.insert("owner".into(), values(MAX_VALUES + 1));
        assert!(plan(&s).is_err(), "twenty-one are not");
        s.filter.insert("owner".into(), vec!["x".repeat(100)]);
        assert!(plan(&s).is_ok(), "a hundred characters are allowed");
        s.filter.insert("owner".into(), vec!["x".repeat(101)]);
        assert!(plan(&s).is_err(), "a hundred and one are not");
        s.filter.insert("owner".into(), vec!["  ".into()]);
        assert!(plan(&s).is_err(), "a blank value is not");
        s.filter.clear();
        let day = NaiveDate::from_ymd_opt(2026, 6, 1);
        s.from = day;
        s.to = day;
        assert!(plan(&s).is_ok(), "one day is a range");
    }
}
