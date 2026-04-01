use chrono::NaiveDate;
use uuid::Uuid;

use crate::database::postgres_repository::PostgresRepository;
use crate::dto::subscriptions::{
    CreateSubscriptionRequest, SubscriptionDetailResponse, SubscriptionListResponse, SubscriptionResponse, SubscriptionStatus, UpcomingChargesResponse,
    UpdateSubscriptionRequest,
};
use crate::error::app_error::AppError;
use crate::models::category::CategoryBehavior;

pub struct SubscriptionService<'a> {
    repository: &'a PostgresRepository,
}

impl<'a> SubscriptionService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        SubscriptionService { repository }
    }

    pub async fn list(&self, user_id: &Uuid, status: Option<SubscriptionStatus>, category_id: Option<Uuid>) -> Result<SubscriptionListResponse, AppError> {
        self.repository.list_subscriptions(user_id, status, category_id).await
    }

    pub async fn get_detail(&self, id: &Uuid, user_id: &Uuid) -> Result<SubscriptionDetailResponse, AppError> {
        self.repository
            .get_subscription_detail(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))
    }

    pub async fn create(&self, req: &CreateSubscriptionRequest, user_id: &Uuid) -> Result<SubscriptionResponse, AppError> {
        let response = self.repository.create_subscription(req, user_id).await?;
        self.sync_category_target(&req.category_id, user_id).await?;
        Ok(response)
    }

    pub async fn update(&self, id: &Uuid, req: &UpdateSubscriptionRequest, user_id: &Uuid) -> Result<SubscriptionResponse, AppError> {
        // Get the old subscription to check if category changed
        let old_sub = self.repository.get_subscription(id, user_id).await?;
        let old_category_id = old_sub.as_ref().map(|s| s.category_id);

        let response = self
            .repository
            .update_subscription(id, req, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;

        // Sync the new category's target
        self.sync_category_target(&req.category_id, user_id).await?;

        // If category changed, also sync the old category
        if let Some(old_cat_id) = old_category_id
            && old_cat_id != req.category_id
        {
            self.sync_category_target(&old_cat_id, user_id).await?;
        }

        Ok(response)
    }

    pub async fn delete(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        // Get category_id before deleting
        let sub = self
            .repository
            .get_subscription(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Subscription not found".to_string()))?;
        let category_id = sub.category_id;

        let deleted = self.repository.delete_subscription(id, user_id).await?;
        if !deleted {
            return Err(AppError::NotFound("Subscription not found".to_string()));
        }

        self.sync_category_target(&category_id, user_id).await?;
        Ok(())
    }

    pub async fn cancel(&self, id: &Uuid, user_id: &Uuid, cancellation_date: Option<&NaiveDate>) -> Result<SubscriptionResponse, AppError> {
        let response = self
            .repository
            .cancel_subscription(id, user_id, cancellation_date)
            .await?
            .ok_or_else(|| AppError::NotFound("Subscription not found or already cancelled".to_string()))?;

        self.sync_category_target(&response.category_id, user_id).await?;
        Ok(response)
    }

    pub async fn upcoming(&self, user_id: &Uuid, limit: i64) -> Result<UpcomingChargesResponse, AppError> {
        self.repository.get_upcoming_charges(user_id, limit).await
    }

    /// Recompute and persist the monthly target for a subscription-behavior category.
    /// No-op if the category doesn't have subscription behavior.
    async fn sync_category_target(&self, category_id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        let category = self.repository.get_category_by_id(category_id, user_id).await?;
        let Some(cat) = category else { return Ok(()) };

        if cat.behavior != Some(CategoryBehavior::Subscription) {
            return Ok(());
        }

        let monthly_total = self.repository.compute_monthly_target_for_category(category_id, user_id).await?;

        self.repository.upsert_category_target(category_id, monthly_total, user_id).await?;

        Ok(())
    }
}
