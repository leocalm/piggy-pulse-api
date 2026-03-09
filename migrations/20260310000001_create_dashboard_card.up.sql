CREATE TYPE card_size AS ENUM ('half', 'full');

CREATE TABLE IF NOT EXISTS dashboard_card
(
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    card_type   TEXT        NOT NULL,
    entity_id   UUID        NULL,
    size        card_size   NOT NULL DEFAULT 'half',
    position    INTEGER     NOT NULL,
    enabled     BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_position_non_negative CHECK (position >= 0)
);

-- One global card per type per user (entity_id IS NULL)
CREATE UNIQUE INDEX idx_dashboard_card_global_unique
    ON dashboard_card (user_id, card_type) WHERE entity_id IS NULL;

-- One entity card per type+entity per user
CREATE UNIQUE INDEX idx_dashboard_card_entity_unique
    ON dashboard_card (user_id, card_type, entity_id) WHERE entity_id IS NOT NULL;

-- Fast fetch of a user's layout, ordered
CREATE INDEX idx_dashboard_card_user_position
    ON dashboard_card (user_id, position);
