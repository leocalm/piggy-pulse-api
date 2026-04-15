use crate::crypto::Dek;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::{
    CategoryListResponse, CategoryOptionListResponse, CreateCategoryRequest, CreateTargetRequest, EncryptedCategoryResponse, EncryptedTargetResponse,
    TargetListResponse, UpdateCategoryRequest, UpdateTargetRequest, target_to_response, to_encrypted_response, to_option_response,
};
use crate::dto::common::PaginatedResponse;
use crate::error::app_error::AppError;
use uuid::Uuid;

pub struct CategoryService<'a> {
    repository: &'a PostgresRepository,
}

impl<'a> CategoryService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        CategoryService { repository }
    }

    pub async fn list_categories(&self, user_id: &Uuid) -> Result<CategoryListResponse, AppError> {
        let categories = self.repository.list_categories(user_id).await?;
        let total_count = categories.len() as i64;
        let data: Vec<EncryptedCategoryResponse> = categories.iter().map(to_encrypted_response).collect();
        Ok(PaginatedResponse {
            data,
            total_count,
            has_more: false,
            next_cursor: None,
        })
    }

    pub async fn list_category_options(&self, user_id: &Uuid) -> Result<CategoryOptionListResponse, AppError> {
        let categories = self.repository.list_categories(user_id).await?;
        Ok(categories.iter().filter(|c| !c.is_archived).map(to_option_response).collect())
    }

    pub async fn create_category(&self, request: &CreateCategoryRequest, user_id: &Uuid, dek: &Dek) -> Result<EncryptedCategoryResponse, AppError> {
        let category = self.repository.create_category(request, user_id, dek).await?;
        Ok(to_encrypted_response(&category))
    }

    pub async fn update_category(&self, id: &Uuid, request: &UpdateCategoryRequest, user_id: &Uuid, dek: &Dek) -> Result<EncryptedCategoryResponse, AppError> {
        let category = self.repository.update_category(id, request, user_id, dek).await?;
        Ok(to_encrypted_response(&category))
    }

    pub async fn delete_category(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        self.repository.delete_category(id, user_id).await
    }

    pub async fn archive_category(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        self.repository.archive_category(id, user_id).await
    }

    pub async fn unarchive_category(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        self.repository.unarchive_category(id, user_id).await
    }

    // ===== Targets =====

    pub async fn list_targets(&self, user_id: &Uuid) -> Result<TargetListResponse, AppError> {
        let rows = self.repository.list_targets(user_id).await?;
        Ok(rows
            .into_iter()
            .map(|(id, category_id, is_excluded, value_enc)| target_to_response(id, category_id, is_excluded, &value_enc))
            .collect())
    }

    pub async fn create_target(&self, request: &CreateTargetRequest, user_id: &Uuid, dek: &Dek) -> Result<EncryptedTargetResponse, AppError> {
        let (id, category_id, is_excluded, value_enc) = self.repository.create_target(request, user_id, dek).await?;
        Ok(target_to_response(id, category_id, is_excluded, &value_enc))
    }

    pub async fn update_target(&self, target_id: &Uuid, request: &UpdateTargetRequest, user_id: &Uuid, dek: &Dek) -> Result<EncryptedTargetResponse, AppError> {
        let (id, category_id, is_excluded, value_enc) = self.repository.update_target(target_id, request, user_id, dek).await?;
        Ok(target_to_response(id, category_id, is_excluded, &value_enc))
    }

    pub async fn toggle_target_excluded(&self, target_id: &Uuid, user_id: &Uuid) -> Result<EncryptedTargetResponse, AppError> {
        let (id, category_id, is_excluded, value_enc) = self.repository.toggle_target_excluded(target_id, user_id).await?;
        Ok(target_to_response(id, category_id, is_excluded, &value_enc))
    }
}
