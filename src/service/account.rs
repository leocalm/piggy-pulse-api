use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::{
    AccountBalanceHistoryPoint, AccountBalanceHistoryResponse, AccountDetailsResponse, AccountListResponse as V2AccountListResponse, AccountResponse,
    AccountStatus, AccountSummaryListResponse, AccountSummaryResponse, CategoryBreakdownItem, LargestOutflow, StabilityContext, TransactionBreakdownItem,
};
use crate::dto::common::{Date, PaginatedResponse};
use crate::error::app_error::AppError;
use crate::models::account::{AccountBalancePerDay, AccountListResponse, AccountRequest, AccountType as ModelAccountType, AccountWithMetrics};
use crate::models::currency::CurrencyResponse;
use crate::models::dashboard::BudgetPerDayResponse;
use crate::models::pagination::CursorParams;
use std::collections::HashMap;
use uuid::Uuid;

pub struct AccountService<'a> {
    repository: &'a PostgresRepository,
}

impl<'a> AccountService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        AccountService { repository }
    }

    pub async fn list_accounts(&self, params: &CursorParams, budget_period_id: &Uuid, user_id: &Uuid) -> Result<Vec<AccountListResponse>, AppError> {
        let accounts = self.repository.list_accounts(params, budget_period_id, user_id).await?;
        if accounts.is_empty() {
            return Ok(Vec::new());
        }

        let account_ids: Vec<Uuid> = accounts.iter().map(|account| account.account.id).collect();
        let balance_per_day = self.repository.list_account_balance_per_day(&account_ids, budget_period_id, user_id).await?;

        Ok(account_responses(&accounts, &balance_per_day))
    }

    // ===== V2 Methods =====

    pub async fn list_accounts_v2(&self, cursor: Option<Uuid>, limit: i64, user_id: &Uuid) -> Result<V2AccountListResponse, AppError> {
        let (mut accounts, total_count) = self.repository.list_accounts_v2(cursor, limit, user_id).await?;

        let has_more = accounts.len() as i64 > limit;
        if has_more {
            accounts.truncate(limit as usize);
        }
        let next_cursor = if has_more { accounts.last().map(|a| a.id.to_string()) } else { None };

        let data: Vec<AccountResponse> = accounts.iter().map(AccountResponse::from).collect();

        Ok(PaginatedResponse {
            data,
            total_count,
            has_more,
            next_cursor,
        })
    }

    pub async fn list_account_summaries_v2(
        &self,
        cursor: Option<Uuid>,
        limit: i64,
        period_id: Option<Uuid>,
        user_id: &Uuid,
    ) -> Result<AccountSummaryListResponse, AppError> {
        let resolved_period_id = match period_id {
            Some(pid) => Some(pid),
            None => self.repository.get_current_period_id(user_id).await?,
        };

        let (mut accounts, total_count) = self
            .repository
            .list_accounts_summary_v2(cursor, limit, resolved_period_id.as_ref(), user_id)
            .await?;

        let has_more = accounts.len() as i64 > limit;
        if has_more {
            accounts.truncate(limit as usize);
        }
        let next_cursor = if has_more { accounts.last().map(|a| a.account.id.to_string()) } else { None };

        let data: Vec<AccountSummaryResponse> = accounts
            .iter()
            .map(|m| AccountSummaryResponse {
                id: m.account.id,
                name: m.account.name.clone(),
                account_type: crate::dto::accounts::AccountType::from(m.account.account_type),
                color: m.account.color.clone(),
                status: if m.account.is_archived {
                    AccountStatus::Inactive
                } else {
                    AccountStatus::Active
                },
                current_balance: m.current_balance,
                net_change_this_period: m.balance_change_this_period,
                next_transfer: None,
                balance_after_next_transfer: None,
                number_of_transactions: m.transaction_count,
            })
            .collect();

        Ok(PaginatedResponse {
            data,
            total_count,
            has_more,
            next_cursor,
        })
    }

    pub async fn get_balance_history(&self, account_id: &Uuid, period_id: Option<Uuid>, user_id: &Uuid) -> Result<AccountBalanceHistoryResponse, AppError> {
        self.repository
            .get_account_by_id(account_id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Account not found".to_string()))?;

        let resolved_period_id = match period_id {
            Some(pid) => Some(pid),
            None => self.repository.get_current_period_id(user_id).await?,
        };

        if let Some(pid) = resolved_period_id {
            let period = self.repository.get_budget_period(&pid, user_id).await?;
            let points = self
                .repository
                .get_account_balance_history(account_id, period.start_date, period.end_date, user_id)
                .await?;

            let response: Vec<AccountBalanceHistoryPoint> = points
                .into_iter()
                .filter(|p| p.transaction_count > 0)
                .map(|p| {
                    let date = chrono::NaiveDate::parse_from_str(&p.date, "%Y-%m-%d").map_err(|e| {
                        tracing::error!(date = %p.date, error = %e, "Malformed date in balance history — possible data integrity issue");
                        AppError::Db {
                            message: format!("Invalid date in balance history: {}", p.date),
                            source: sqlx::Error::Protocol(e.to_string()),
                        }
                    })?;
                    Ok(AccountBalanceHistoryPoint {
                        date: Date(date),
                        balance: p.balance,
                        transaction_count: p.transaction_count,
                    })
                })
                .collect::<Result<Vec<_>, AppError>>()?;

            return Ok(response);
        }

        Ok(vec![])
    }

    pub async fn get_account_details(&self, account_id: &Uuid, period_id: Option<Uuid>, user_id: &Uuid) -> Result<AccountDetailsResponse, AppError> {
        let metrics = self
            .repository
            .get_account_with_metrics(account_id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Account not found".to_string()))?;

        let resolved_period_id = match period_id {
            Some(pid) => Some(pid),
            None => self.repository.get_current_period_id(user_id).await?,
        };

        // Fetch period-scoped detail (inflow/outflow/balance_change)
        let detail = if let Some(pid) = &resolved_period_id {
            Some(self.repository.get_account_detail(account_id, pid, user_id).await?)
        } else {
            None
        };

        let inflow = detail.as_ref().map_or(0, |d| d.inflows);
        let outflow = detail.as_ref().map_or(0, |d| d.outflows);
        let net_change = detail.as_ref().map_or(0, |d| d.inflows - d.outflows);
        // Use period-scoped count when available, fall back to all-time
        let transaction_count = detail.as_ref().map_or(metrics.transaction_count, |d| d.transaction_count);

        // Fetch context (stability + category impact) when we have a period
        let context = if let Some(pid) = &resolved_period_id {
            match self.repository.get_account_context(account_id, pid, user_id).await {
                Ok(ctx) => Some(ctx),
                Err(e) => {
                    tracing::warn!(account_id = %account_id, error = %e, "Failed to fetch account context");
                    None
                }
            }
        } else {
            None
        };

        let stability_context = context.as_ref().map_or_else(
            || StabilityContext {
                periods_on_target: 0,
                average_closing_balance: 0,
                highest_closing_balance: 0,
                lowest_closing_balance: 0,
                largest_single_outflow: None,
            },
            |ctx| StabilityContext {
                periods_on_target: ctx.stability.periods_closed_positive,
                average_closing_balance: ctx.stability.avg_closing_balance,
                highest_closing_balance: ctx.stability.highest_closing_balance,
                lowest_closing_balance: ctx.stability.lowest_closing_balance,
                largest_single_outflow: if ctx.stability.largest_single_outflow > 0 {
                    Some(LargestOutflow {
                        category_name: ctx.stability.largest_single_outflow_category.clone(),
                        value: ctx.stability.largest_single_outflow,
                    })
                } else {
                    None
                },
            },
        );

        let categories_breakdown = context
            .map(|ctx| {
                ctx.category_impact
                    .into_iter()
                    .map(|c| CategoryBreakdownItem {
                        category_id: c.category_id,
                        category_name: c.category_name,
                        value: c.amount,
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Fetch transactions breakdown when we have a period
        let transactions_breakdown = if let Some(pid) = &resolved_period_id {
            let params = CursorParams { cursor: None, limit: Some(50) };
            let txs = self.repository.get_account_transactions(account_id, pid, None, &params, user_id).await?;
            txs.into_iter()
                .map(|tx| TransactionBreakdownItem {
                    date: Date(tx.occurred_at),
                    description: tx.description,
                    category_name: tx.category_name,
                    amount: tx.amount,
                    balance: tx.running_balance,
                })
                .collect()
        } else {
            vec![]
        };

        let account = &metrics.account;
        let is_allowance = matches!(account.account_type, ModelAccountType::Allowance);
        let is_credit_card = matches!(account.account_type, ModelAccountType::CreditCard);
        let is_checking = matches!(account.account_type, ModelAccountType::Checking);

        // Compute allowance-specific: spent this cycle
        let spent_this_cycle = if is_allowance {
            self.repository.get_allowance_spent_this_cycle(account_id, user_id).await.unwrap_or(0)
        } else {
            0
        };

        // Compute checking-specific: average daily balance over the period
        let avg_daily_balance = if is_checking {
            if let Some(pid) = &resolved_period_id {
                let period = self.repository.get_budget_period(pid, user_id).await?;
                self.repository
                    .get_avg_daily_balance(account_id, period.start_date, period.end_date, user_id)
                    .await
                    .unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        };

        let base = AccountSummaryResponse {
            id: account.id,
            name: account.name.clone(),
            account_type: crate::dto::accounts::AccountType::from(account.account_type),
            color: account.color.clone(),
            status: if account.is_archived {
                AccountStatus::Inactive
            } else {
                AccountStatus::Active
            },
            current_balance: metrics.current_balance,
            net_change_this_period: net_change,
            next_transfer: None,
            balance_after_next_transfer: None,
            number_of_transactions: transaction_count,
        };

        Ok(AccountDetailsResponse {
            base,
            inflow,
            outflow,
            stability_context,
            categories_breakdown,
            transactions_breakdown,
            top_up_amount: if is_allowance { account.top_up_amount } else { None },
            top_up_cycle: if is_allowance { account.top_up_cycle.clone() } else { None },
            top_up_day: if is_allowance { account.top_up_day } else { None },
            spent_this_cycle,
            statement_close_day: if is_credit_card { account.statement_close_day } else { None },
            payment_due_day: if is_credit_card { account.payment_due_day } else { None },
            avg_daily_balance,
        })
    }

    pub async fn create_account(&self, request: &AccountRequest, user_id: &Uuid) -> Result<AccountResponse, AppError> {
        let account = self.repository.create_account(request, user_id).await?;
        Ok(AccountResponse::from(&account))
    }
}

fn account_responses(accounts: &[AccountWithMetrics], balance_per_day: &[AccountBalancePerDay]) -> Vec<AccountListResponse> {
    let mut per_day_by_account = balance_per_day_map(balance_per_day);

    accounts
        .iter()
        .map(|account| {
            let account_data = &account.account;
            let per_day = per_day_by_account.remove(&account_data.id).unwrap_or_default();

            AccountListResponse {
                id: account_data.id,
                name: account_data.name.clone(),
                color: account_data.color.clone(),
                icon: account_data.icon.clone(),
                account_type: account_data.account_type,
                currency: CurrencyResponse::from(&account_data.currency),
                balance: account.current_balance,
                spend_limit: account_data.spend_limit,
                is_archived: account_data.is_archived,
                next_transfer_amount: account_data.next_transfer_amount,
                balance_per_day: per_day,
                balance_change_this_period: account.balance_change_this_period,
                transaction_count: account.transaction_count,
            }
        })
        .collect()
}

fn balance_per_day_map(balance_per_day: &[AccountBalancePerDay]) -> HashMap<Uuid, Vec<BudgetPerDayResponse>> {
    let mut per_day_by_account: HashMap<Uuid, Vec<BudgetPerDayResponse>> = HashMap::new();

    for row in balance_per_day {
        per_day_by_account.entry(row.account_id).or_default().push(BudgetPerDayResponse {
            account_name: row.account_name.clone(),
            date: row.date.clone(),
            balance: row.balance,
        });
    }

    per_day_by_account
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::sample_account;

    #[tokio::test]
    async fn test_list_accounts() {
        let account = sample_account();
        let metrics = AccountWithMetrics {
            account: account.clone(),
            current_balance: 1200,
            balance_change_this_period: 200,
            transaction_count: 2,
        };
        let balances = vec![
            AccountBalancePerDay {
                account_id: account.id,
                account_name: account.name.clone(),
                date: "2026-02-01".to_string(),
                balance: 1000,
            },
            AccountBalancePerDay {
                account_id: account.id,
                account_name: account.name.clone(),
                date: "2026-02-02".to_string(),
                balance: 1200,
            },
        ];
        let responses = account_responses(&[metrics], &balances);
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].id, account.id);
        assert_eq!(responses[0].balance, 1200);
        assert_eq!(responses[0].balance_change_this_period, 200);
        assert_eq!(responses[0].transaction_count, 2);
        assert_eq!(responses[0].balance_per_day.len(), 2);
    }

    #[tokio::test]
    async fn test_list_accounts_without_balance_per_day() {
        let account = sample_account();
        let metrics = AccountWithMetrics {
            account: account.clone(),
            current_balance: 1000,
            balance_change_this_period: 0,
            transaction_count: 0,
        };
        let responses = account_responses(&[metrics], &[]);
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].balance_per_day.len(), 0);
    }
}
