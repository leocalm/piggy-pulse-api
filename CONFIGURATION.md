# Configuration Guide

The Budget API uses [Figment](https://docs.rs/figment/) for flexible, layered configuration management.

## Configuration Sources (Priority Order)

Configuration is loaded from multiple sources in this priority order (later sources override earlier ones):

1. **Default values** (hardcoded in `src/config.rs`)
2. **Budget.toml** (config file)
3. **Environment variables** (prefixed with `BUDGET_`)
4. **DATABASE_URL** env var (for backwards compatibility)

## Quick Start

### Option 1: Environment Variables Only (Simplest)

```bash
# Copy the example file
cp .env.example .env

# Edit .env with your values
export DATABASE_URL=postgres://user:password@localhost:5432/budget_db

# Run the app
cargo run
```

### Option 2: Config File (Recommended for Development)

```bash
# Copy the example config
cp Budget.toml.example Budget.toml

# Edit Budget.toml with your settings
# Then run the app
cargo run
```

### Option 3: Environment Variables Override Config File (Production)

```bash
# Use Budget.toml for base config
cp Budget.toml.example Budget.toml

# Override specific values with env vars
export BUDGET_DATABASE_URL=postgres://prod-user:prod-pass@prod-host:5432/prod_db
export BUDGET_LOGGING_LEVEL=warn
export BUDGET_LOGGING_JSON_FORMAT=true

# Run the app
cargo run
```

## Configuration Options

### Database

```toml
[database]
url = "postgres://user:password@host:5432/dbname"
max_connections = 16
min_connections = 4
connection_timeout = 5   # seconds
acquire_timeout = 5      # seconds
```

Or with environment variables:
```bash
BUDGET_DATABASE_URL=postgres://user:password@host:5432/dbname
BUDGET_DATABASE_MAX_CONNECTIONS=16
BUDGET_DATABASE_MIN_CONNECTIONS=4
BUDGET_DATABASE_CONNECTION_TIMEOUT=5
BUDGET_DATABASE_ACQUIRE_TIMEOUT=5
```

### Server

```toml
[server]
port = 8000
address = "127.0.0.1"
```

Or with environment variables:
```bash
BUDGET_SERVER_PORT=8000
BUDGET_SERVER_ADDRESS=127.0.0.1
```

### Logging

```toml
[logging]
level = "info"           # trace, debug, info, warn, error
json_format = false      # true for JSON logs (good for production)
```

Or with environment variables:
```bash
BUDGET_LOGGING_LEVEL=info
BUDGET_LOGGING_JSON_FORMAT=false
```

You can also use the standard `RUST_LOG` environment variable which takes precedence.

## Environment-Specific Configuration

### Development

Create `Budget.toml`:
```toml
[database]
url = "postgres://postgres:example@localhost:5432/budget_db"
max_connections = 4

[logging]
level = "debug"
json_format = false
```

### Production

Use environment variables:
```bash
export BUDGET_DATABASE_URL=postgres://prod-user:secure-pass@prod-host:5432/budget_db
export BUDGET_DATABASE_MAX_CONNECTIONS=32
export BUDGET_LOGGING_LEVEL=warn
export BUDGET_LOGGING_JSON_FORMAT=true
```

## Examples

### Example 1: Override just the database URL

```bash
# Budget.toml has all your settings
# Just override the database URL for production
export DATABASE_URL=postgres://prod-db/budget_db
cargo run
```

### Example 2: Different logging per environment

```bash
# Development
export BUDGET_LOGGING_LEVEL=debug
cargo run

# Production
export BUDGET_LOGGING_LEVEL=warn
export BUDGET_LOGGING_JSON_FORMAT=true
cargo run
```

### Example 3: Test with different database

```bash
# Use a test database without modifying Budget.toml
BUDGET_DATABASE_URL=postgres://localhost/budget_test cargo test
```

## Validation

The application validates configuration at startup. If required fields are missing or invalid, it will fail with a clear error message:

```
Failed to load configuration: missing field `database.url`
```

## Troubleshooting

### Configuration not loading?

Check the order of precedence:
1. Is the config file in the right location? (Budget.toml in project root)
2. Are environment variables named correctly? (BUDGET_ prefix, underscores for nesting)
3. Check for typos in config keys

### Database connection fails?

Verify your DATABASE_URL format:
```
postgres://username:password@hostname:port/database_name
```

Example:
```
postgres://postgres:example@localhost:5432/budget_db
```

## Best Practices

1. ✅ **Never commit** `Budget.toml` or `.env` to git (they're in .gitignore)
2. ✅ **Use Budget.toml** for local development settings
3. ✅ **Use environment variables** for production and secrets
4. ✅ **Keep Budget.toml.example** up to date as a template
5. ✅ **Document all config options** when adding new ones
6. ✅ **Provide sensible defaults** in code
7. ✅ **Validate configuration** at startup, not at runtime
