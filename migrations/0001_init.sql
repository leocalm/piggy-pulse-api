-- Initial schema for budget API

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- 1) Enum for AccountType
-- CREATE TYPE account_type AS ENUM (
--     'Checking',
--     'Savings',
--     'CreditCard',
--     'Wallet'
--     );
--
-- CREATE TYPE transaction_type AS ENUM (
--     'Incoming',
--     'Outgoing',
--     'Transfer'
--     );

-- CREATE TYPE category_type AS ENUM (
--     'Incoming',
--     'Outgoing',
--     'Transfer'
--     );

-- 2) Currency table
CREATE TABLE IF NOT EXISTS currency
(
    id             UUID PRIMARY KEY     DEFAULT gen_random_uuid(),
    name           TEXT        NOT NULL,
    symbol         TEXT        NOT NULL,
    currency       TEXT        NOT NULL,
    decimal_places INTEGER     NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 3) Account table
CREATE TABLE IF NOT EXISTS account
(
    id           UUID PRIMARY KEY      DEFAULT gen_random_uuid(),
    name         TEXT         NOT NULL UNIQUE,
    color        TEXT         NOT NULL,
    icon         TEXT         NOT NULL,
    account_type account_type NOT NULL,
    currency_id  UUID         NOT NULL REFERENCES currency (id) ON DELETE CASCADE,
    balance      BIGINT       NOT NULL,
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS category
(
    id            UUID PRIMARY KEY       DEFAULT gen_random_uuid(),
    name          TEXT          NOT NULL UNIQUE,
    color         TEXT          NULL,
    icon          TEXT          NULL,
    parent_id     UUID          NULL REFERENCES category (id) ON DELETE CASCADE,
    category_type category_type NOT NULL,
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS vendor
(
    id         UUID PRIMARY KEY     DEFAULT gen_random_uuid(),
    name       TEXT        NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS transaction
(
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    amount           INTEGER          NOT NULL,
    description      TEXT             NOT NULL,
    occurred_at      DATE             NOT NULL,
    transaction_type transaction_type NOT NULL,
    category_id      UUID             NOT NULL REFERENCES category (id) ON DELETE CASCADE,
    from_account_id  UUID             NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    to_account_id    UUID             NULL REFERENCES account (id) ON DELETE CASCADE,
    vendor_id        UUID             NOT NULL REFERENCES vendor (id) ON DELETE CASCADE,
);

CREATE TABLE IF NOT EXISTS users
(
    id            UUID PRIMARY KEY     DEFAULT gen_random_uuid(),
    name          TEXT        NOT NULL,
    email         TEXT        NOT NULL,
    salt          TEXT        NOT NULL,
    password_hash TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS budget
(
    id         UUID PRIMARY KEY     DEFAULT gen_random_uuid(),
    name       TEXT        NOT NULL,
    start_day  INTEGER     NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS budget_category
(
    id             UUID PRIMARY KEY     DEFAULT gen_random_uuid(),
    category_id    UUID        NOT NULL REFERENCES category (id) ON DELETE CASCADE,
    budgeted_value INTEGER     NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS budget_period
(
    id         UUID PRIMARY KEY     DEFAULT gen_random_uuid(),
    name       TEXT        NOT NULL UNIQUE,
    start_date DATE        NOT NULL,
    end_date   DATE        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
)
