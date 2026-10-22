//! `find_live()`: the rows of an entity that are not soft-deleted.
//!
//! Almost every read starts from "this entity, `deleted_at` is null". This
//! names it once, so a query cannot forget the filter by typing it differently.

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Select};

/// A column set with a nullable `deleted_at` (soft delete).
pub trait HasDeletedAt: ColumnTrait {
    /// The `deleted_at` column.
    fn deleted_at() -> Self;
}

/// `Entity::find_live()`.
pub trait FindLive: EntityTrait
where
    Self::Column: HasDeletedAt,
{
    /// Every row whose `deleted_at` is null.
    #[must_use]
    fn find_live() -> Select<Self> {
        Self::find().filter(Self::Column::deleted_at().is_null())
    }
}

impl<E: EntityTrait> FindLive for E where E::Column: HasDeletedAt {}

macro_rules! soft_deleted {
    ($($module:ident),* $(,)?) => {$(
        impl HasDeletedAt for super::_entities::$module::Column {
            fn deleted_at() -> Self {
                Self::DeletedAt
            }
        }
    )*};
}

soft_deleted!(
    accounts,
    activities,
    articles,
    campaigns,
    contacts,
    deals,
    forecast_snapshots,
    leads,
    memberships,
    nurture_enrollments,
    nurture_sequences,
    nurture_steps,
    partnerships,
    pipeline_stages,
    pipelines,
    segments,
    sla_policies,
    tickets,
    working_groups,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::_entities::{contacts, deals};
    use sea_orm::{DatabaseBackend, QueryTrait};

    #[test]
    fn find_live_filters_on_deleted_at() {
        let sql = contacts::Entity::find_live()
            .build(DatabaseBackend::Postgres)
            .to_string();
        assert!(sql.contains(r#""contacts"."deleted_at" IS NULL"#), "{sql}");
    }

    #[test]
    fn it_composes_with_other_filters() {
        let sql = deals::Entity::find_live()
            .filter(deals::Column::ClosedAt.is_null())
            .build(DatabaseBackend::Postgres)
            .to_string();
        assert!(sql.contains(r#""deals"."deleted_at" IS NULL"#), "{sql}");
        assert!(sql.contains(r#""deals"."closed_at" IS NULL"#), "{sql}");
    }
}
