//! Plain SQL (CRM-D24): each query is written out with `sqlx::query!` and checked against the
//! schema at build time, so a reviewer reads exactly what runs. While `SeaORM` is being retired,
//! both share one pool. A handler moves whole, because a transaction belongs to one library and
//! cannot span the two.

use axa::AppContext;
use sqlx::PgPool;

/// The connection pool, which is `SeaORM`'s own.
#[must_use]
pub fn pool(ctx: &AppContext) -> &PgPool {
    ctx.db.get_postgres_connection_pool()
}
