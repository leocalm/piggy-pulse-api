use crate::models::account::{Account, AccountRequest, AccountType};
use crate::models::budget_category::{BudgetCategory, BudgetCategoryRequest};
use crate::models::budget_period::BudgetPeriod;
use crate::models::category::{Category, CategoryType};
use crate::models::currency::Currency;
use crate::models::transaction::{Transaction, TransactionRequest};
use crate::models::vendor::Vendor;
use chrono::NaiveDate;
use uuid::Uuid;

impl From<&TransactionRequest> for Transaction {
    fn from(transaction_request: &TransactionRequest) -> Self {
        let to_account = transaction_request.to_account_id.as_ref().map(|acc_id| Account {
            id: *acc_id,
            ..Account::default()
        });

        let vendor = transaction_request.vendor_id.as_ref().map(|v_id| Vendor {
            id: *v_id,
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
                currency: "USD".to_string(), // Default currency for tests
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

pub fn sample_account() -> Account {
    Account {
        id: Uuid::new_v4(),
        user_id: Uuid::nil(),
        name: "Sample Account".to_string(),
        color: "#000000".to_string(),
        icon: "icon".to_string(),
        account_type: AccountType::Checking,
        currency: Currency {
            id: Uuid::new_v4(),
            name: "US Dollar".to_string(),
            symbol: "$".to_string(),
            currency: "USD".to_string(),
            decimal_places: 2,
            ..Currency::default()
        },
        balance: 1_000,
        spend_limit: None,
        ..Account::default()
    }
}

pub fn sample_transaction() -> Transaction {
    Transaction {
        id: Uuid::new_v4(),
        user_id: Uuid::nil(),
        amount: 500,
        description: "Sample".into(),
        occurred_at: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        category: Category {
            id: Uuid::new_v4(),
            category_type: CategoryType::Outgoing,
            name: "Food".into(),
            ..Category::default()
        },
        from_account: sample_account(),
        to_account: None,
        vendor: Some(Vendor {
            id: Uuid::new_v4(),
            name: "Vendor".into(),
            ..Vendor::default()
        }),
    }
}

pub fn sample_budget_category() -> BudgetCategory {
    BudgetCategory {
        id: Uuid::new_v4(),
        user_id: Uuid::nil(),
        category: Category {
            id: Uuid::new_v4(),
            name: "Food".into(),
            category_type: CategoryType::Outgoing,
            ..Category::default()
        },
        budgeted_value: 1_000,
        ..BudgetCategory::default()
    }
}

pub fn sample_budget_period() -> BudgetPeriod {
    BudgetPeriod {
        id: Uuid::new_v4(),
        user_id: Uuid::nil(),
        name: "Jan".into(),
        start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        end_date: NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
        ..BudgetPeriod::default()
    }
}
