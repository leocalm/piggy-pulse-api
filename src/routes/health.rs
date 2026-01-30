use rocket::{http::Status, routes};

#[rocket::get("/")]
pub async fn healthcheck() -> Status {
    Status::Ok
}

pub fn routes() -> Vec<rocket::Route> {
    routes![healthcheck]
}

#[cfg(test)]
mod tests {
    //! Integration tests for routes require a running PostgreSQL database
    //! configured at the DATABASE_URL specified in the test Config.
    //! These tests verify the full request/response cycle including database connections.
    //!
    //! To run integration tests, ensure PostgreSQL is running with appropriate test credentials.

    use crate::{Config, build_rocket};
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn health_check_works() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        let response = client.get("/api/health").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
    }
}
