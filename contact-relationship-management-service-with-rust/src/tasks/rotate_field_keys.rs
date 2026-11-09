//! `cargo run -- task rotate_field_keys` — re-seal every encrypted column under
//! the active key (CRM-T41). Run it after adding a new first key to
//! `CRM_FIELD_KEYS`, and before retiring an old one; it also seals any legacy
//! plaintext. It is idempotent: values already under the active key are left
//! alone, and it reports the counts.

use axa::prelude::*;
use axa::task::{TaskInfo, Vars};
use sqlx::{AssertSqlSafe, PgPool};

use crate::crypto;

/// The encrypted columns: `(table, column)`.
pub const ENCRYPTED_COLUMNS: [(&str, &str); 2] =
    [("contacts", "preferred_channel"), ("activities", "summary")];

/// Re-seal every encrypted column; returns the rows changed per column.
///
/// The two sealed columns are named from [`ENCRYPTED_COLUMNS`] at run time, so their statements
/// are not checked by the `sqlx` macros (CRM-D24); they read the stored text as it is, every row,
/// deleted or not, rather than through `EncryptedText`.
///
/// # Errors
///
/// When no keyring is configured, a stored value cannot be opened (for
/// example its key was already retired), or a statement fails. Nothing is
/// written for a column after the first failure in it.
pub async fn rotate(pool: &PgPool) -> std::result::Result<Vec<(String, u64)>, String> {
    let ring = crypto::current().ok_or("no field keys are configured (CRM_FIELD_KEYS)")?;
    let mut report = Vec::new();
    for (table, column) in ENCRYPTED_COLUMNS {
        // Both names come from the constant above, never from a caller.
        let select = format!("SELECT id, {column} AS value FROM {table} ORDER BY id");
        let rows: Vec<(i32, String)> = sqlx::query_as(AssertSqlSafe(select))
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
        let mut updates = Vec::new();
        for (id, stored) in &rows {
            if ring.needs_rotation(stored) {
                let plain = ring
                    .open(stored)
                    .map_err(|e| format!("{table}.{column} row {id}: {e}"))?;
                updates.push((*id, ring.seal(&plain)));
            }
        }
        let update = format!("UPDATE {table} SET {column} = $1 WHERE id = $2");
        for (id, sealed) in &updates {
            sqlx::query(AssertSqlSafe(update.as_str()))
                .bind(sealed)
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| e.to_string())?;
        }
        report.push((format!("{table}.{column}"), updates.len() as u64));
    }
    report.push((
        "custom_field_values.value (sensitive fields)".to_string(),
        rotate_custom(pool, &ring).await?,
    ));
    Ok(report)
}

/// Re-seal the values of `sensitive` custom fields (CRM-R41). The other fields' values are kept
/// as plain text, so they are left alone.
async fn rotate_custom(pool: &PgPool, ring: &crypto::Keyring) -> std::result::Result<u64, String> {
    let rows = sqlx::query!(
        "SELECT v.id, v.value FROM custom_field_values v JOIN custom_field_defs d \
         ON d.pid = v.field_pid WHERE d.sensitivity = 'sensitive' ORDER BY v.id"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    let mut updates = Vec::new();
    for row in &rows {
        if ring.needs_rotation(&row.value) {
            let plain = ring
                .open(&row.value)
                .map_err(|e| format!("custom_field_values row {}: {e}", row.id))?;
            updates.push((row.id, ring.seal(&plain)));
        }
    }
    for (id, sealed) in &updates {
        sqlx::query!(
            "UPDATE custom_field_values SET value = $1 WHERE id = $2",
            sealed,
            id
        )
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(updates.len() as u64)
}

/// The task wrapper.
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
        let report = rotate(crate::db::pool(ctx))
            .await
            .map_err(|e| Error::string(&e))?;
        for (column, count) in report {
            tracing::info!(%column, count, "re-sealed");
            println!("{column}: {count} row(s) re-sealed");
        }
        Ok(())
    }
}
