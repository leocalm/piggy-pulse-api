use chrono::NaiveDate;
use uuid::Uuid;

use crate::crypto::Dek;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::subscriptions::{CreateSubscriptionRequest, EncryptedSubscriptionResponse, SubscriptionListResponse, UpdateSubscriptionRequest};
use crate::error::app_error::AppError;

pub struct SubscriptionService<'a> {
    repository: &'a PostgresRepository,
}

impl<'a> SubscriptionService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        SubscriptionService { repository }
    }

    pub async fn list(&self, user_id: &Uuid) -> Result<SubscriptionListResponse, AppError> {
        self.repository.list_subscriptions(user_id).await
    }

    pub async fn create(&self, req: &CreateSubscriptionRequest, user_id: &Uuid, dek: &Dek) -> Result<EncryptedSubscriptionResponse, AppError> {
        self.repository.create_subscription(req, user_id, dek).await
    }

    pub async fn update(&self, id: &Uuid, req: &UpdateSubscriptionRequest, user_id: &Uuid, dek: &Dek) -> Result<EncryptedSubscriptionResponse, AppError> {
        self.repository.update_subscription(id, req, user_id, dek).await
    }

    pub async fn delete(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        self.repository.delete_subscription(id, user_id).await
    }

    pub async fn cancel(&self, id: &Uuid, user_id: &Uuid, cancellation_date: Option<&NaiveDate>) -> Result<EncryptedSubscriptionResponse, AppError> {
        self.repository.cancel_subscription(id, user_id, cancellation_date).await
    }
}
