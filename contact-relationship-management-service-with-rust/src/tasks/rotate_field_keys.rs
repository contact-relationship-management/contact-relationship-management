//! `cargo loco task rotate_field_keys` — re-seal every encrypted column under
//! the active key (CRM-T41). Run it after adding a new first key to
//! `CRM_FIELD_KEYS`, and before retiring an old one; it also seals any legacy
//! plaintext. It is idempotent: values already under the active key are left
//! alone, and it reports the counts.

use loco_rs::prelude::*;
use loco_rs::task::{TaskInfo, Vars};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

use crate::crypto;

/// The encrypted columns: `(table, column)`.
pub const ENCRYPTED_COLUMNS: [(&str, &str); 2] =
    [("contacts", "preferred_channel"), ("activities", "summary")];

/// Re-seal every encrypted column; returns the rows changed per column.
///
/// # Errors
///
/// When no keyring is configured, a stored value cannot be opened (for
/// example its key was already retired), or a statement fails. Nothing is
/// written for a column after the first failure in it.
pub async fn rotate(db: &DatabaseConnection) -> std::result::Result<Vec<(String, u64)>, String> {
    let ring = crypto::current().ok_or("no field keys are configured (CRM_FIELD_KEYS)")?;
    let mut report = Vec::new();
    for (table, column) in ENCRYPTED_COLUMNS {
        let rows = db
            .query_all_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                format!("SELECT id, {column} AS value FROM {table} ORDER BY id"),
            ))
            .await
            .map_err(|e| e.to_string())?;
        let mut updates = Vec::new();
        for row in &rows {
            let id: i32 = row.try_get("", "id").map_err(|e| e.to_string())?;
            let stored: String = row.try_get("", "value").map_err(|e| e.to_string())?;
            if ring.needs_rotation(&stored) {
                let plain = ring
                    .open(&stored)
                    .map_err(|e| format!("{table}.{column} row {id}: {e}"))?;
                updates.push((id, ring.seal(&plain)));
            }
        }
        for (id, sealed) in &updates {
            db.execute_raw(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                format!("UPDATE {table} SET {column} = $1 WHERE id = $2"),
                [sealed.clone().into(), (*id).into()],
            ))
            .await
            .map_err(|e| e.to_string())?;
        }
        report.push((format!("{table}.{column}"), updates.len() as u64));
    }
    report.push((
        "custom_field_values.value (sensitive fields)".to_string(),
        rotate_custom(db, &ring).await?,
    ));
    Ok(report)
}

/// Re-seal the values of `sensitive` custom fields (CRM-R41). The other fields' values are kept
/// as plain text, so they are left alone.
async fn rotate_custom(
    db: &DatabaseConnection,
    ring: &crypto::Keyring,
) -> std::result::Result<u64, String> {
    let rows = db
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT v.id, v.value FROM custom_field_values v JOIN custom_field_defs d \
             ON d.pid = v.field_pid WHERE d.sensitivity = 'sensitive' ORDER BY v.id"
                .to_string(),
        ))
        .await
        .map_err(|e| e.to_string())?;
    let mut updates = Vec::new();
    for row in &rows {
        let id: i64 = row.try_get("", "id").map_err(|e| e.to_string())?;
        let stored: String = row.try_get("", "value").map_err(|e| e.to_string())?;
        if ring.needs_rotation(&stored) {
            let plain = ring
                .open(&stored)
                .map_err(|e| format!("custom_field_values row {id}: {e}"))?;
            updates.push((id, ring.seal(&plain)));
        }
    }
    for (id, sealed) in &updates {
        db.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE custom_field_values SET value = $1 WHERE id = $2",
            [sealed.clone().into(), (*id).into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(updates.len() as u64)
}

/// The loco task wrapper.
pub struct RotateFieldKeys;

#[async_trait]
impl Task for RotateFieldKeys {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "rotate_field_keys".to_string(),
            detail: "Re-seal encrypted columns under the active CRM_FIELD_KEYS key".to_string(),
        }
    }

    async fn run(&self, ctx: &AppContext, _vars: &Vars) -> Result<()> {
        let report = rotate(&ctx.db).await.map_err(|e| Error::string(&e))?;
        for (column, count) in report {
            tracing::info!(%column, count, "re-sealed");
            println!("{column}: {count} row(s) re-sealed");
        }
        Ok(())
    }
}
