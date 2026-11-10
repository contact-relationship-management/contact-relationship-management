//! The row of `leads`. One unqualified prospect with its derived score (CRM-R3).

#![allow(missing_docs)]

use crate::models::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[schemars(rename = "Lead")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub id: i32,
    pub pid: Uuid,
    pub source: String,
    pub campaign_pid: Option<Uuid>,
    pub contact_pid: Option<Uuid>,
    pub display_name: String,
    pub email: Option<String>,
    pub email_domain: Option<String>,
    pub score: i32,
    pub campaign_click: bool,
    pub unsubscribed: bool,
    pub status: String,
    pub owner_ref: Option<String>,
    pub board_position: i32,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}
