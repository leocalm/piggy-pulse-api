CREATE TYPE subscription_billing_cycle AS ENUM ('weekly', 'monthly', 'yearly');
CREATE TYPE subscription_status AS ENUM ('active', 'cancelled', 'paused');

CREATE TABLE subscription (
    id                UUID                     PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID                     NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    name              VARCHAR(255)             NOT NULL,
    category_id       UUID                     NOT NULL REFERENCES category (id) ON DELETE RESTRICT,
    vendor_id         UUID                     REFERENCES vendor (id) ON DELETE SET NULL,
    billing_amount    BIGINT                   NOT NULL CHECK (billing_amount > 0),
    billing_cycle     subscription_billing_cycle NOT NULL,
    billing_day       SMALLINT                 NOT NULL,
    next_charge_date  DATE                     NOT NULL,
    status            subscription_status      NOT NULL DEFAULT 'active',
    cancelled_at      TIMESTAMPTZ,
    created_at        TIMESTAMPTZ              NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ              NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_subscription_user_id ON subscription (user_id);
CREATE INDEX idx_subscription_status ON subscription (user_id, status);

CREATE TABLE subscription_billing_event (
    id                UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    subscription_id   UUID        NOT NULL REFERENCES subscription (id) ON DELETE CASCADE,
    transaction_id    UUID        REFERENCES transaction (id) ON DELETE SET NULL,
    amount            BIGINT      NOT NULL,
    date              DATE        NOT NULL,
    detected          BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_billing_event_subscription ON subscription_billing_event (subscription_id);
