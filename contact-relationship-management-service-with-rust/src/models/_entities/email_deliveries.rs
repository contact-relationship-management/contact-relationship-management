//! `SeaORM` Entity -- `email_deliveries`. One message to one contact for one campaign (CRM-T39); the idempotency key of a send.

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "email_deliveries")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub campaign_pid: Uuid,
    pub contact_pid: Uuid,
    pub state: String,
    pub attempts: i32,
    pub message_id: Option<String>,
    pub provider_id: Option<String>,
    pub error: Option<String>,
    pub sent_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
