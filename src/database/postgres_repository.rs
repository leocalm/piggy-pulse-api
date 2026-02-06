use sqlx::PgPool;

#[derive(Clone)]
pub struct PostgresRepository {
    pub pool: PgPool,
}

pub(crate) fn is_unique_violation(err: &sqlx::Error) -> bool {
    if let sqlx::Error::Database(db_err) = err {
        return db_err.code().is_some_and(|code| code == "23505");
    }
    false
}
