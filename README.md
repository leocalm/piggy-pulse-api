# PiggyPulse API

Backend service for **PiggyPulse**, a security-focused, SaaS-ready personal budgeting platform.

Production:

- App: https://piggy-pulse.com
- API: https://api.piggy-pulse.com
- Documentation: https://docs.piggy-pulse.com

PiggyPulse is both a production-deployed beta product and a laboratory for backend architecture, domain modeling, and secure authentication design.

---

## Overview

PiggyPulse enables users to define flexible budgeting periods (not restricted to calendar months), assign category goals within those periods, and manage
accounts, transactions, and vendors.

Unlike many budgeting tools that enforce calendar-aligned months, PiggyPulse explicitly models configurable budget periods with:

- Custom start dates
- Custom durations
- Multi-user isolation
- SaaS-ready design

The backend is designed for correctness, explicit domain boundaries, and security-first authentication.

---

## Technology Stack

- Rust
- Rocket (web framework)
- SQLx (async PostgreSQL driver with compile-time query checking)
- PostgreSQL
- Docker
- Drone CI (self-hosted)
- Cloudflare (frontend hosting)

---

## Design Philosophy

PiggyPulse API was structured around:

- Explicit domain modeling
- Strong compile-time guarantees
- Clear boundary separation (DB ↔ domain ↔ API DTOs)
- SaaS-ready user isolation
- Security as a first-class concern
- Avoidance of premature complexity

The project leverages AI-assisted implementation, while architectural direction, tradeoffs, and domain modeling decisions are explicitly designed and reviewed.

---

## Architecture

The service follows a layered structure:

HTTP Layer (Rocket routes)  
↓  
Request / Response DTOs  
↓  
Domain Models  
↓  
Data Access Layer (SQLx)  
↓  
PostgreSQL

Key principles:

- API DTOs are separate from internal domain models.
- Internal database structs are never exposed externally.
- Manual mapping is used between domain entities and response objects.
- Validation occurs at the request boundary.
- Stateless API design.

This separation prevents accidental leakage of internal fields such as password hashes or audit metadata.

---

## Security Model

Authentication and authorization were designed as first-class concerns.

### Password Hashing

Passwords are hashed using **Argon2** with per-user salts and memory-hard parameters.

The choice reflects familiarity with modern PHC-era password hashing research and prioritizes resistance against GPU/ASIC-based brute-force attacks.

The author previously contributed to **Lyra**, a Password Hashing Competition candidate, during graduate research. That background informs the security-first
design of the authentication layer and the emphasis on memory-hard constructions.

Password strength validation is enforced using `zxcvbn`, rejecting weak passwords before hashing.

### Rate Limiting

Rate limiting is applied to sensitive endpoints, including:

- Login
- Password reset
- Authentication flows

This mitigates brute-force and credential-stuffing attacks.

### Two-Factor Authentication (2FA)

Optional 2FA support provides an additional verification layer during authentication.

### Password Recovery

Password reset uses:

- Time-limited tokens
- Single-use validation
- Email-based delivery
- Explicit token invalidation

Authentication flows were designed intentionally rather than retrofitted.

---

## Multi-User & SaaS Considerations

The system enforces user-level data isolation.

Design decisions include:

- Explicit ownership for financial entities
- Query boundaries scoped per user
- Stateless API
- Separation between product domain and infrastructure concerns

The architecture is SaaS-ready while remaining self-hostable.

---

## API Versioning & Governance

The API is versioned under:

`/api/v1`

### Versioning Strategy

- Breaking changes require a new major version (`/api/v2`)
- Backward-compatible additions remain within the same version
- Deprecated endpoints are documented before removal
- Field removals or type changes require a version bump
- Enum expansions are additive only

### Stability Principles

- No silent breaking changes
- Request/response schemas are treated as contract artifacts
- Validation rules are enforced at the API boundary
- Internal refactors must not affect external contracts

### Change Review Discipline

Before release:

- OpenAPI diff is reviewed
- DTO exposure is audited
- Authentication flows are manually validated
- Breaking changes requires an explicit version bump

The API contract is treated as a public boundary and reviewed accordingly.

### Documentation

Public OpenAPI specification:

https://docs.piggy-pulse.com

Production specification endpoint:

https://api.piggy-pulse.com/api/v1/openapi.json

The OpenAPI contract is treated as a first-class artifact and reflects the current production API.

---

## Deployment

Backend:

- Dockerized
- Deployed on Hetzner VPS
- CI/CD via self-hosted Drone
- Automatic deployments on release

Frontend:

- Deployed via Cloudflare Pages
- Separate deployment pipeline

API documentation:

- Hosted separately on Cloudflare Pages
- Consumes live OpenAPI specification
- Swagger UI is not exposed from the production runtime

---

## Testing Strategy

- Unit tests for domain logic
- Integration tests for API routes
- Database tests (being re-enabled with fixtures and parametrized cases)
- CI pipeline enforces build and test validation

Testing focuses on:

- Boundary correctness
- Validation rules
- Authentication flows
- Contract stability

---

## What Is Intentionally Not Included (Yet)

- Caching layer
- Event sourcing
- Observability stack (metrics/log aggregation planned)
- Role-based access control (RBAC)
- OAuth login providers (planned)
- Full onboarding flow (planned)

These omissions are deliberate to avoid premature complexity while core domain behavior stabilizes.

---

## Roadmap

- OAuth provider integration
- Structured onboarding flow
- Improved observability
- RBAC extension
- Database test automation improvements

---

## Development Notes

PiggyPulse is:

- Production-deployed (beta)
- Security-conscious
- Domain-driven
- Versioned
- Containerized
- AI-assisted in implementation
- Architecturally guided

The project serves both as a personal financial tool and as an exploration of building secure backend services in Rust.

---

## Running Locally

### Requirements

- Rust (stable)
- PostgreSQL

### Environment Variables

Create a `.env` file with:

BUDGET_DATABASE__URL=postgres://user:password@localhost:5432/budget_db

### Run with Cargo

cargo run

The API will be available at:
http://localhost:8000/api/v1

---

## Contributing

For development details, see: docs/DEVELOPMENT.md

For deployment details, see: docs/DEPLOYMENT.md

---

## License

PiggyPulse is licensed under the GNU Affero General Public License v3.0 (AGPLv3).

You are free to use, modify, and self-host the software.  
If you run a modified version as a network service, you must make the modified source code available under the same license.

See the LICENSE file for full details.


