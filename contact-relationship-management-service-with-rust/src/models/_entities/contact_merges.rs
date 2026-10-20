//! `SeaORM` Entity -- `contact_merges`. The merge ledger (CRM-R24): one row per merge, with the ids of every row that moved and what is needed to undo it (CRM-R26).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "contact_merges")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub survivor_pid: Uuid,
    pub merged_pid: Uuid,
    pub merged_person_ref: String,
    pub actor: Option<String>,
    pub moved: Json,
    /// What the merge changed beyond moving rows; `None` for a merge made
    /// before the undo record existed, which cannot be undone.
    pub before: Option<Json>,
    pub unmerged_at: Option<DateTimeWithTimeZone>,
    pub unmerged_actor: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
