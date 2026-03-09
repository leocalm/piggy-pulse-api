use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::models::dashboard_card::{
    AvailableCardsResponse, AvailableEntity, AvailableEntityCardType, AvailableGlobalCard, CreateDashboardCardRequest, DashboardCardResponse,
    ENTITY_CARD_TYPES, GLOBAL_CARD_TYPES, MAX_CARDS_PER_USER, MAX_ENTITY_CARDS_PER_TYPE, ReorderRequest, UpdateDashboardCardRequest,
    default_size_for_card_type, entity_table_for_card_type, is_entity_card_type, is_valid_card_type,
};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, delete, get, post, put};
use rocket_okapi::openapi;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

/// Get the user's dashboard layout. Seeds defaults if no cards exist.
#[openapi(tag = "Dashboard Layout")]
#[get("/")]
pub async fn get_layout(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Json<Vec<DashboardCardResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let mut cards = repo.get_dashboard_cards(&current_user.id).await?;

    if cards.is_empty() {
        cards = repo.seed_default_dashboard_cards(&current_user.id).await?;
    }

    Ok(Json(cards.iter().map(DashboardCardResponse::from).collect()))
}

/// Add a new card to the dashboard layout.
#[openapi(tag = "Dashboard Layout")]
#[post("/", data = "<payload>")]
pub async fn create_card(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<CreateDashboardCardRequest>,
) -> Result<(Status, Json<DashboardCardResponse>), AppError> {
    payload.validate()?;
    let req = payload.into_inner();

    if !is_valid_card_type(&req.card_type) {
        return Err(AppError::BadRequest(format!("Unknown card type: {}", req.card_type)));
    }

    if req.position < 0 {
        return Err(AppError::BadRequest("Position must be non-negative".to_string()));
    }

    let is_entity = is_entity_card_type(&req.card_type);

    // Validate entity_id presence
    if is_entity && req.entity_id.is_none() {
        return Err(AppError::BadRequest("Entity cards require entity_id".to_string()));
    }
    if !is_entity && req.entity_id.is_some() {
        return Err(AppError::BadRequest("Global cards must not have entity_id".to_string()));
    }

    let repo = PostgresRepository { pool: pool.inner().clone() };

    // Check total card limit
    let total = repo.count_dashboard_cards(&current_user.id).await?;
    if total >= MAX_CARDS_PER_USER {
        return Err(AppError::BadRequest(format!("Maximum of {} dashboard cards reached", MAX_CARDS_PER_USER)));
    }

    // Check per-type entity card limit
    if is_entity {
        let type_count = repo.count_entity_cards_by_type(&current_user.id, &req.card_type).await?;
        if type_count >= MAX_ENTITY_CARDS_PER_TYPE {
            return Err(AppError::BadRequest(format!(
                "Maximum of {} cards of type {} reached",
                MAX_ENTITY_CARDS_PER_TYPE, req.card_type
            )));
        }
    }

    // Validate entity ownership
    if let Some(entity_id) = &req.entity_id {
        let table = entity_table_for_card_type(&req.card_type).ok_or_else(|| AppError::BadRequest("Unknown entity card type".to_string()))?;
        if !repo.entity_exists(table, entity_id, &current_user.id).await? {
            return Err(AppError::NotFound("Entity not found".to_string()));
        }
    }

    let size = default_size_for_card_type(&req.card_type).ok_or_else(|| AppError::BadRequest("Unknown card type".to_string()))?;

    let card = repo
        .create_dashboard_card(&current_user.id, &req.card_type, req.entity_id.as_ref(), size, req.position, req.enabled)
        .await?;

    Ok((Status::Created, Json(DashboardCardResponse::from(&card))))
}

