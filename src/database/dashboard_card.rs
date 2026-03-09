use crate::database::postgres_repository::{PostgresRepository, is_unique_violation};
use crate::error::app_error::AppError;
use crate::models::dashboard_card::{CardSize, DEFAULT_CARDS, DashboardCard};
use uuid::Uuid;

impl PostgresRepository {
    /// Fetch all dashboard cards for a user, ordered by position.
    pub async fn get_dashboard_cards(&self, user_id: &Uuid) -> Result<Vec<DashboardCard>, AppError> {
        let cards = sqlx::query_as::<_, DashboardCard>(
            r#"
            SELECT id, user_id, card_type, entity_id, size, position, enabled, created_at, updated_at
            FROM dashboard_card
            WHERE user_id = $1
            ORDER BY position ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(cards)
    }

    /// Seed default dashboard cards for a user. Returns the seeded cards.
    pub async fn seed_default_dashboard_cards(&self, user_id: &Uuid) -> Result<Vec<DashboardCard>, AppError> {
        for default in DEFAULT_CARDS {
            sqlx::query(
                r#"
                INSERT INTO dashboard_card (user_id, card_type, entity_id, size, position, enabled)
                VALUES ($1, $2, NULL, $3, $4, TRUE)
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(user_id)
            .bind(default.card_type)
            .bind(default.size)
            .bind(default.position)
            .execute(&self.pool)
            .await?;
        }

        self.get_dashboard_cards(user_id).await
    }

    /// Insert a new dashboard card.
    pub async fn create_dashboard_card(
        &self,
        user_id: &Uuid,
        card_type: &str,
        entity_id: Option<&Uuid>,
        size: CardSize,
        position: i32,
        enabled: bool,
    ) -> Result<DashboardCard, AppError> {
        let result = sqlx::query_as::<_, DashboardCard>(
            r#"
            INSERT INTO dashboard_card (user_id, card_type, entity_id, size, position, enabled)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, user_id, card_type, entity_id, size, position, enabled, created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(card_type)
        .bind(entity_id)
        .bind(size)
        .bind(position)
        .bind(enabled)
        .fetch_one(&self.pool)
        .await;

        match result {
            Ok(card) => Ok(card),
            Err(err) if is_unique_violation(&err) => Err(AppError::BadRequest("A card of this type already exists".to_string())),
            Err(err) => Err(err.into()),
        }
    }

    /// Fetch a single dashboard card by ID, scoped to user.
    pub async fn get_dashboard_card_by_id(&self, card_id: &Uuid, user_id: &Uuid) -> Result<DashboardCard, AppError> {
        let card = sqlx::query_as::<_, DashboardCard>(
            r#"
            SELECT id, user_id, card_type, entity_id, size, position, enabled, created_at, updated_at
            FROM dashboard_card
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(card_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        card.ok_or_else(|| AppError::NotFound("Dashboard card not found".to_string()))
    }

    /// Update fields on a dashboard card. Only provided (Some) fields are updated.
    pub async fn update_dashboard_card(
        &self,
        card_id: &Uuid,
        user_id: &Uuid,
        position: Option<i32>,
        enabled: Option<bool>,
        entity_id: Option<Uuid>,
    ) -> Result<DashboardCard, AppError> {
        let result = sqlx::query_as::<_, DashboardCard>(
            r#"
            UPDATE dashboard_card
            SET position   = COALESCE($3, position),
                enabled    = COALESCE($4, enabled),
                entity_id  = COALESCE($5, entity_id),
                updated_at = now()
            WHERE id = $1 AND user_id = $2
            RETURNING id, user_id, card_type, entity_id, size, position, enabled, created_at, updated_at
            "#,
        )
        .bind(card_id)
        .bind(user_id)
        .bind(position)
        .bind(enabled)
        .bind(entity_id)
        .fetch_optional(&self.pool)
        .await;

        match result {
            Ok(Some(card)) => Ok(card),
            Ok(None) => Err(AppError::NotFound("Dashboard card not found".to_string())),
            Err(err) if is_unique_violation(&err) => Err(AppError::BadRequest("A card for this entity already exists".to_string())),
            Err(err) => Err(err.into()),
        }
    }

    /// Bulk reorder dashboard cards. Sets position for each card in the list.
    pub async fn reorder_dashboard_cards(&self, user_id: &Uuid, order: &[(Uuid, i32)]) -> Result<Vec<DashboardCard>, AppError> {
        for (card_id, position) in order {
            let rows_affected = sqlx::query(
                r#"
                UPDATE dashboard_card
                SET position = $3, updated_at = now()
                WHERE id = $1 AND user_id = $2
                "#,
            )
            .bind(card_id)
            .bind(user_id)
            .bind(position)
            .execute(&self.pool)
            .await?
            .rows_affected();

            if rows_affected == 0 {
                return Err(AppError::NotFound(format!("Dashboard card {} not found", card_id)));
            }
        }

        self.get_dashboard_cards(user_id).await
    }

    /// Delete a dashboard card by ID, scoped to user.
    pub async fn delete_dashboard_card(&self, card_id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        let rows_affected = sqlx::query(
            r#"
            DELETE FROM dashboard_card
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(card_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(AppError::NotFound("Dashboard card not found".to_string()));
        }

        Ok(())
    }

    /// Delete all dashboard cards for a user (used by reset).
    pub async fn delete_all_dashboard_cards(&self, user_id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM dashboard_card WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Count total dashboard cards for a user.
    pub async fn count_dashboard_cards(&self, user_id: &Uuid) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dashboard_card WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    /// Count entity cards of a specific type for a user.
    pub async fn count_entity_cards_by_type(&self, user_id: &Uuid, card_type: &str) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dashboard_card WHERE user_id = $1 AND card_type = $2 AND entity_id IS NOT NULL")
            .bind(user_id)
            .bind(card_type)
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    /// Check if an entity exists and belongs to the user. Uses dynamic table name.
    /// Only called for known entity tables (account, category, vendor) — not user input.
    pub async fn entity_exists(&self, table: &str, entity_id: &Uuid, user_id: &Uuid) -> Result<bool, AppError> {
        // table is always from entity_table_for_card_type, never from user input
        let query = format!("SELECT EXISTS (SELECT 1 FROM {} WHERE id = $1 AND user_id = $2)", table);
        let exists: bool = sqlx::query_scalar(&query).bind(entity_id).bind(user_id).fetch_one(&self.pool).await?;
        Ok(exists)
    }

    /// Get existing card types for a user (used by available-cards endpoint).
    pub async fn get_existing_card_types(&self, user_id: &Uuid) -> Result<Vec<(String, Option<Uuid>)>, AppError> {
        #[derive(sqlx::FromRow)]
        struct CardTypeRow {
            card_type: String,
            entity_id: Option<Uuid>,
        }

        let rows = sqlx::query_as::<_, CardTypeRow>("SELECT card_type, entity_id FROM dashboard_card WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(|r| (r.card_type, r.entity_id)).collect())
    }

    /// Get all non-archived entities of a type for a user (for available-cards).
    pub async fn get_available_entities(&self, table: &str, user_id: &Uuid) -> Result<Vec<(Uuid, String)>, AppError> {
        // table is always from entity_table_for_card_type, never from user input
        let archived_filter = if table == "account" || table == "category" || table == "vendor" {
            " AND archived = FALSE"
        } else {
            ""
        };

        let query = format!("SELECT id, name FROM {} WHERE user_id = $1{} ORDER BY name ASC", table, archived_filter);
        #[derive(sqlx::FromRow)]
        struct EntityRow {
            id: Uuid,
            name: String,
        }

        let rows = sqlx::query_as::<_, EntityRow>(&query).bind(user_id).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| (r.id, r.name)).collect())
    }
}
