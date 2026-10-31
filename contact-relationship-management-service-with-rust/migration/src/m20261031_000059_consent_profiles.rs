//! Migration: consent and e-marketing rules as jurisdiction profiles (CRM-T166, CRM-D21).
//!
//! - `consent_jurisdictions`: the codes the organization keeps, each with a layer and a parent (WPM-D144's shape).
//! - `consent_profile_rows`: dated, sourced parameter rows with a status; append-only (a trigger refuses an update
//!   or a delete), so a change is a new row. `id` is the recording order the resolver ties on.
//! - `consent_profile_acknowledgements`: an administrator's acknowledgement of a code's `example` and `unverified`
//!   rows up to `through_id` (decision 9); append-only.
//! - `consent_purposes`: the purposes, until now fixed in code; the four of today are its first rows, at the
//!   positions a signed link already names them by.
//! - Consent per channel (decision 7): `contact_consents` and `consent_events` gain `channel`; every existing row
//!   becomes `email`'s, the only channel ever sent. `contacts.marketing_consent` keeps meaning `marketing` on
//!   `email`. No row means `never`, so `sms`, `phone` and `post` start `never`.
//! - `consent_jurisdiction` on contacts, campaigns and nurture sequences (decision 1), empty for every existing row,
//!   so every existing answer stays today's.
//! - The organization's sender name, postal address and contact link (decision 8), unset.
//! - `email_deliveries.basis`: the basis a message was sent on, with the code and profile status in force.

use sea_orm_migration::prelude::*;

/// The consent-profiles migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

/// The statements of `up`, in order.
pub(crate) const UP: [&str; 16] = [
    "CREATE TABLE IF NOT EXISTS consent_jurisdictions (
         code VARCHAR(8) PRIMARY KEY,
         layer VARCHAR(16) NOT NULL CHECK (layer IN ('supranational', 'national', 'subnational')),
         parent VARCHAR(8) NULL,
         name VARCHAR(200) NULL,
         updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
         set_by VARCHAR NULL
     )",
    "CREATE TABLE IF NOT EXISTS consent_profile_rows (
         id BIGSERIAL PRIMARY KEY,
         code VARCHAR(8) NOT NULL,
         key VARCHAR(64) NOT NULL,
         value JSONB NOT NULL,
         effective_from DATE NOT NULL,
         effective_to DATE NULL,
         source TEXT NOT NULL,
         checked_on DATE NULL,
         status VARCHAR(16) NOT NULL CHECK (status IN ('read', 'unverified', 'example')),
         recorded_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
         recorded_by VARCHAR NULL,
         CHECK (effective_to IS NULL OR effective_to >= effective_from)
     )",
    "CREATE INDEX IF NOT EXISTS consent_profile_rows_code ON consent_profile_rows (code, key)",
    "CREATE TABLE IF NOT EXISTS consent_profile_acknowledgements (
         id BIGSERIAL PRIMARY KEY,
         code VARCHAR(8) NOT NULL,
         through_id BIGINT NOT NULL,
         acknowledged_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
         acknowledged_by VARCHAR NULL
     )",
    "CREATE OR REPLACE FUNCTION crm_consent_profile_append_only() RETURNS trigger AS $$
     BEGIN
       RAISE EXCEPTION '% is append-only', TG_TABLE_NAME;
     END; $$ LANGUAGE plpgsql",
    "DROP TRIGGER IF EXISTS crm_append_only ON consent_profile_rows",
    "CREATE TRIGGER crm_append_only BEFORE UPDATE OR DELETE ON consent_profile_rows
     FOR EACH ROW EXECUTE FUNCTION crm_consent_profile_append_only()",
    "DROP TRIGGER IF EXISTS crm_append_only ON consent_profile_acknowledgements",
    "CREATE TRIGGER crm_append_only BEFORE UPDATE OR DELETE ON consent_profile_acknowledgements
     FOR EACH ROW EXECUTE FUNCTION crm_consent_profile_append_only()",
    "CREATE TABLE IF NOT EXISTS consent_purposes (
         key VARCHAR(64) PRIMARY KEY,
         position INTEGER NOT NULL UNIQUE,
         names JSONB NOT NULL DEFAULT '{}'::jsonb,
         descriptions JSONB NOT NULL DEFAULT '{}'::jsonb,
         active BOOLEAN NOT NULL DEFAULT TRUE,
         soft_opt_in_eligible BOOLEAN NOT NULL DEFAULT FALSE,
         updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
         set_by VARCHAR NULL
     )",
    "INSERT INTO consent_purposes (key, position, names) VALUES
         ('marketing', 0, '{\"en-001\": \"Marketing\"}'),
         ('newsletter', 1, '{\"en-001\": \"Newsletter\"}'),
         ('events', 2, '{\"en-001\": \"Events\"}'),
         ('product_updates', 3, '{\"en-001\": \"Product updates\"}')
     ON CONFLICT (key) DO NOTHING",
    "ALTER TABLE contact_consents ADD COLUMN IF NOT EXISTS channel VARCHAR(8) NOT NULL DEFAULT 'email'",
    "ALTER TABLE contact_consents DROP CONSTRAINT IF EXISTS contact_consents_pkey, \
     ADD PRIMARY KEY (contact_pid, purpose, channel)",
    "ALTER TABLE consent_events ADD COLUMN IF NOT EXISTS channel VARCHAR(8) NOT NULL DEFAULT 'email'",
    "ALTER TABLE organization_settings \
     ADD COLUMN IF NOT EXISTS sender_name VARCHAR(200) NULL, \
     ADD COLUMN IF NOT EXISTS postal_address TEXT NULL, \
     ADD COLUMN IF NOT EXISTS contact_link VARCHAR(500) NULL",
    "ALTER TABLE email_deliveries ADD COLUMN IF NOT EXISTS basis JSONB NULL",
];

