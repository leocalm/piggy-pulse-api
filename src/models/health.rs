use rocket::serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[cfg(test)]
mod tests {
    use crate::rocket;
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;

    #[rocket::async_test]
    async fn health_check_works() {
        let client = Client::tracked(rocket())
            .await
            .expect("valid rocket instance");
        let response = client.get("/api/health").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
    }
}
