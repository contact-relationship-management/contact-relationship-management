//! Migration: what a stalled deal is derived from (CRM-T183, CRM-D20).
//!
//! - `deals.stage_moved_at`: the deal's last stage move or reopen (its creation until the first).
//!   Existing deals take the newest `deal_stage_changed`, `deal_won`, `deal_lost` or
//!   `deal_reopened` audit row, else `created_at`. `updated_at` is not moved: nobody edited them.
//! - `organization_settings.stalled_after_days`: after how many days without a stage move or an
//!   activity an open deal is stalled, 1 to 365, starting at 14.

use sea_orm_migration::prelude::*;

/// The deal-stalled migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

/// The statements of `up`, in order.
pub(crate) const UP: [&str; 6] = [
    "ALTER TABLE organization_settings ADD COLUMN IF NOT EXISTS stalled_after_days SMALLINT \
     NOT NULL DEFAULT 14 CHECK (stalled_after_days BETWEEN 1 AND 365)",
    "ALTER TABLE deals ADD COLUMN IF NOT EXISTS stage_moved_at TIMESTAMPTZ NULL",
    "ALTER TABLE deals DISABLE TRIGGER crm_touch_updated_at",
    "UPDATE deals SET stage_moved_at = COALESCE(\
         (SELECT max(a.created_at) FROM audit_logs a \
          WHERE a.entity = 'deal' AND a.entity_pid = deals.pid \
            AND a.action IN ('deal_stage_changed', 'deal_won', 'deal_lost', 'deal_reopened')), \
         deals.created_at) \
     WHERE stage_moved_at IS NULL",
    "ALTER TABLE deals ENABLE TRIGGER crm_touch_updated_at",
    "ALTER TABLE deals ALTER COLUMN stage_moved_at SET DEFAULT CURRENT_TIMESTAMP, \
     ALTER COLUMN stage_moved_at SET NOT NULL",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the two columns and fill the deals' last stage move.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        for statement in UP {
            conn.execute_unprepared(statement).await?;
        }
        Ok(())
    }

    /// Drop them.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared("ALTER TABLE deals DROP COLUMN IF EXISTS stage_moved_at")
            .await?;
        conn.execute_unprepared(
            "ALTER TABLE organization_settings DROP COLUMN IF EXISTS stalled_after_days",
        )
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::UP;

    #[test]
    fn the_fill_does_not_touch_updated_at_and_the_trigger_comes_back() {
        let off = UP
            .iter()
            .position(|s| s.contains("DISABLE TRIGGER"))
            .unwrap();
        let fill = UP
            .iter()
            .position(|s| s.starts_with("UPDATE deals"))
            .unwrap();
        let on = UP
            .iter()
            .position(|s| s.contains("ENABLE TRIGGER"))
            .unwrap();
        let required = UP.iter().position(|s| s.contains("SET NOT NULL")).unwrap();
        assert!(off < fill && fill < on && on < required);
        for action in [
            "deal_stage_changed",
            "deal_won",
            "deal_lost",
            "deal_reopened",
        ] {
            assert!(UP[fill].contains(action), "{action}");
        }
    }
}