/// The tables gaining `consent_jurisdiction`.
const CODED: [&str; 3] = ["contacts", "campaigns", "nurture_sequences"];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the tables and add the columns.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        for statement in UP {
            conn.execute_unprepared(statement).await?;
        }
        for table in CODED {
            conn.execute_unprepared(&format!(
                "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS consent_jurisdiction VARCHAR(8) NULL"
            ))
            .await?;
        }
        Ok(())
    }

    /// Drop them (rollback). Consent held on channels other than email, and every profile, is lost.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        for table in CODED {
            conn.execute_unprepared(&format!(
                "ALTER TABLE {table} DROP COLUMN IF EXISTS consent_jurisdiction"
            ))
            .await?;
        }
        for statement in [
            "ALTER TABLE email_deliveries DROP COLUMN IF EXISTS basis",
            "ALTER TABLE organization_settings DROP COLUMN IF EXISTS sender_name, \
             DROP COLUMN IF EXISTS postal_address, DROP COLUMN IF EXISTS contact_link",
            "ALTER TABLE consent_events DROP COLUMN IF EXISTS channel",
            "DELETE FROM contact_consents WHERE channel <> 'email'",
            "ALTER TABLE contact_consents DROP CONSTRAINT IF EXISTS contact_consents_pkey, \
             ADD PRIMARY KEY (contact_pid, purpose)",
            "ALTER TABLE contact_consents DROP COLUMN IF EXISTS channel",
            "DROP TABLE IF EXISTS consent_purposes",
            "DROP TABLE IF EXISTS consent_profile_acknowledgements",
            "DROP TABLE IF EXISTS consent_profile_rows",
            "DROP FUNCTION IF EXISTS crm_consent_profile_append_only()",
            "DROP TABLE IF EXISTS consent_jurisdictions",
        ] {
            conn.execute_unprepared(statement).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::UP;

    #[test]
    fn existing_consent_becomes_emails_and_the_purposes_keep_their_link_positions() {
        let channel = |table: &str| {
            UP.iter().any(|s| {
                s.starts_with(&format!(
                    "ALTER TABLE {table} ADD COLUMN IF NOT EXISTS channel"
                )) && s.contains("DEFAULT 'email'")
            })
        };
        assert!(channel("contact_consents") && channel("consent_events"));
        let seed = UP
            .iter()
            .find(|s| s.starts_with("INSERT INTO consent_purposes"))
            .unwrap();
        for (position, key) in ["marketing", "newsletter", "events", "product_updates"]
            .iter()
            .enumerate()
        {
            assert!(seed.contains(&format!("('{key}', {position},")), "{key}");
        }
        // The profile tables refuse an update or a delete.
        for table in ["consent_profile_rows", "consent_profile_acknowledgements"] {
            assert!(UP.iter().any(
                |s| s.contains("BEFORE UPDATE OR DELETE") && s.contains(&format!("ON {table}"))
            ));
        }
    }
}
