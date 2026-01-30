use crate::models::account::Account;
use crate::models::category::CategoryType;
use crate::models::transaction::Transaction;
use chrono::NaiveDate;

pub fn add_transaction(tx: &Transaction, account: &Account) -> i32 {
    match tx.category.category_type {
        CategoryType::Incoming => tx.amount,
        CategoryType::Outgoing => -tx.amount,
        CategoryType::Transfer => {
            if tx.from_account.id == account.id {
                -tx.amount
            } else {
                tx.amount
            }
        }
    }
}

pub fn account_involved(account: &Account, transaction: &Transaction) -> bool {
    transaction.from_account.id == account.id
        || (transaction.to_account.is_some() && transaction.to_account.as_ref().is_some_and(|to_acc| to_acc.id == account.id))
}

pub fn balance_on_date(date: Option<&NaiveDate>, account: &Account, transactions: &[Transaction]) -> i32 {
    transactions
        .iter()
        .filter(|tx| account_involved(account, tx) && (date.is_none() || &tx.occurred_at < date.unwrap()))
        .fold(account.balance as i32, |acc, tx| acc + add_transaction(tx, account))
}

#[cfg(test)]
mod service_utils_tests {
    use crate::models::account::Account;
    use crate::models::category::{Category, CategoryType};
    use crate::models::transaction::Transaction;
    use crate::service::service_util::{account_involved, add_transaction, balance_on_date};
    use chrono::NaiveDate;
    use uuid::Uuid;

    #[test]
    fn add_transaction_outgoing_test() {
        let account = Account {
            balance: 10000,
            ..Account::default()
        };

        let transaction = Transaction {
            amount: 5000,
            category: Category {
                category_type: CategoryType::Outgoing,
                ..Category::default()
            },
            ..Transaction::default()
        };

        let result = add_transaction(&transaction, &account);
        assert_eq!(-5000, result);
    }

    #[test]
    fn add_transaction_incoming_test() {
        let account = Account {
            balance: 10000,
            ..Account::default()
        };

        let transaction = Transaction {
            amount: 5000,
            category: Category {
                category_type: CategoryType::Incoming,
                ..Category::default()
            },
            ..Transaction::default()
        };

        let result = add_transaction(&transaction, &account);
        assert_eq!(5000, result);
    }

    #[test]
    fn add_transaction_transfer_incoming_test() {
        let account = Account {
            id: Uuid::new_v4(),
            balance: 10000,
            ..Account::default()
        };

        let account_2 = Account {
            id: Uuid::new_v4(),
            ..Account::default()
        };

        let transaction = Transaction {
            amount: 5000,
            from_account: account_2.clone(),
            to_account: Some(account.clone()),
            category: Category {
                category_type: CategoryType::Transfer,
                ..Category::default()
            },
            ..Transaction::default()
        };

        let result = add_transaction(&transaction, &account);
        assert_eq!(5000, result);
    }

    #[test]
    fn add_transaction_transfer_outgoing_test() {
        let account = Account {
            id: Uuid::new_v4(),
            balance: 10000,
            ..Account::default()
        };

        let account_2 = Account {
            id: Uuid::new_v4(),
            ..Account::default()
        };

        let transaction = Transaction {
            amount: 5000,
            from_account: account.clone(),
            to_account: Some(account_2.clone()),
            category: Category {
                category_type: CategoryType::Transfer,
                ..Category::default()
            },
            ..Transaction::default()
        };

        let result = add_transaction(&transaction, &account);
        assert_eq!(-5000, result);
    }

    #[test]
    fn account_involved_from_test() {
        let account = Account {
            id: Uuid::new_v4(),
            ..Account::default()
        };

        let transaction = Transaction {
            from_account: account.clone(),
            ..Transaction::default()
        };

        let result = account_involved(&account, &transaction);
        assert!(result);
    }

    #[test]
    fn account_involved_to_test() {
        let account = Account {
            id: Uuid::new_v4(),
            ..Account::default()
        };

        let transaction = Transaction {
            to_account: Some(account.clone()),
            ..Transaction::default()
        };

        let result = account_involved(&account, &transaction);
        assert!(result);
    }

    #[test]
    fn account_not_involved_test() {
        let account = Account {
            id: Uuid::new_v4(),
            ..Account::default()
        };

        let account_2 = Account {
            id: Uuid::new_v4(),
            ..Account::default()
        };

        let transaction = Transaction {
            to_account: Some(account_2.clone()),
            ..Transaction::default()
        };

        let result = account_involved(&account, &transaction);
        assert!(!result);
    }

    #[test]
    fn balance_on_date_test() {
        let account = Account {
            balance: 10000,
            ..Account::default()
        };

        let transactions = [
            Transaction {
                occurred_at: NaiveDate::from_ymd_opt(2026, 1, 10).unwrap(),
                id: Uuid::new_v4(),
                amount: 1000,
                category: Category {
                    category_type: CategoryType::Outgoing,
                    ..Category::default()
                },
                ..Transaction::default()
            },
            Transaction {
                id: Uuid::new_v4(),
                occurred_at: NaiveDate::from_ymd_opt(2026, 1, 12).unwrap(),
                amount: 3500,
                category: Category {
                    category_type: CategoryType::Outgoing,
                    ..Category::default()
                },
                ..Transaction::default()
            },
        ];

        let date = &NaiveDate::from_ymd_opt(2026, 1, 11).unwrap();

        let result = balance_on_date(Some(date), &account, &transactions);
        assert_eq!(9000, result);
    }

    #[test]
    fn balance_on_date_empty_list_test() {
        let account = Account {
            balance: 10000,
            ..Account::default()
        };

        let transactions = [];

        let date = &NaiveDate::from_ymd_opt(2026, 1, 11).unwrap();

        let result = balance_on_date(Some(date), &account, &transactions);
        assert_eq!(10000, result);
    }
}
