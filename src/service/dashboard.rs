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
}
