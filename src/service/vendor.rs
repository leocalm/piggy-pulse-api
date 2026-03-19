use crate::database::postgres_repository::PostgresRepository;
use crate::dto::common::{PaginatedResponse, VendorMinimal};
use crate::dto::vendors::{CreateVendorRequest, VendorBase, VendorListResponse, VendorOptionListResponse, VendorResponse, VendorStatus, VendorSummaryResponse};
use crate::error::app_error::AppError;
use crate::models::vendor::VendorRequest;
use uuid::Uuid;

pub struct VendorService<'a> {
    repository: &'a PostgresRepository,
}

impl<'a> VendorService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        VendorService { repository }
    }

    pub async fn list_vendors_v2(&self, cursor: Option<Uuid>, limit: i64, user_id: &Uuid) -> Result<VendorListResponse, AppError> {
        let (mut rows, total_count) = self.repository.list_vendors_v2(cursor, limit, user_id).await?;

        let has_more = rows.len() as i64 > limit;
        if has_more {
            rows.truncate(limit as usize);
        }
        let next_cursor = if has_more { rows.last().map(|(v, _)| v.id.to_string()) } else { None };

        let data: Vec<VendorSummaryResponse> = rows
            .into_iter()
            .map(|(vendor, tx_count)| VendorSummaryResponse {
                base: VendorResponse {
                    base: VendorBase {
                        id: vendor.id,
                        name: vendor.name,
                        status: if vendor.archived { VendorStatus::Inactive } else { VendorStatus::Active },
                    },
                    description: vendor.description,
                },
                number_of_transactions: tx_count,
            })
            .collect();

        Ok(PaginatedResponse {
            data,
            total_count,
            has_more,
            next_cursor,
        })
    }

    pub async fn create_vendor(&self, request: &CreateVendorRequest, user_id: &Uuid) -> Result<VendorResponse, AppError> {
        let v1_request = VendorRequest {
            name: request.name.clone(),
            description: request.description.clone(),
        };

        let vendor = self.repository.create_vendor(&v1_request, user_id).await?;

        Ok(VendorResponse {
            base: VendorBase {
                id: vendor.id,
                name: vendor.name,
                status: if vendor.archived { VendorStatus::Inactive } else { VendorStatus::Active },
            },
            description: vendor.description,
        })
    }

    pub async fn update_vendor(&self, id: &Uuid, request: &CreateVendorRequest, user_id: &Uuid) -> Result<VendorResponse, AppError> {
        // Check existence first to return 404 before attempting update
        self.repository
            .get_vendor_by_id(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Vendor not found".to_string()))?;

        let v1_request = VendorRequest {
            name: request.name.clone(),
            description: request.description.clone(),
        };

        let vendor = self.repository.update_vendor(id, &v1_request, user_id).await?;

        Ok(VendorResponse {
            base: VendorBase {
                id: vendor.id,
                name: vendor.name,
                status: if vendor.archived { VendorStatus::Inactive } else { VendorStatus::Active },
            },
            description: vendor.description,
        })
    }

    pub async fn delete_vendor(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        self.repository
            .get_vendor_by_id(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Vendor not found".to_string()))?;

        self.repository.delete_vendor(id, user_id).await
    }

    pub async fn list_vendor_options(&self, user_id: &Uuid) -> Result<VendorOptionListResponse, AppError> {
        let vendors = self.repository.list_all_vendors(user_id).await?;
        Ok(vendors.into_iter().map(|v| VendorMinimal { id: v.id, name: v.name }).collect())
    }

    pub async fn archive_vendor(&self, id: &Uuid, user_id: &Uuid) -> Result<VendorResponse, AppError> {
        let vendor = self
            .repository
            .get_vendor_by_id(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Vendor not found".to_string()))?;

        if vendor.archived {
            return Err(AppError::Conflict("Vendor is already archived".to_string()));
        }

        let archived = self.repository.archive_vendor(id, user_id).await?;

        Ok(VendorResponse {
            base: VendorBase {
                id: archived.id,
                name: archived.name,
                status: VendorStatus::Inactive,
            },
            description: archived.description,
        })
    }

    pub async fn unarchive_vendor(&self, id: &Uuid, user_id: &Uuid) -> Result<VendorResponse, AppError> {
        let vendor = self
            .repository
            .get_vendor_by_id(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Vendor not found".to_string()))?;

        if !vendor.archived {
            return Err(AppError::Conflict("Vendor is not archived".to_string()));
        }

        let restored = self.repository.restore_vendor(id, user_id).await?;

        Ok(VendorResponse {
            base: VendorBase {
                id: restored.id,
                name: restored.name,
                status: VendorStatus::Active,
            },
            description: restored.description,
        })
    }
}
