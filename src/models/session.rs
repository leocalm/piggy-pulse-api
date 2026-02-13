use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct Session {
    pub id: Uuid,
}

#[derive(Debug, sqlx::FromRow)]
pub struct SessionUser {
    pub id: Uuid,
    pub email: String,
}
