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
    use crate::{build_rocket, Config};
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;

    #[rocket::async_test]
    async fn health_check_works() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config))
            .await
            .expect("valid rocket instance");
        let response = client.get("/api/health").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
    }
}
