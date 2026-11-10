//! The row of `contacts`. One relationship wrapper over a `person:` record (CRM-R1); `marketing_consent` is send-path law (CRM-D6).

#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "Contact")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub pid: Uuid,
    pub person_ref: String,
    pub account_pid: Option<Uuid>,
    pub owner_ref: Option<String>,
    pub display_name: String,
    /// Optional; never derived from `display_name` (CRM-R37).
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub status: String,
    pub job_title: Option<String>,
    pub preferred_channel: crate::models::encrypted::EncryptedText,
    pub marketing_consent: String,
    pub consent_changed_at: Option<DateTimeWithTimeZone>,
    pub stakeholder_role: Option<String>,
    pub influence: Option<i32>,
    pub interest: Option<i32>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
    pub merged_into_pid: Option<Uuid>,
    /// The jurisdiction code whose consent profile applies to this contact (CRM-D21); set by staff, an import
    /// column or a web form, never inferred.
    pub consent_jurisdiction: Option<String>,
}
