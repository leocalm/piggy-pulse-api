use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::budget_period::{BudgetPeriod, BudgetPeriodRequest};
use tokio_postgres::Row;
use uuid::Uuid;

pub async fn get_budget_period(client: &tokio_postgres::Client, budget_period_id: &Uuid) -> Result<BudgetPeriod, AppError> {
    let rows = client
        .query(
            r#"
            SELECT id, name, start_date, end_date, created_at
            FROM budget_period
            WHERE id = $1
            "#,
            &[budget_period_id],
        )
        .await?;

    if let Some(row) = rows.first() {
        Ok(map_row_to_budget_period(row))
    } else {
        Err(AppError::NotFound("Budget period not found".to_string()))
    }
}

#[async_trait::async_trait]
pub trait BudgetPeriodRepository {
    async fn create_budget_period(&self, request: &BudgetPeriodRequest) -> Result<Uuid, AppError>;
    async fn list_budget_periods(&self) -> Result<Vec<BudgetPeriod>, AppError>;
    async fn get_current_budget_period(&self) -> Result<BudgetPeriod, AppError>;
    async fn get_budget_period(&self, budget_period_id: &Uuid) -> Result<BudgetPeriod, AppError>;
    async fn update_budget_period(&self, id: &Uuid, request: &BudgetPeriodRequest) -> Result<BudgetPeriod, AppError>;
    async fn delete_budget_period(&self, id: &Uuid) -> Result<(), AppError>;
}

#[async_trait::async_trait]
impl<'a> BudgetPeriodRepository for PostgresRepository<'a> {
    async fn create_budget_period(&self, request: &BudgetPeriodRequest) -> Result<Uuid, AppError> {
        let rows = self
            .client
            .query(
                r#"
            INSERT INTO budget_period (name, start_date, end_date)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
                &[&request.name, &request.start_date, &request.end_date],
            )
            .await?;

        if let Some(row) = rows.first() {
            Ok(row.get("id"))
        } else {
            Err(AppError::Db("Unable to create budget period".to_string()))
        }
    }

    async fn list_budget_periods(&self) -> Result<Vec<BudgetPeriod>, AppError> {
        let rows = self
            .client
            .query(
                r#"
            SELECT id, name, start_date, end_date, created_at
            FROM budget_period
            ORDER BY start_date
            "#,
                &[],
            )
            .await?;

        Ok(rows.into_iter().map(|r| map_row_to_budget_period(&r)).collect())
    }

    async fn get_current_budget_period(&self) -> Result<BudgetPeriod, AppError> {
        let rows = self
            .client
            .query(
                r#"
            SELECT id, name, start_date, end_date, created_at
            FROM budget_period
            WHERE start_date <= now()
                AND end_date >= now()
            "#,
                &[],
            )
            .await?;

        if let Some(row) = rows.first() {
            Ok(map_row_to_budget_period(row))
        } else {
            Err(AppError::Db("Unable to get current budget period".to_string()))
        }
    }

    async fn get_budget_period(&self, budget_period_id: &Uuid) -> Result<BudgetPeriod, AppError> {
        get_budget_period(self.client, budget_period_id).await
    }

    async fn update_budget_period(&self, id: &Uuid, request: &BudgetPeriodRequest) -> Result<BudgetPeriod, AppError> {
        let rows = self
            .client
            .query(
                r#"
            UPDATE budget_period
            SET name = $1, start_date = $2, end_date = $3
            WHERE id = $4
            RETURNING id, name, start_date, end_date, created_at
            "#,
                &[&request.name, &request.start_date, &request.end_date, &id],
            )
            .await?;

        if let Some(row) = rows.first() {
            Ok(map_row_to_budget_period(row))
        } else {
            Err(AppError::NotFound("Budget period not found".to_string()))
        }
    }

    async fn delete_budget_period(&self, id: &Uuid) -> Result<(), AppError> {
        self.client
            .execute(
                r#"
            DELETE FROM budget_period
            WHERE id = $1
            "#,
                &[id],
            )
            .await?;
        Ok(())
    }
}

fn map_row_to_budget_period(row: &Row) -> BudgetPeriod {
    BudgetPeriod {
        id: row.get("id"),
        name: row.get("name"),
        start_date: row.get("start_date"),
        end_date: row.get("end_date"),
        created_at: row.get("created_at"),
    }
}
