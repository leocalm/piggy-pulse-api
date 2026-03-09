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
