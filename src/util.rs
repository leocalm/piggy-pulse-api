pub mod util {
    use crate::models::account::Account;
    use crate::models::category::CategoryType;
    use crate::models::transaction::Transaction;
    use chrono::NaiveDate;

    pub fn add_transaction(acc: i32, tx: &Transaction, account: &Account) -> i32 {
        let value = match tx.category.category_type {
            CategoryType::Incoming => tx.amount,
            CategoryType::Outgoing => -tx.amount,
            CategoryType::Transfer => {
                if tx.from_account.id == account.id {
                    -tx.amount
                } else {
                    tx.amount
                }
            }
        };
        acc + value
    }

    pub fn account_involved(account: &Account, transaction: &Transaction) -> bool {
        transaction.from_account.id == account.id || (transaction.to_account.is_some() && transaction.to_account.clone().unwrap().id == account.id)
    }

    pub fn balance_on_date(date: Option<&NaiveDate>, account: &Account, transactions: &[Transaction]) -> i32 {
        transactions
            .iter()
            .filter(|tx| account_involved(account, tx) && (date.is_none() || &tx.occurred_at < date.unwrap()))
            .fold(account.balance as i32, |acc, tx| add_transaction(acc, tx, account))
    }
}
