use crate::database::postgres_repository::PostgresRepository;
use crate::dto::common::{Date, PaginatedResponse};
use crate::dto::overlay::{
    CreateOverlayRequest, InclusionMode as DtoInclusionMode, OverlayCategoryCap as DtoOverlayCategoryCap, OverlayResponse, OverlayRules as DtoOverlayRules,
    OverlayTransactionListResponse, OverlayTransactionMembership, OverlayTransactionResponse,
};
use crate::dto::transactions::{AccountRef, CategoryRef, TransactionKind, TransactionResponse, VendorRef};
use crate::error::app_error::AppError;
use crate::models::overlay::{InclusionSource, OverlayCategoryCap, OverlayRequest, OverlayRules, OverlayWithMetrics};
use uuid::Uuid;

pub struct OverlayService<'a> {
    repository: &'a PostgresRepository,
}

impl<'a> OverlayService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        OverlayService { repository }
    }

    pub async fn create_overlay(&self, request: &CreateOverlayRequest, user_id: &Uuid) -> Result<OverlayResponse, AppError> {
        let v1_request = to_v1_request(request);
        let overlay = self.repository.create_overlay(&v1_request, user_id).await?;
        Ok(to_dto(&overlay))
    }

    pub async fn get_overlay(&self, overlay_id: &Uuid, user_id: &Uuid) -> Result<OverlayResponse, AppError> {
        let overlay = self.repository.get_overlay(overlay_id, user_id).await?;
        Ok(to_dto(&overlay))
    }

    pub async fn list_overlays(&self, cursor: Option<Uuid>, limit: i64, user_id: &Uuid) -> Result<PaginatedResponse<OverlayResponse>, AppError> {
        let overlays = self.repository.list_overlays(user_id).await?;

        // Apply cursor-based pagination in-memory (V1 repo returns all)
        let filtered: Vec<_> = match cursor {
            Some(cursor_id) => {
                let pos = overlays.iter().position(|o| o.overlay.id == cursor_id);
                match pos {
                    Some(idx) => overlays.into_iter().skip(idx + 1).collect(),
                    None => overlays,
                }
            }
            None => overlays,
        };

        let total_count = filtered.len() as i64;
        let has_more = total_count > limit;
        let page: Vec<_> = filtered.into_iter().take(limit as usize).collect();
        let next_cursor = if has_more { page.last().map(|o| o.overlay.id.to_string()) } else { None };

        let data: Vec<OverlayResponse> = page.iter().map(to_dto).collect();

        Ok(PaginatedResponse {
            data,
            total_count,
            has_more,
            next_cursor,
        })
    }

    pub async fn update_overlay(&self, overlay_id: &Uuid, request: &CreateOverlayRequest, user_id: &Uuid) -> Result<OverlayResponse, AppError> {
        let v1_request = to_v1_request(request);
        let overlay = self.repository.update_overlay(overlay_id, &v1_request, user_id).await?;
        Ok(to_dto(&overlay))
    }

    pub async fn delete_overlay(&self, overlay_id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        // Verify the overlay exists first — V2 returns 404 for missing overlays
        let _ = self.repository.get_overlay(overlay_id, user_id).await?;
        self.repository.delete_overlay(overlay_id, user_id).await
    }

    pub async fn list_overlay_transactions(&self, overlay_id: &Uuid, user_id: &Uuid) -> Result<OverlayTransactionListResponse, AppError> {
        let transactions = self.repository.get_overlay_transactions(overlay_id, user_id).await?;

        let data: Vec<OverlayTransactionResponse> = transactions
            .into_iter()
            .map(|twm| {
                let tx_resp = v1_tx_to_dto(&twm.transaction);
                let membership = OverlayTransactionMembership {
                    is_included: twm.membership.is_included,
                    inclusion_source: twm.membership.inclusion_source.map(|s| match s {
                        InclusionSource::Manual => DtoInclusionMode::Manual,
                        InclusionSource::Rules => DtoInclusionMode::Rules,
                        InclusionSource::All => DtoInclusionMode::All,
                    }),
                };
                OverlayTransactionResponse {
                    transaction: tx_resp,
                    membership,
                }
            })
            .collect();

        let total_count = data.len() as i64;
        Ok(PaginatedResponse {
            data,
            total_count,
            has_more: false,
            next_cursor: None,
        })
    }

    pub async fn include_transaction(&self, overlay_id: &Uuid, transaction_id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        // Verify overlay exists
        let _ = self.repository.get_overlay(overlay_id, user_id).await?;

        // Verify transaction exists
        self.repository
            .get_transaction_by_id(transaction_id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Transaction not found".to_string()))?;

        // Check if already manually included
        if self.repository.is_transaction_manually_included(overlay_id, transaction_id).await? {
            return Err(AppError::Conflict("Transaction is already included in this overlay".to_string()));
        }

        self.repository.include_transaction(overlay_id, transaction_id, user_id).await
    }

    pub async fn exclude_transaction(&self, overlay_id: &Uuid, transaction_id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        // Verify overlay exists
        let _ = self.repository.get_overlay(overlay_id, user_id).await?;

        // Verify transaction exists
        self.repository
            .get_transaction_by_id(transaction_id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Transaction not found".to_string()))?;

        // Check if already manually excluded
        if self.repository.is_transaction_manually_excluded(overlay_id, transaction_id).await? {
            return Err(AppError::Conflict("Transaction is already excluded from this overlay".to_string()));
        }

        self.repository.exclude_transaction_v2(overlay_id, transaction_id, user_id).await
    }
}

// ===== Conversion helpers =====

fn to_v1_request(request: &CreateOverlayRequest) -> OverlayRequest {
    OverlayRequest {
        name: request.name.clone(),
        icon: request.icon.clone(),
        start_date: request.start_date.0,
        end_date: request.end_date.0,
        inclusion_mode: match request.inclusion_mode {
            DtoInclusionMode::Manual => crate::models::overlay::InclusionMode::Manual,
            DtoInclusionMode::Rules => crate::models::overlay::InclusionMode::Rules,
            DtoInclusionMode::All => crate::models::overlay::InclusionMode::All,
        },
        total_cap_amount: request.total_cap_amount,
        category_caps: request
            .category_caps
            .iter()
            .map(|c| OverlayCategoryCap {
                category_id: c.category_id,
                cap_amount: c.cap_amount,
            })
            .collect(),
        rules: match &request.rules {
            Some(r) => OverlayRules {
                category_ids: r.category_ids.clone().unwrap_or_default(),
                vendor_ids: r.vendor_ids.clone().unwrap_or_default(),
                account_ids: r.account_ids.clone().unwrap_or_default(),
            },
            None => OverlayRules::default(),
        },
    }
}

fn to_dto(overlay: &OverlayWithMetrics) -> OverlayResponse {
    OverlayResponse {
        id: overlay.overlay.id,
        name: overlay.overlay.name.clone(),
        icon: overlay.overlay.icon.clone(),
        start_date: Date(overlay.overlay.start_date),
        end_date: Date(overlay.overlay.end_date),
        inclusion_mode: match overlay.overlay.inclusion_mode {
            crate::models::overlay::InclusionMode::Manual => DtoInclusionMode::Manual,
            crate::models::overlay::InclusionMode::Rules => DtoInclusionMode::Rules,
            crate::models::overlay::InclusionMode::All => DtoInclusionMode::All,
        },
        total_cap_amount: overlay.overlay.total_cap_amount,
        spent_amount: overlay.spent_amount,
        transaction_count: overlay.transaction_count,
        category_caps: overlay
            .category_caps
            .iter()
            .map(|c| DtoOverlayCategoryCap {
                category_id: c.category_id,
                cap_amount: c.cap_amount,
            })
            .collect(),
        rules: DtoOverlayRules {
            category_ids: if overlay.overlay.rules.category_ids.is_empty() {
                None
            } else {
                Some(overlay.overlay.rules.category_ids.clone())
            },
            vendor_ids: if overlay.overlay.rules.vendor_ids.is_empty() {
                None
            } else {
                Some(overlay.overlay.rules.vendor_ids.clone())
            },
            account_ids: if overlay.overlay.rules.account_ids.is_empty() {
                None
            } else {
                Some(overlay.overlay.rules.account_ids.clone())
            },
        },
    }
}

fn v1_tx_to_dto(tx: &crate::models::transaction::TransactionResponse) -> TransactionResponse {
    use crate::dto::categories::CategoryType;

    let from_account = AccountRef {
        id: tx.from_account.id,
        name: tx.from_account.name.clone(),
        color: tx.from_account.color.clone(),
    };

    let category = CategoryRef {
        id: tx.category.id,
        name: tx.category.name.clone(),
        color: tx.category.color.clone(),
        icon: tx.category.icon.clone(),
        category_type: CategoryType::from_v1(tx.category.category_type),
    };

    let vendor = tx.vendor.as_ref().map(|v| VendorRef {
        id: v.id,
        name: v.name.clone(),
    });

    let kind = match &tx.to_account {
        Some(to_acc) => TransactionKind::Transfer {
            to_account: AccountRef {
                id: to_acc.id,
                name: to_acc.name.clone(),
                color: to_acc.color.clone(),
            },
        },
        None => TransactionKind::Regular { to_account: None },
    };

    TransactionResponse {
        id: tx.id,
        date: Date(tx.occurred_at),
        description: tx.description.clone(),
        amount: tx.amount,
        from_account,
        category,
        vendor,
        kind,
    }
}
