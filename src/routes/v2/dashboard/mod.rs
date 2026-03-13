mod budget_stability;
mod current_period;
mod net_position;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        current_period::get_current_period,
        net_position::get_net_position,
        budget_stability::get_budget_stability,
    ]
}