/// Update a dashboard card (position, enabled, entity_id).
#[openapi(tag = "Dashboard Layout")]
#[put("/<card_id>", data = "<payload>")]
pub async fn update_card(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    card_id: &str,
    payload: Json<UpdateDashboardCardRequest>,
) -> Result<Json<DashboardCardResponse>, AppError> {
    payload.validate()?;
    let req = payload.into_inner();
    let card_uuid = Uuid::parse_str(card_id).map_err(|e| AppError::uuid("Invalid card id", e))?;

    if let Some(pos) = req.position
        && pos < 0
    {
        return Err(AppError::BadRequest("Position must be non-negative".to_string()));
    }

    let repo = PostgresRepository { pool: pool.inner().clone() };

    // If entity_id is being updated, validate it
    if let Some(new_entity_id) = &req.entity_id {
        let existing = repo.get_dashboard_card_by_id(&card_uuid, &current_user.id).await?;

        if !is_entity_card_type(&existing.card_type) {
            return Err(AppError::BadRequest("Cannot set entity_id on a global card".to_string()));
        }

        let table = entity_table_for_card_type(&existing.card_type).ok_or_else(|| AppError::BadRequest("Unknown entity card type".to_string()))?;
        if !repo.entity_exists(table, new_entity_id, &current_user.id).await? {
            return Err(AppError::NotFound("Entity not found".to_string()));
        }
    }

    let card = repo
        .update_dashboard_card(&card_uuid, &current_user.id, req.position, req.enabled, req.entity_id)
        .await?;

    Ok(Json(DashboardCardResponse::from(&card)))
}

/// Bulk reorder dashboard cards.
#[openapi(tag = "Dashboard Layout")]
#[put("/reorder", data = "<payload>")]
pub async fn reorder_cards(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<ReorderRequest>,
) -> Result<Json<Vec<DashboardCardResponse>>, AppError> {
    payload.validate()?;
    let req = payload.into_inner();

    if req.order.is_empty() {
        return Err(AppError::BadRequest("Order list cannot be empty".to_string()));
    }

    // Validate positions are contiguous from 0
    let mut positions: Vec<i32> = req.order.iter().map(|e| e.position).collect();
    positions.sort();
    for (i, pos) in positions.iter().enumerate() {
        if *pos != i as i32 {
            return Err(AppError::BadRequest("Positions must be contiguous starting from 0".to_string()));
        }
    }

    let order: Vec<(Uuid, i32)> = req.order.iter().map(|e| (e.id, e.position)).collect();

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let cards = repo.reorder_dashboard_cards(&current_user.id, &order).await?;

    Ok(Json(cards.iter().map(DashboardCardResponse::from).collect()))
}

/// Delete a dashboard card (entity cards only).
#[openapi(tag = "Dashboard Layout")]
#[delete("/<card_id>")]
pub async fn delete_card(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser, card_id: &str) -> Result<Status, AppError> {
    let card_uuid = Uuid::parse_str(card_id).map_err(|e| AppError::uuid("Invalid card id", e))?;

    let repo = PostgresRepository { pool: pool.inner().clone() };

    // Check that it's an entity card
    let existing = repo.get_dashboard_card_by_id(&card_uuid, &current_user.id).await?;
    if !is_entity_card_type(&existing.card_type) {
        return Err(AppError::BadRequest("Global cards cannot be deleted. Disable them instead.".to_string()));
    }

    repo.delete_dashboard_card(&card_uuid, &current_user.id).await?;
    Ok(Status::NoContent)
}

/// Reset dashboard layout to defaults.
#[openapi(tag = "Dashboard Layout")]
#[post("/reset")]
pub async fn reset_layout(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Json<Vec<DashboardCardResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    repo.delete_all_dashboard_cards(&current_user.id).await?;
    let cards = repo.seed_default_dashboard_cards(&current_user.id).await?;
    Ok(Json(cards.iter().map(DashboardCardResponse::from).collect()))
}

/// Get available card types for adding to the dashboard.
#[openapi(tag = "Dashboard Layout")]
#[get("/available-cards")]
pub async fn available_cards(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Json<AvailableCardsResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };

    let existing = repo.get_existing_card_types(&current_user.id).await?;

    // Build global cards list
    let global_cards: Vec<AvailableGlobalCard> = GLOBAL_CARD_TYPES
        .iter()
        .map(|ct| {
            let already_added = existing.iter().any(|(t, _)| t == ct);
            AvailableGlobalCard {
                card_type: ct.to_string(),
                default_size: default_size_for_card_type(ct).unwrap_or(crate::models::dashboard_card::CardSize::Half),
                already_added,
            }
        })
        .collect();

    // Build entity cards list
    let mut entity_cards: Vec<AvailableEntityCardType> = Vec::new();
    for ct in ENTITY_CARD_TYPES {
        let table = match entity_table_for_card_type(ct) {
            Some(t) => t,
            None => continue,
        };

        let entities = repo.get_available_entities(table, &current_user.id).await?;
        let available_entities: Vec<AvailableEntity> = entities
            .into_iter()
            .map(|(id, name)| {
                let already_added = existing.iter().any(|(t, eid)| t == ct && *eid == Some(id));
                AvailableEntity { id, name, already_added }
            })
            .collect();

        entity_cards.push(AvailableEntityCardType {
            card_type: ct.to_string(),
            default_size: default_size_for_card_type(ct).unwrap_or(crate::models::dashboard_card::CardSize::Half),
            available_entities,
        });
    }

    Ok(Json(AvailableCardsResponse { global_cards, entity_cards }))
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![get_layout, create_card, update_card, reorder_cards, delete_card, reset_layout, available_cards,]
}

