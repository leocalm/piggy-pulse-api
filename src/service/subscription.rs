use chrono::NaiveDate;
use uuid::Uuid;

use crate::database::postgres_repository::PostgresRepository;
use crate::dto::subscriptions::{
    CreateSubscriptionRequest, SubscriptionDetailResponse, SubscriptionListResponse, SubscriptionResponse, SubscriptionStatus, UpcomingChargesResponse,
    UpdateSubscriptionRequest,
};
use crate::error::app_error::AppError;

pub struct SubscriptionService<'a> {
    repository: &'a PostgresRepository,
}

impl<'a> SubscriptionService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        SubscriptionService { repository }
    }

    pub async fn list(&self, user_id: &Uuid, status: Option<SubscriptionStatus>) -> Result<SubscriptionListResponse, AppError> {
        self.repository.list_subscriptions(user_id, status).await
    }

    pub async fn get_detail(&self, id: &Uuid, user_id: &Uuid) -> Result<SubscriptionDetailResponse, AppError> {
        self.repository
            .get_subscription_detail(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))
    }

    pub async fn create(&self, req: &CreateSubscriptionRequest, user_id: &Uuid) -> Result<SubscriptionResponse, AppError> {
        self.repository.create_subscription(req, user_id).await
    }

    pub async fn update(&self, id: &Uuid, req: &UpdateSubscriptionRequest, user_id: &Uuid) -> Result<SubscriptionResponse, AppError> {
        self.repository
            .update_subscription(id, req, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))
    }

    pub async fn delete(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        let deleted = self.repository.delete_subscription(id, user_id).await?;
        if deleted {
            Ok(())
        } else {
            Err(AppError::NotFound("Subscription not found".to_string()))
        }
    }

    pub async fn cancel(&self, id: &Uuid, user_id: &Uuid, cancellation_date: Option<&NaiveDate>) -> Result<SubscriptionResponse, AppError> {
        self.repository
            .cancel_subscription(id, user_id, cancellation_date)
            .await?
            .ok_or_else(|| AppError::NotFound("Subscription not found or already cancelled".to_string()))
    }

    pub async fn upcoming(&self, user_id: &Uuid, limit: i64) -> Result<UpcomingChargesResponse, AppError> {
        self.repository.get_upcoming_charges(user_id, limit).await
    }
}
