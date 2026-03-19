use crate::database::postgres_repository::PostgresRepository;
use crate::dto::misc::UnlockResponse;
use crate::error::app_error::AppError;

pub struct UnlockService<'a> {
    repository: &'a PostgresRepository,
}

impl<'a> UnlockService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        UnlockService { repository }
    }

    pub async fn unlock_by_token(&self, token: &str, ip_address: &str) -> Result<UnlockResponse, AppError> {
        let success = self.repository.verify_and_apply_unlock_token_v2(token, ip_address).await?;

        if success {
            Ok(UnlockResponse {
                message: "Account unlocked successfully. You can now log in.".to_string(),
            })
        } else {
            Err(AppError::BadRequest("Invalid or expired unlock token".to_string()))
        }
    }
}
