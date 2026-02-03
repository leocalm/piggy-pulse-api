use crate::database::account::AccountRepository;
use crate::database::budget::BudgetRepository;
use crate::database::budget_category::BudgetCategoryRepository;
use crate::database::transaction::TransactionRepository;
use crate::error::app_error::AppError;
use crate::models::account::{Account, AccountRequest};
use crate::models::budget::{Budget, BudgetRequest};
use crate::models::budget_category::{BudgetCategory, BudgetCategoryRequest};
use crate::models::category::Category;
use crate::models::currency::Currency;
use crate::models::transaction::{Transaction, TransactionRequest};
use crate::models::vendor::Vendor;
use uuid::Uuid;

impl From<&TransactionRequest> for Transaction {
    fn from(transaction_request: &TransactionRequest) -> Self {
        let to_account = transaction_request.to_account_id.as_ref().map(|acc_id| Account {
            id: acc_id.clone(),
            ..Account::default()
        });

        let vendor = transaction_request.vendor_id.as_ref().map(|v_id| Vendor {
            id: v_id.clone(),
            ..Vendor::default()
        });

        Self {
            id: Uuid::new_v4(),
            user_id: Uuid::nil(),
            amount: transaction_request.amount,
            description: transaction_request.description.clone(),
            occurred_at: transaction_request.occurred_at,
            category: Category {
                id: transaction_request.category_id,
                user_id: Uuid::nil(),
                ..Category::default()
            },
            from_account: Account {
                id: transaction_request.from_account_id,
                user_id: Uuid::nil(),
                ..Account::default()
            },
            to_account,
            vendor,
        }
    }
}

impl From<&AccountRequest> for Account {
    fn from(request: &AccountRequest) -> Self {
        Account {
            id: Uuid::new_v4(),
            name: request.name.clone(),
            color: request.color.clone(),
            icon: request.icon.clone(),
            account_type: request.account_type,
            currency: Currency {
                currency: request.currency.clone(),
                ..Currency::default()
            },
            balance: request.balance,
            spend_limit: request.spend_limit,
            ..Account::default()
        }
    }
}

impl From<&BudgetCategoryRequest> for BudgetCategory {
    fn from(request: &BudgetCategoryRequest) -> Self {
        Self {
            id: Uuid::new_v4(),
            category: Category {
                id: request.category_id,
                ..Category::default()
            },
            budgeted_value: request.budgeted_value,
            ..BudgetCategory::default()
        }
    }
}

impl From<&BudgetRequest> for Budget {
    fn from(request: &BudgetRequest) -> Self {
        Budget {
            id: Uuid::new_v4(),
            name: request.name.clone(),
            start_day: request.start_day,
            ..Budget::default()
        }
    }
}

pub struct MockRepository {}

#[async_trait::async_trait]
impl AccountRepository for MockRepository {
    async fn create_account(&self, request: &AccountRequest, _user_id: &Uuid) -> Result<Account, AppError> {
        Ok(request.into())
    }

    async fn get_account_by_id(&self, id: &Uuid, _user_id: &Uuid) -> Result<Option<Account>, AppError> {
        Ok(Some(Account { id: *id, ..Account::default() }))
    }

    async fn list_accounts(&self, _params: &crate::models::pagination::CursorParams, _user_id: &Uuid) -> Result<Vec<Account>, AppError> {
        Ok(vec![Account::default()])
    }

    async fn delete_account(&self, _id: &Uuid, _user_id: &Uuid) -> Result<(), AppError> {
        Ok(())
    }

    async fn update_account(&self, id: &Uuid, request: &AccountRequest, _user_id: &Uuid) -> Result<Account, AppError> {
        let mut account: Account = request.into();
        account.id = *id;
        Ok(account)
    }
}

#[async_trait::async_trait]
impl BudgetCategoryRepository for MockRepository {
    async fn create_budget_category(&self, request: &BudgetCategoryRequest) -> Result<BudgetCategory, AppError> {
        Ok(request.into())
    }

    async fn get_budget_category_by_id(&self, id: &Uuid) -> Result<Option<BudgetCategory>, AppError> {
        Ok(Some(BudgetCategory {
            id: *id,
            ..BudgetCategory::default()
        }))
    }

    async fn list_budget_categories(&self, _params: &crate::models::pagination::CursorParams) -> Result<Vec<BudgetCategory>, AppError> {
        Ok(vec![BudgetCategory::default()])
    }

    async fn delete_budget_category(&self, _id: &Uuid) -> Result<(), AppError> {
        Ok(())
    }

    async fn update_budget_category_value(&self, _id: &Uuid, _new_budget_value: &i32) -> Result<(), AppError> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl TransactionRepository for MockRepository {
    async fn create_transaction(&self, transaction_request: &TransactionRequest) -> Result<Transaction, AppError> {
        Ok(Transaction {
            id: Uuid::new_v4(),
            ..transaction_request.into()
        })
    }

    async fn get_transaction_by_id(&self, id: &Uuid) -> Result<Option<Transaction>, AppError> {
        Ok(Some(Transaction {
            id: *id,
            ..Transaction::default()
        }))
    }

    async fn list_transactions(&self, _params: &crate::models::pagination::CursorParams) -> Result<Vec<Transaction>, AppError> {
        Ok(vec![Transaction::default()])
    }

    async fn get_transactions_for_period(&self, _period_id: &Uuid, _params: &crate::models::pagination::CursorParams) -> Result<Vec<Transaction>, AppError> {
        Ok(vec![Transaction::default()])
    }

    async fn delete_transaction(&self, _id: &Uuid) -> Result<(), AppError> {
        Ok(())
    }

    async fn update_transaction(&self, id: &Uuid, transaction_request: &TransactionRequest) -> Result<Transaction, AppError> {
        Ok(Transaction {
            id: *id,
            ..transaction_request.into()
        })
    }
}

#[async_trait::async_trait]
impl BudgetRepository for MockRepository {
    async fn create_budget(&self, request: &BudgetRequest) -> Result<Budget, AppError> {
        Ok(request.into())
    }

    async fn get_budget_by_id(&self, id: &Uuid) -> Result<Option<Budget>, AppError> {
        Ok(Some(Budget { id: *id, ..Budget::default() }))
    }

    async fn list_budgets(&self, _params: &crate::models::pagination::CursorParams) -> Result<Vec<Budget>, AppError> {
        Ok(vec![Budget::default()])
    }

    async fn delete_budget(&self, _id: &Uuid) -> Result<(), AppError> {
        Ok(())
    }

    async fn update_budget(&self, id: &Uuid, request: &BudgetRequest) -> Result<Budget, AppError> {
        let mut budget: Budget = request.into();
        budget.id = *id;
        Ok(budget)
    }
}
