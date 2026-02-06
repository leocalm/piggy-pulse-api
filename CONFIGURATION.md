# Configuration Guide

The Budget API uses [Figment](https://docs.rs/figment/) for flexible, layered configuration management.

## Configuration Sources (Priority Order)

Configuration is loaded from multiple sources in this priority order (later sources override earlier ones):

1. **Default values** (hardcoded in `src/config.rs`)
2. **Budget.toml** (config file)
3. **Environment variables** (prefixed with `BUDGET_`)
4. **DATABASE_URL** env var (for backwards compatibility)

Rocket requires a `ROCKET_SECRET_KEY` environment variable in non-debug profiles to encrypt private cookies. Local development (debug profile) does not require this value, but generating and setting one is recommended when testing authentication flows.

## Quick Start

### Option 1: Environment Variables Only (Simplest)

```bash
# Copy the example file
cp .env.example .env

# Edit .env with your values
export DATABASE_URL=postgres://user:password@localhost:5432/budget_db
export ROCKET_SECRET_KEY=replace-with-random-base64-32-bytes

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

### Rocket Secret Key

<<<<<<< HEAD
<<<<<<< HEAD
Rocket uses `ROCKET_SECRET_KEY` to encrypt private cookies.
=======
Rocket uses `ROCKET_SECRET_KEY` to encrypt private cookies. This is required for non-debug profiles.
>>>>>>> 591f38d (Fix critical security items)
=======
Rocket uses `ROCKET_SECRET_KEY` to encrypt private cookies.
>>>>>>> a22fc0b (Require ROCKET_SECRET_KEY in all profiles)

```bash
# Generate once and store securely (required in non-debug profiles)
export ROCKET_SECRET_KEY=$(openssl rand -base64 32)
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

### API

```toml
[api]
base_path = "/api/v1"
additional_base_paths = ["/api/v2"]
```

Or with environment variables:
```bash
BUDGET_API__BASE_PATH=/api/v1
```

Notes:
- `additional_base_paths` is easiest to set in `Budget.toml` as a list.
- If you need to set multiple base paths via environment variables, prefer the config file to avoid platform-specific list syntax.

### Rate Limiting

```toml
[rate_limit]
read_limit = 300
mutation_limit = 60
auth_limit = 10
window_seconds = 60
cleanup_interval_seconds = 60
require_client_ip = true
backend = "in_memory" # redis or in_memory
redis_url = "redis://127.0.0.1:6379/0"
redis_key_prefix = "budget:rate_limit:"
```

Or with environment variables:
```bash
BUDGET_RATE_LIMIT_READ_LIMIT=300
BUDGET_RATE_LIMIT_MUTATION_LIMIT=60
BUDGET_RATE_LIMIT_AUTH_LIMIT=10
BUDGET_RATE_LIMIT_WINDOW_SECONDS=60
BUDGET_RATE_LIMIT_CLEANUP_INTERVAL_SECONDS=60
BUDGET_RATE_LIMIT_REQUIRE_CLIENT_IP=true
BUDGET_RATE_LIMIT_BACKEND=in_memory
BUDGET_RATE_LIMIT_REDIS_URL=redis://127.0.0.1:6379/0
BUDGET_RATE_LIMIT_REDIS_KEY_PREFIX=budget:rate_limit:
```

Notes:
- The limiter uses a fixed window; bursts near the window boundary can exceed the nominal rate.
- If `require_client_ip` is enabled and the client IP cannot be determined, requests fail with 400.
- The default backend is `in_memory` for local development; set `backend = "redis"` in production.

### Session

```toml
[session]
ttl_seconds = 2592000  # 30 days
```

Or with environment variables:
```bash
BUDGET_SESSION_TTL_SECONDS=2592000
```

#### Advanced Logging Configuration with RUST_LOG

For fine-grained control over logging levels per module, use the `RUST_LOG` environment variable.
This takes precedence over `BUDGET_LOGGING_LEVEL`.

Examples:
```bash
# Set all modules to debug level
export RUST_LOG=debug

# Set only the budget crate to debug
export RUST_LOG=budget=debug

# Set specific modules to different levels
export RUST_LOG=budget::routes=trace,budget::database=debug,info

# Global info, but routes module at debug level
export RUST_LOG=info,budget::routes=debug

# Trace specific route handlers
export RUST_LOG=budget::routes::transaction=trace
```

#### Request/Response Logging

The application automatically logs:
- **Incoming requests**: method, URI, and unique request ID
- **Completed responses**: status code, request ID, method, and URI
- **Errors**: Full error context including request ID, user ID (if authenticated), method, and URI
- **Request IDs**: Added to all response headers as `X-Request-Id` for distributed tracing

Log levels for requests/responses:
- `info` level: Successful requests (2xx, 3xx status codes)
- `warn` level: Client and server errors (4xx, 5xx status codes)

#### Structured Logging for Production

Enable JSON-formatted structured logs for production environments:
```bash
export BUDGET_LOGGING_JSON_FORMAT=true
```

This outputs logs in JSON format, making them easier to parse by log aggregation tools like ELK, Datadog, or CloudWatch.

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
