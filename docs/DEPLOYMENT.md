# Deployment Guide

This document describes the production deployment model of the PiggyPulse API.

The deployment process is container-based and CI-driven.

---

## Overview

The production system consists of:

- Dockerized API service
- PostgreSQL database
- Reverse proxy / TLS termination
- CI pipeline (self-hosted)

The API is exposed via:

```bash
https://api.piggy-pulse.com
```

Documentation is hosted separately and consumes the OpenAPI specification exposed by the API.

---

## Deployment Model

The application is packaged as a Docker image.

Deployment flow:

1. Code is pushed to the main branch.
2. CI runs formatting, linting, and tests.
3. A Docker image is built.
4. The production environment pulls the updated image.
5. Containers are restarted using Docker Compose.

Production update command:

```bash
docker compose pull
docker compose down
docker compose up -d
```

Database migrations run automatically on container startup.

---

## CI Pipeline

The CI pipeline performs the following checks before deployment:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

The pipeline ensures:

- Formatting consistency
- No lint warnings
- Test suite passing
- Build validity

Only successful builds are deployed.

---

## Configuration

Production configuration is provided via environment variables.

Sensitive configuration includes:

- Database URL
- Secret keys
- Email credentials
- Argon2 parameters
- CORS configuration

Secrets are not committed to the repository.

---

## Database

- PostgreSQL is used as the primary datastore.
- Migrations are applied automatically on startup.
- No seed scripts are executed in production.
- Schema evolution is controlled via SQLx migrations.

---

## OpenAPI Documentation

The OpenAPI specification is exposed at:

```bash
/api/v1/openapi.json
```

Documentation is hosted separately and consumes the live specification.

Swagger UI is not exposed directly from the production runtime.

---

## Operational Principles

- The API is versioned under `/api/v1`.
- Breaking changes require explicit version bump.
- The OpenAPI contract is treated as a public boundary.
- No manual changes are made directly in production containers.
- Deployment is reproducible via Docker.

---

## Future Improvements

Planned improvements include:

- Observability stack (metrics and log aggregation)
- Structured health checks
- Improved deployment rollback strategy
- Dedicated staging environment
