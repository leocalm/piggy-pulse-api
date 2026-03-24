use crate::database::postgres_repository::PostgresRepository;
use crate::dto::common::{Date, PaginatedResponse, VendorMinimal};
use crate::dto::vendors::{
    CreateVendorRequest, MergeVendorRequest, VendorBase, VendorDetailResponse, VendorListResponse, VendorOptionListResponse, VendorResponse,
    VendorStatsResponse, VendorStatus, VendorSummaryResponse, VendorTopCategoryItem, VendorTransactionItem, VendorTrendItem,
};
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
        let next_cursor = if has_more { rows.last().map(|(v, _, _)| v.id.to_string()) } else { None };

        let data: Vec<VendorSummaryResponse> = rows
            .into_iter()
            .map(|(vendor, tx_count, total_spend)| VendorSummaryResponse {
                base: VendorResponse {
                    base: VendorBase {
                        id: vendor.id,
                        name: vendor.name,
                        status: if vendor.archived { VendorStatus::Inactive } else { VendorStatus::Active },
                    },
                    description: vendor.description,
                },
                number_of_transactions: tx_count,
                total_spend,
            })
            .collect();

        Ok(PaginatedResponse {
            data,
            total_count,
            has_more,
            next_cursor,
        })
    }

    pub async fn get_vendor_detail(&self, vendor_id: &Uuid, period_id: &Uuid, user_id: &Uuid) -> Result<VendorDetailResponse, AppError> {
        let db = self
            .repository
            .get_vendor_detail_v2(vendor_id, user_id, period_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Vendor not found".to_string()))?;

        let average = if db.transaction_count > 0 {
            db.period_spend / db.transaction_count
        } else {
            0
        };

        let trend = db
            .trend
            .into_iter()
            .map(|r| VendorTrendItem {
                period_id: r.period_id,
                period_name: r.period_name,
                total_spend: r.total_spend,
            })
            .collect();

        let total_spend = db.total_vendor_spend;
        let top_categories = db
            .top_categories
            .into_iter()
            .map(|r| {
                let pct = if total_spend > 0 {
                    (r.total_spend as f64 / total_spend as f64) * 100.0
                } else {
                    0.0
                };
                VendorTopCategoryItem {
                    category_id: r.category_id,
                    category_name: r.category_name,
                    total_spend: r.total_spend,
                    percentage: pct,
                }
            })
            .collect();

        let recent_transactions = db
            .recent_txns
            .into_iter()
            .map(|r| VendorTransactionItem {
                id: r.id,
                date: Date(r.date),
                amount: r.amount,
                description: r.description,
                category_id: r.category_id,
                category_name: r.category_name,
            })
            .collect();

        Ok(VendorDetailResponse {
            base: VendorResponse {
                base: VendorBase {
                    id: db.vendor.id,
                    name: db.vendor.name,
                    status: if db.vendor.archived { VendorStatus::Inactive } else { VendorStatus::Active },
                },
                description: db.vendor.description,
            },
            period_spend: db.period_spend,
            transaction_count: db.transaction_count,
            average_transaction_amount: average,
            trend,
            top_categories,
            recent_transactions,
        })
    }

    pub async fn merge_vendor(&self, source_id: &Uuid, req: &MergeVendorRequest, user_id: &Uuid) -> Result<(), AppError> {
        if source_id == &req.target_vendor_id {
            return Err(AppError::BadRequest("Source and target vendors must be different".to_string()));
        }
        let found = self.repository.merge_vendor(source_id, &req.target_vendor_id, user_id).await?;
        if !found {
            return Err(AppError::NotFound("Source vendor not found".to_string()));
        }
        Ok(())
    }

    pub async fn get_stats(&self, period_id: &Uuid, user_id: &Uuid) -> Result<VendorStatsResponse, AppError> {
        let db = self.repository.get_vendor_stats_v2(user_id, period_id).await?;
        Ok(VendorStatsResponse {
            total_vendors: db.total_vendors,
            total_spend_this_period: db.total_spend,
            avg_spend_per_vendor: db.avg_spend_per_vendor,
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