#[cfg(test)]
mod tests {
    use crate::{Config, build_rocket};
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::Value;
    use uuid::Uuid;

    fn test_config() -> Config {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string(); // pragma: allowlist secret
        config.rate_limit.require_client_ip = false;
        config.session.cookie_secure = false;
        config
    }

    async fn create_user_and_auth(client: &Client) -> String {
        let unique = Uuid::new_v4();
        let payload = serde_json::json!({
            "name": format!("Test User {}", unique),
            "email": format!("test.dashboard.{}@example.com", unique),
            "password": "CorrectHorseBatteryStaple!2026" // pragma: allowlist secret
        });

        let response = client
            .post("/api/v1/users/")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("user response body");
        let user_json: Value = serde_json::from_str(&body).expect("valid user json");
        let user_id = user_json["id"].as_str().expect("user id").to_string();
        let user_email = user_json["email"].as_str().expect("user email").to_string();

        let login_payload = serde_json::json!({
            "email": user_email,
            "password": "CorrectHorseBatteryStaple!2026" // pragma: allowlist secret
        });

        let login_response = client
            .post("/api/v1/users/login")
            .header(ContentType::JSON)
            .body(login_payload.to_string())
            .dispatch()
            .await;
        assert_eq!(login_response.status(), Status::Ok);

        user_id
    }

    async fn create_account(client: &Client) -> String {
        let payload = serde_json::json!({
            "name": format!("Test Account {}", Uuid::new_v4()),
            "color": "#123456",
            "icon": "wallet",
            "account_type": "Checking",
            "balance": 10_000,
            "spend_limit": null
        });

        let response = client
            .post("/api/v1/accounts/")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("account response body");
        let json: Value = serde_json::from_str(&body).expect("valid json");
        json["id"].as_str().expect("account id").to_string()
    }

    fn parse_json(body: &str) -> Value {
        serde_json::from_str(body).expect("valid json")
    }

    const BASE: &str = "/api/v1/dashboard-layout";

    // ─── GET /dashboard-layout ─────────────────────────────────────────

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn get_layout_unauthorized() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        let response = client.get(BASE).dispatch().await;
        assert_eq!(response.status(), Status::Unauthorized);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn get_layout_seeds_defaults_on_first_call() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;

        let response = client.get(BASE).dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("body");
        let cards: Vec<Value> = serde_json::from_str(&body).expect("json array");
        assert_eq!(cards.len(), 3);
        assert_eq!(cards[0]["card_type"].as_str().unwrap(), "current_period");
        assert_eq!(cards[0]["size"].as_str().unwrap(), "full");
        assert_eq!(cards[0]["position"].as_i64().unwrap(), 0);
        assert!(cards[0]["enabled"].as_bool().unwrap());
        assert_eq!(cards[1]["card_type"].as_str().unwrap(), "budget_stability");
        assert_eq!(cards[1]["size"].as_str().unwrap(), "half");
        assert_eq!(cards[1]["position"].as_i64().unwrap(), 1);
        assert_eq!(cards[2]["card_type"].as_str().unwrap(), "net_position");
        assert_eq!(cards[2]["size"].as_str().unwrap(), "half");
        assert_eq!(cards[2]["position"].as_i64().unwrap(), 2);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn get_layout_response_shape() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;

        let body = client.get(BASE).dispatch().await.into_string().await.expect("body");
        let cards: Vec<Value> = serde_json::from_str(&body).expect("json");

