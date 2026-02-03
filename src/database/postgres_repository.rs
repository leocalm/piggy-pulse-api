use sqlx::PgPool;

#[derive(Clone)]
pub struct PostgresRepository {
    pub pool: PgPool,
}
