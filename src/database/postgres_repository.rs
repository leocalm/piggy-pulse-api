use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresRepository {
    pub pool: PgPool,
}

impl PostgresRepository {
    // Placeholder: repository methods expect to run in request context where CurrentUser is available.
    // For now provide a helper that returns a zero UUID; routes should pass the real user id when available.
    pub fn current_user_id(&self) -> Uuid {
        // TODO: integrate with request guard; using nil UUID to avoid compilation errors in this refactor
        Uuid::nil()
    }
}
