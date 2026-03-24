mod apply_template;
mod category_templates;
mod complete;
mod status;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        status::get_onboarding_status,
        complete::complete_onboarding,
        category_templates::list_category_templates,
        apply_template::apply_template,
    ]
}