        for card in &cards {
            assert!(card["id"].is_string());
            assert!(card["card_type"].is_string());
            assert!(card["size"].is_string());
            assert!(card["position"].is_number());
            assert!(card["enabled"].is_boolean());
            // entity_id can be null or string
            assert!(card["entity_id"].is_null() || card["entity_id"].is_string());
        }
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn get_layout_scoped_to_user() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");

        // Create user A and get their layout
        create_user_and_auth(&client).await;
        let body_a = client.get(BASE).dispatch().await.into_string().await.expect("body");
        let cards_a: Vec<Value> = serde_json::from_str(&body_a).expect("json");
        let id_a = cards_a[0]["id"].as_str().unwrap().to_string();

        // Create user B (new session replaces A's)
        create_user_and_auth(&client).await;
        let body_b = client.get(BASE).dispatch().await.into_string().await.expect("body");
        let cards_b: Vec<Value> = serde_json::from_str(&body_b).expect("json");

        // User B's cards should have different IDs
        for card in &cards_b {
            assert_ne!(card["id"].as_str().unwrap(), &id_a);
        }
    }

    // ─── POST /dashboard-layout ────────────────────────────────────────

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn add_global_card_success() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;
        client.get(BASE).dispatch().await; // seed defaults

        let payload = serde_json::json!({
            "card_type": "recent_transactions",
            "entity_id": null,
            "position": 3
        });

        let response = client.post(BASE).header(ContentType::JSON).body(payload.to_string()).dispatch().await;
        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("body");
        let card = parse_json(&body);
        assert_eq!(card["card_type"].as_str().unwrap(), "recent_transactions");
        assert_eq!(card["size"].as_str().unwrap(), "full"); // Fixed size from registry
        assert!(card["entity_id"].is_null());
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn add_entity_card_success() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;
        client.get(BASE).dispatch().await;

        let account_id = create_account(&client).await;

        let payload = serde_json::json!({
            "card_type": "account_summary",
            "entity_id": account_id,
            "position": 3
        });

        let response = client.post(BASE).header(ContentType::JSON).body(payload.to_string()).dispatch().await;
        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("body");
        let card = parse_json(&body);
        assert_eq!(card["card_type"].as_str().unwrap(), "account_summary");
        assert_eq!(card["entity_id"].as_str().unwrap(), account_id);
        assert_eq!(card["size"].as_str().unwrap(), "half");
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn add_duplicate_global_card_conflict() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;
        client.get(BASE).dispatch().await; // seeds current_period, budget_stability, net_position

        // Try to add current_period again
        let payload = serde_json::json!({
            "card_type": "current_period",
            "entity_id": null,
            "position": 3
        });

        let response = client.post(BASE).header(ContentType::JSON).body(payload.to_string()).dispatch().await;
        assert_eq!(response.status(), Status::Conflict);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn add_duplicate_entity_card_conflict() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;
        client.get(BASE).dispatch().await;

        let account_id = create_account(&client).await;

        let payload = serde_json::json!({
            "card_type": "account_summary",
            "entity_id": account_id,
            "position": 3
        });

        // First add succeeds
        let r1 = client.post(BASE).header(ContentType::JSON).body(payload.to_string()).dispatch().await;
        assert_eq!(r1.status(), Status::Created);

        // Second add with same entity_id conflicts
        let payload2 = serde_json::json!({
            "card_type": "account_summary",
            "entity_id": account_id,
            "position": 4
        });
        let r2 = client.post(BASE).header(ContentType::JSON).body(payload2.to_string()).dispatch().await;
        assert_eq!(r2.status(), Status::Conflict);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn add_entity_card_missing_entity_id() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;
        client.get(BASE).dispatch().await;

        let payload = serde_json::json!({
            "card_type": "account_summary",
            "entity_id": null,
            "position": 3
        });

        let response = client.post(BASE).header(ContentType::JSON).body(payload.to_string()).dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn add_global_card_with_entity_id() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;
        client.get(BASE).dispatch().await;

        let payload = serde_json::json!({
            "card_type": "recent_transactions",
            "entity_id": Uuid::new_v4().to_string(),
            "position": 3
        });

        let response = client.post(BASE).header(ContentType::JSON).body(payload.to_string()).dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn add_invalid_card_type() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;
        client.get(BASE).dispatch().await;

        let payload = serde_json::json!({
            "card_type": "nonexistent_type",
            "entity_id": null,
            "position": 3
        });

        let response = client.post(BASE).header(ContentType::JSON).body(payload.to_string()).dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn add_entity_not_found() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;
        client.get(BASE).dispatch().await;

        let payload = serde_json::json!({
            "card_type": "account_summary",
            "entity_id": Uuid::new_v4().to_string(),
            "position": 3
        });

        let response = client.post(BASE).header(ContentType::JSON).body(payload.to_string()).dispatch().await;
        assert_eq!(response.status(), Status::NotFound);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn add_card_unauthorized() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");

        let payload = serde_json::json!({
            "card_type": "recent_transactions",
            "entity_id": null,
            "position": 0
        });

        let response = client.post(BASE).header(ContentType::JSON).body(payload.to_string()).dispatch().await;
        assert_eq!(response.status(), Status::Unauthorized);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn add_card_size_auto_set_from_registry() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;
        client.get(BASE).dispatch().await;

        // recent_transactions has fixed size "full" — request does not include size
        let payload = serde_json::json!({
            "card_type": "recent_transactions",
            "entity_id": null,
            "position": 3
        });

        let response = client.post(BASE).header(ContentType::JSON).body(payload.to_string()).dispatch().await;
        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("body");
        let card = parse_json(&body);
        assert_eq!(card["size"].as_str().unwrap(), "full");
    }

    // ─── PUT /dashboard-layout/<card_id> ───────────────────────────────

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn update_card_enabled() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;

        let body = client.get(BASE).dispatch().await.into_string().await.expect("body");
        let cards: Vec<Value> = serde_json::from_str(&body).expect("json");
        let card_id = cards[0]["id"].as_str().unwrap();

        let payload = serde_json::json!({ "enabled": false });
        let response = client
            .put(format!("{}/{}", BASE, card_id))
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);

        let updated = parse_json(&response.into_string().await.expect("body"));
        assert!(!updated["enabled"].as_bool().unwrap());
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn update_card_position() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;

        let body = client.get(BASE).dispatch().await.into_string().await.expect("body");
        let cards: Vec<Value> = serde_json::from_str(&body).expect("json");
        let card_id = cards[0]["id"].as_str().unwrap();

        let payload = serde_json::json!({ "position": 5 });
        let response = client
            .put(format!("{}/{}", BASE, card_id))
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);

        let updated = parse_json(&response.into_string().await.expect("body"));
        assert_eq!(updated["position"].as_i64().unwrap(), 5);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn update_entity_card_entity_id() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;
        client.get(BASE).dispatch().await;

        let account_a = create_account(&client).await;
        let account_b = create_account(&client).await;

        // Create entity card pointing to account A
        let create_payload = serde_json::json!({
            "card_type": "account_summary",
            "entity_id": account_a,
            "position": 3
        });
        let create_resp = client.post(BASE).header(ContentType::JSON).body(create_payload.to_string()).dispatch().await;
        assert_eq!(create_resp.status(), Status::Created);
        let card = parse_json(&create_resp.into_string().await.expect("body"));
        let card_id = card["id"].as_str().unwrap();

        // Update to point to account B
        let update_payload = serde_json::json!({ "entity_id": account_b });
        let update_resp = client
            .put(format!("{}/{}", BASE, card_id))
            .header(ContentType::JSON)
            .body(update_payload.to_string())
            .dispatch()
            .await;
        assert_eq!(update_resp.status(), Status::Ok);

        let updated = parse_json(&update_resp.into_string().await.expect("body"));
        assert_eq!(updated["entity_id"].as_str().unwrap(), account_b);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn update_card_not_found() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;
        client.get(BASE).dispatch().await;

        let payload = serde_json::json!({ "enabled": false });
        let response = client
            .put(format!("{}/{}", BASE, Uuid::new_v4()))
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn update_card_unauthorized() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");

        let payload = serde_json::json!({ "enabled": false });
        let response = client
            .put(format!("{}/{}", BASE, Uuid::new_v4()))
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Unauthorized);
    }

    // ─── PUT /dashboard-layout/reorder ─────────────────────────────────

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn reorder_success() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;

        let body = client.get(BASE).dispatch().await.into_string().await.expect("body");
        let cards: Vec<Value> = serde_json::from_str(&body).expect("json");
        assert_eq!(cards.len(), 3);

        // Reverse the order
        let order = serde_json::json!({
            "order": [
                { "id": cards[2]["id"], "position": 0 },
                { "id": cards[1]["id"], "position": 1 },
                { "id": cards[0]["id"], "position": 2 },
            ]
        });

        let response = client
            .put(format!("{}/reorder", BASE))
            .header(ContentType::JSON)
            .body(order.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);

        let reordered: Vec<Value> = serde_json::from_str(&response.into_string().await.expect("body")).expect("json");
        assert_eq!(reordered[0]["id"], cards[2]["id"]);
        assert_eq!(reordered[1]["id"], cards[1]["id"]);
        assert_eq!(reordered[2]["id"], cards[0]["id"]);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn reorder_non_contiguous_positions() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;

        let body = client.get(BASE).dispatch().await.into_string().await.expect("body");
        let cards: Vec<Value> = serde_json::from_str(&body).expect("json");

        let order = serde_json::json!({
            "order": [
                { "id": cards[0]["id"], "position": 0 },
                { "id": cards[1]["id"], "position": 2 }, // gap at 1
                { "id": cards[2]["id"], "position": 3 },
            ]
        });

        let response = client
            .put(format!("{}/reorder", BASE))
            .header(ContentType::JSON)
            .body(order.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn reorder_foreign_card_ids() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;
        client.get(BASE).dispatch().await;

        let order = serde_json::json!({
            "order": [
                { "id": Uuid::new_v4().to_string(), "position": 0 },
            ]
        });

        let response = client
            .put(format!("{}/reorder", BASE))
            .header(ContentType::JSON)
            .body(order.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn reorder_unauthorized() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");

        let order = serde_json::json!({
            "order": [{ "id": Uuid::new_v4().to_string(), "position": 0 }]
        });

        let response = client
            .put(format!("{}/reorder", BASE))
            .header(ContentType::JSON)
            .body(order.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Unauthorized);
    }

    // ─── DELETE /dashboard-layout/<card_id> ─────────────────────────────

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn delete_entity_card_success() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;
        client.get(BASE).dispatch().await;

        let account_id = create_account(&client).await;
        let payload = serde_json::json!({
            "card_type": "account_summary",
            "entity_id": account_id,
            "position": 3
        });
        let create_resp = client.post(BASE).header(ContentType::JSON).body(payload.to_string()).dispatch().await;
        assert_eq!(create_resp.status(), Status::Created);
        let card = parse_json(&create_resp.into_string().await.expect("body"));
        let card_id = card["id"].as_str().unwrap();

        let delete_resp = client.delete(format!("{}/{}", BASE, card_id)).dispatch().await;
        assert_eq!(delete_resp.status(), Status::NoContent);

        // Verify it's gone
        let layout = client.get(BASE).dispatch().await.into_string().await.expect("body");
        let cards: Vec<Value> = serde_json::from_str(&layout).expect("json");
        assert!(cards.iter().all(|c| c["id"].as_str().unwrap() != card_id));
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn delete_global_card_rejected() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;

        let body = client.get(BASE).dispatch().await.into_string().await.expect("body");
        let cards: Vec<Value> = serde_json::from_str(&body).expect("json");
        let card_id = cards[0]["id"].as_str().unwrap(); // current_period — a global card

        let response = client.delete(format!("{}/{}", BASE, card_id)).dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn delete_card_not_owned() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;

        let response = client.delete(format!("{}/{}", BASE, Uuid::new_v4())).dispatch().await;
        assert_eq!(response.status(), Status::NotFound);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn delete_card_unauthorized() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");

        let response = client.delete(format!("{}/{}", BASE, Uuid::new_v4())).dispatch().await;
        assert_eq!(response.status(), Status::Unauthorized);
    }

    // ─── POST /dashboard-layout/reset ──────────────────────────────────

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn reset_returns_3_default_cards() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;
        client.get(BASE).dispatch().await;

        // Add extra cards
        let payload = serde_json::json!({
            "card_type": "recent_transactions",
            "entity_id": null,
            "position": 3
        });
        client.post(BASE).header(ContentType::JSON).body(payload.to_string()).dispatch().await;

        // Reset
        let response = client.post(format!("{}/reset", BASE)).dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("body");
        let cards: Vec<Value> = serde_json::from_str(&body).expect("json");
        assert_eq!(cards.len(), 3);
        assert_eq!(cards[0]["card_type"].as_str().unwrap(), "current_period");
        assert_eq!(cards[1]["card_type"].as_str().unwrap(), "budget_stability");
        assert_eq!(cards[2]["card_type"].as_str().unwrap(), "net_position");
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn reset_removes_entity_cards() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;
        client.get(BASE).dispatch().await;

        let account_id = create_account(&client).await;
        let payload = serde_json::json!({
            "card_type": "account_summary",
            "entity_id": account_id,
            "position": 3
        });
        client.post(BASE).header(ContentType::JSON).body(payload.to_string()).dispatch().await;

        // Reset
        let response = client.post(format!("{}/reset", BASE)).dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("body");
        let cards: Vec<Value> = serde_json::from_str(&body).expect("json");
        assert_eq!(cards.len(), 3);
        assert!(cards.iter().all(|c| c["entity_id"].is_null()));
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn reset_is_idempotent() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;
        client.get(BASE).dispatch().await;

        let r1 = client.post(format!("{}/reset", BASE)).dispatch().await;
        assert_eq!(r1.status(), Status::Ok);
        let body1 = r1.into_string().await.expect("body");

        let r2 = client.post(format!("{}/reset", BASE)).dispatch().await;
        assert_eq!(r2.status(), Status::Ok);
        let body2 = r2.into_string().await.expect("body");

        let cards1: Vec<Value> = serde_json::from_str(&body1).expect("json");
        let cards2: Vec<Value> = serde_json::from_str(&body2).expect("json");
        assert_eq!(cards1.len(), cards2.len());
        for i in 0..cards1.len() {
            assert_eq!(cards1[i]["card_type"], cards2[i]["card_type"]);
            assert_eq!(cards1[i]["position"], cards2[i]["position"]);
            assert_eq!(cards1[i]["size"], cards2[i]["size"]);
        }
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn reset_unauthorized() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        let response = client.post(format!("{}/reset", BASE)).dispatch().await;
        assert_eq!(response.status(), Status::Unauthorized);
    }

    // ─── GET /dashboard-layout/available-cards ─────────────────────────

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn available_cards_lists_globals_and_entities() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;
        client.get(BASE).dispatch().await; // seed defaults

        let response = client.get(format!("{}/available-cards", BASE)).dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("body");
        let data = parse_json(&body);

        let globals = data["global_cards"].as_array().expect("global_cards array");
        assert_eq!(globals.len(), 10); // all 10 global types

        // The 3 defaults should be marked as already_added
        let added_types: Vec<&str> = globals
            .iter()
            .filter(|g| g["already_added"].as_bool().unwrap())
            .map(|g| g["card_type"].as_str().unwrap())
            .collect();
        assert!(added_types.contains(&"current_period"));
        assert!(added_types.contains(&"budget_stability"));
        assert!(added_types.contains(&"net_position"));

        let entity_cards = data["entity_cards"].as_array().expect("entity_cards array");
        assert_eq!(entity_cards.len(), 3); // account_summary, category_breakdown, vendor_spend
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn available_cards_marks_added_entity() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        create_user_and_auth(&client).await;
        client.get(BASE).dispatch().await;

        let account_id = create_account(&client).await;

        // Add account card
        let payload = serde_json::json!({
            "card_type": "account_summary",
            "entity_id": account_id,
            "position": 3
        });
        client.post(BASE).header(ContentType::JSON).body(payload.to_string()).dispatch().await;

        let response = client.get(format!("{}/available-cards", BASE)).dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("body");
        let data = parse_json(&body);
        let entity_cards = data["entity_cards"].as_array().unwrap();
        let account_type = entity_cards
            .iter()
            .find(|e| e["card_type"].as_str().unwrap() == "account_summary")
            .expect("account_summary type");
        let entities = account_type["available_entities"].as_array().unwrap();
        let added = entities.iter().find(|e| e["id"].as_str().unwrap() == account_id).expect("our account");
        assert!(added["already_added"].as_bool().unwrap());
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn available_cards_unauthorized() {
        let client = Client::tracked(build_rocket(test_config())).await.expect("rocket");
        let response = client.get(format!("{}/available-cards", BASE)).dispatch().await;
        assert_eq!(response.status(), Status::Unauthorized);
    }
}
