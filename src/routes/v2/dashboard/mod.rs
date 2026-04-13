mod budget_stability;
mod cash_flow;
mod current_period;
mod current_period_history;
mod fixed_categories;
mod net_position;
mod net_position_history;
mod spending_trend;
mod subscriptions;
mod top_vendors;
mod uncategorized;
mod variable_categories;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        current_period::get_current_period,
        current_period_history::get_current_period_history,
        net_position::get_net_position,
        net_position_history::get_net_position_history,
        budget_stability::get_budget_stability,
        cash_flow::get_cash_flow,
        spending_trend::get_spending_trend,
        top_vendors::get_top_vendors,
        uncategorized::get_uncategorized,
        fixed_categories::get_fixed_categories,
        variable_categories::get_variable_categories,
        subscriptions::get_subscriptions,
    ]
}
