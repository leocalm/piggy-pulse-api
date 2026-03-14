mod get;
mod list;

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![list::list_currencies, get::get_currency]
}
