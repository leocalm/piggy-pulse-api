mod budget_stability;
mod cash_flow;
mod current_period;
mod fixed_categories;
mod net_position;
mod spending_trend;
mod top_vendors;
mod uncategorized;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        current_period::get_current_period,
        net_position::get_net_position,
        budget_stability::get_budget_stability,
        cash_flow::get_cash_flow,
        spending_trend::get_spending_trend,
        top_vendors::get_top_vendors,
        uncategorized::get_uncategorized,
        fixed_categories::get_fixed_categories,
    ]
}
