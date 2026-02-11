# Docker Deployment Guide

This guide covers running the Budget API using Docker Compose with Caddy as a reverse proxy.

## Architecture

The Docker Compose setup includes:

- **PostgreSQL**: Database server with persistent storage
- **Budget API**: Rust/Rocket application running on port 8000 (internal)
- **Cron Worker**: Lightweight cron container that calls `/api/v1/cron/generate-periods`
- **Caddy**: Reverse proxy and web server (ports 80/443)
- **Adminer**: Database management UI (debug profile only, port 8080)

## Quick Start

### 1. Configure Environment Variables

Copy the `.env` file and update the required values:

```bash
cp .env .env.local
```

**IMPORTANT**: Before running, you must set:

1. **ROCKET_SECRET_KEY**: Generate with `openssl rand -base64 32`
2. **POSTGRES_PASSWORD**: Choose a strong password for production
3. **CRON_TOKEN**: Must match `BUDGET_CRON__AUTH_TOKEN` configured on the API

```bash
# Generate a secret key
openssl rand -base64 32

# Update .env file with the generated key
# ROCKET_SECRET_KEY=<generated-key>
# POSTGRES_PASSWORD=<strong-password>
# CRON_TOKEN=<same token as BUDGET_CRON__AUTH_TOKEN>
```

### 2. Start the Full Stack

```bash
docker compose up -d
```

This will:
1. Start PostgreSQL database
2. Build and start the Budget API
3. Build and start the cron worker
4. Start Caddy reverse proxy
5. Run database migrations automatically

### 3. Access the Application

- **API**: http://localhost/api/v1/health
- **API Documentation**: http://localhost/api/v1/docs (if enabled)
- **Adminer** (debug mode): http://localhost:8080

## Configuration

All configuration is managed through the `.env` file. Key settings:

### Database Configuration

```env
POSTGRES_DB=budget_db
POSTGRES_USER=postgres
POSTGRES_PASSWORD=your-secure-password
```

### API Server

```env
BUDGET_SERVER_PORT=8000
BUDGET_API_BASE_PATH=/api/v1
BUDGET_API_EXPOSE_DOCS=true
```

### Caddy Reverse Proxy

```env
CADDY_DOMAIN=localhost          # Change to your domain for production
CADDY_HTTP_PORT=80
CADDY_HTTPS_PORT=443
```

### CORS (Cross-Origin Resource Sharing)

```env
# Development: Allow all origins
BUDGET_CORS_ALLOWED_ORIGINS=["*"]

# Production: Restrict to your domain
# BUDGET_CORS_ALLOWED_ORIGINS=["https://yourdomain.com"]
BUDGET_CORS_ALLOW_CREDENTIALS=false
```

## Common Operations

### View Logs

```bash
# All services
docker compose logs -f

# Specific service
docker compose logs -f budget
docker compose logs -f caddy
docker compose logs -f db
```

### Run Database Migrations

Migrations run automatically when the budget container starts via the entrypoint script. To run manually:

```bash
# Run migrations using sqlx-cli
docker compose exec budget sqlx migrate run --source /app/migrations

# Check migration status
docker compose exec budget sqlx migrate info --source /app/migrations
```

### Access Database

Using Adminer (debug mode):

```bash
docker compose --profile debug up -d adminer
```

Then open http://localhost:8080 and connect with:
- **System**: PostgreSQL
- **Server**: db
- **Username**: postgres (or your POSTGRES_USER)
- **Password**: (your POSTGRES_PASSWORD)
- **Database**: budget_db (or your POSTGRES_DB)

### Rebuild Application

After code changes:

```bash
docker compose up -d --build budget
```

### Stop Services

```bash
# Stop all services
docker compose down

# Stop and remove volumes (WARNING: deletes database data)
docker compose down -v
```

## Production Deployment

### Security Checklist

1. **Generate Strong Secret Key**
   ```bash
   openssl rand -base64 32
   ```

2. **Set Strong Database Password**
   ```env
   POSTGRES_PASSWORD=<strong-random-password>
   ```

3. **Configure Domain**
   ```env
   CADDY_DOMAIN=api.yourdomain.com
   ```

4. **Restrict CORS**
   ```env
   BUDGET_CORS_ALLOWED_ORIGINS=["https://yourdomain.com"]
   ```

5. **Disable API Documentation** (optional)
   ```env
   BUDGET_API_EXPOSE_DOCS=false
   ```

6. **Enable JSON Logging**
   ```env
   BUDGET_LOGGING_JSON_FORMAT=true
   ```

### HTTPS with Caddy

Caddy automatically obtains and renews SSL certificates from Let's Encrypt when:

1. You set `CADDY_DOMAIN` to a real domain (not localhost)
2. The domain points to your server's IP
3. Ports 80 and 443 are accessible from the internet

No additional configuration needed!

### Resource Limits

Add resource limits in `docker-compose.yaml`:

```yaml
budget:
  deploy:
    resources:
      limits:
        cpus: '1.0'
        memory: 512M
      reservations:
        cpus: '0.5'
        memory: 256M
```

## Troubleshooting

### Budget API won't start

1. Check logs: `docker compose logs budget`
2. Verify DATABASE_URL is correct
3. Ensure ROCKET_SECRET_KEY is set
4. Check database is healthy: `docker compose ps db`

### Database connection errors

```bash
# Check database is running
docker compose ps db

# Check database logs
docker compose logs db

# Test database connection
docker compose exec db psql -U postgres -d budget_db -c "SELECT 1;"
```

### Caddy not accessible

1. Check if ports are already in use: `lsof -i :80` or `lsof -i :443`
2. Verify Caddyfile syntax: `docker compose exec caddy caddy validate --config /etc/caddy/Caddyfile`
3. Check Caddy logs: `docker compose logs caddy`

### Reset Everything

```bash
# Stop and remove everything including volumes
docker compose down -v

# Remove images
docker compose down --rmi all

# Start fresh
docker compose up -d --build
```

## Development Mode

Run with Adminer for database management:

```bash
docker compose --profile debug up -d
```

Access Adminer at http://localhost:8080

## Health Checks

All services include health checks:

```bash
# Check service health
docker compose ps

# Test API health endpoint
curl http://localhost/api/v1/health
```

Expected response:
```json
{
  "status": "ok",
  "database": "connected"
}
```

## Backup and Restore

### Backup Database

```bash
docker compose exec db pg_dump -U postgres budget_db > backup.sql
```

### Restore Database

```bash
cat backup.sql | docker compose exec -T db psql -U postgres budget_db
```

## Environment Variables Reference

See `.env` file for all available configuration options. Key categories:

- **POSTGRES_***: PostgreSQL configuration
- **BUDGET_DATABASE_***: Database connection pool settings
- **BUDGET_SERVER_***: API server configuration
- **BUDGET_API_***: API endpoint settings
- **BUDGET_CORS_***: CORS policy
- **BUDGET_RATE_LIMIT_***: Rate limiting settings
- **BUDGET_LOGGING_***: Logging configuration
- **ROCKET_SECRET_KEY**: Session encryption key (required)
- **CADDY_***: Reverse proxy settings
- **ADMINER_***: Database UI settings
