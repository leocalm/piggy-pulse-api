use crate::crypto::Dek;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::common::PaginatedResponse;
use crate::dto::vendors::{
    CreateVendorRequest, EncryptedVendorResponse, UpdateVendorRequest, VendorListResponse, VendorOptionListResponse, to_encrypted_response, to_option_response,
};
use crate::error::app_error::AppError;
use uuid::Uuid;

pub struct VendorService<'a> {
    repository: &'a PostgresRepository,
}

impl<'a> VendorService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        VendorService { repository }
    }

    pub async fn list_vendors(&self, user_id: &Uuid) -> Result<VendorListResponse, AppError> {
        let vendors = self.repository.list_vendors(user_id).await?;
        let total_count = vendors.len() as i64;
        let data: Vec<EncryptedVendorResponse> = vendors.iter().map(to_encrypted_response).collect();
        Ok(PaginatedResponse {
            data,
            total_count,
            has_more: false,
            next_cursor: None,
        })
    }

    pub async fn list_vendor_options(&self, user_id: &Uuid) -> Result<VendorOptionListResponse, AppError> {
        let vendors = self.repository.list_vendors(user_id).await?;
        Ok(vendors.iter().filter(|v| !v.archived).map(to_option_response).collect())
    }

    pub async fn create_vendor(&self, request: &CreateVendorRequest, user_id: &Uuid, dek: &Dek) -> Result<EncryptedVendorResponse, AppError> {
        let vendor = self.repository.create_vendor(request, user_id, dek).await?;
        Ok(to_encrypted_response(&vendor))
    }

    pub async fn update_vendor(&self, id: &Uuid, request: &UpdateVendorRequest, user_id: &Uuid, dek: &Dek) -> Result<EncryptedVendorResponse, AppError> {
        let vendor = self.repository.update_vendor(id, request, user_id, dek).await?;
        Ok(to_encrypted_response(&vendor))
    }

    pub async fn delete_vendor(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        self.repository.delete_vendor(id, user_id).await
    }

    pub async fn archive_vendor(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        self.repository.archive_vendor(id, user_id).await
    }

    pub async fn unarchive_vendor(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        self.repository.unarchive_vendor(id, user_id).await
    }
}
