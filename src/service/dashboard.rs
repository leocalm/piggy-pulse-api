use crate::database::postgres_repository::PostgresRepository;
use crate::dto::dashboard::{BudgetStabilityResponse, CurrentPeriodResponse, NetPositionResponse};
use crate::error::app_error::AppError;
use uuid::Uuid;

pub fn is_outside_tolerance(spent_budget: i64, total_budget: i64, tolerance_basis_points: i32) -> bool {
    if total_budget <= 0 {
        return spent_budget > 0;
    }

    let spent = spent_budget.max(0);
    let budget = total_budget;
    let tolerance = i64::from(tolerance_basis_points.max(0));

    let lower_bound = (budget * (10_000 - tolerance)) / 10_000;
    let upper_bound = (budget * (10_000 + tolerance)) / 10_000;

    spent < lower_bound || spent > upper_bound
}

pub struct DashboardService<'a> {
    repository: &'a PostgresRepository,
}

impl<'a> DashboardService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        DashboardService { repository }
    }

    pub async fn get_current_period(&self, period_id: &Uuid, user_id: &Uuid) -> Result<CurrentPeriodResponse, AppError> {
        let row = self.repository.get_current_period_dashboard(period_id, user_id).await?;

        let days_elapsed = row.days_elapsed.max(0) as i64;
        let days_in_period = row.days_in_period.max(1) as i64;
        let days_remaining = row.days_remaining.max(0) as i64;

        // Projected spend: if some days have elapsed, extrapolate to full period
        let projected_spend = if days_elapsed > 0 { (row.spent * days_in_period) / days_elapsed } else { 0 };

        Ok(CurrentPeriodResponse {
            spent: row.spent,
            target: row.target,
            days_remaining,
            days_in_period,
            projected_spend,
        })
    }

    pub async fn get_net_position(&self, period_id: &Uuid, user_id: &Uuid) -> Result<NetPositionResponse, AppError> {
        let row = self.repository.get_net_position_v2(period_id, user_id).await?;

        Ok(NetPositionResponse {
            total: row.total_net_position,
            difference_this_period: row.change_this_period,
            number_of_accounts: row.account_count,
            liquid_amount: row.liquid_balance,
            protected_amount: row.protected_balance,
            debt_amount: row.debt_balance,
        })
    }

    pub async fn get_budget_stability(&self, period_id: &Uuid, user_id: &Uuid) -> Result<BudgetStabilityResponse, AppError> {
        let result = self.repository.get_budget_stability_v2(period_id, user_id).await?;

        Ok(BudgetStabilityResponse {
            stability: result.stability,
            periods_within_range: result.periods_within_range,
            periods_stability: result.periods_stability,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::is_outside_tolerance;

    #[test]
    fn marks_spend_within_tolerance_as_inside() {
        assert!(!is_outside_tolerance(10_500, 10_000, 1000));
    }

    #[test]
    fn marks_spend_above_tolerance_as_outside() {
        assert!(is_outside_tolerance(11_500, 10_000, 1000));
    }

    #[test]
    fn marks_spend_below_tolerance_as_outside() {
        assert!(is_outside_tolerance(8_500, 10_000, 1000));
    }

    #[test]
    fn marks_positive_spend_with_zero_budget_as_outside() {
        assert!(is_outside_tolerance(1, 0, 1000));
    }

    #[test]
    fn marks_zero_spend_with_zero_budget_as_inside() {
        assert!(!is_outside_tolerance(0, 0, 1000));
    }

    #[test]
    fn marks_negative_spend_with_zero_budget_as_inside() {
        assert!(!is_outside_tolerance(-100, 0, 1000));
    }

    #[test]
    fn clamps_negative_spend_to_zero_for_positive_budget() {
        assert!(is_outside_tolerance(-100, 10_000, 1000));
    }
}
