# API Lifecycle and Versioning Strategy

## Overview

The PiggyPulse API uses URL-based versioning to ensure backward compatibility and allow for smooth transitions between API versions. All endpoints are prefixed with a version identifier (e.g., `/api/v1/`).

## Current Version

**Current Stable Version:** `v1`

Base URL (default): `/api/v1`

## Versioning Strategy

### Version Format

- Versions follow a simple integer format: `v1`, `v2`, `v3`, etc.
- All API endpoints are prefixed with the version: `/api/{version}/{resource}`

### When to Introduce a New Version

A new major version will be introduced when:

1. **Breaking Changes** are necessary:
   - Removing or renaming existing endpoints
   - Changing request/response payload structures in incompatible ways
   - Modifying authentication mechanisms
   - Changing error response formats
   - Altering the behavior of existing endpoints in ways that could break client code

2. **Non-Breaking Changes** do NOT require a new version:
   - Adding new endpoints
   - Adding optional fields to request payloads
   - Adding new fields to response payloads (clients should ignore unknown fields)
   - Bug fixes that don't change the API contract
   - Performance improvements

## Version Support Policy

### Active Support

- The current stable version receives full support including:
  - New features
  - Bug fixes
  - Security updates
  - Performance improvements
  - Documentation updates

### Deprecation Timeline

When a new major version is released:

1. **Announcement (T+0)**: New version announced with migration guide
2. **Deprecation Notice (T+0)**: Previous version marked as deprecated
3. **Deprecation Period (6 months)**: Both versions actively maintained
   - Critical bug fixes and security patches for deprecated version
   - No new features added to deprecated version
4. **Sunset Notice (T+5 months)**: 30-day warning before removal
5. **End of Life (T+6 months)**: Deprecated version removed

### Version Status

| Version | Status | Released | Deprecated | End of Life |
|---------|--------|----------|------------|-------------|
| v1      | Active | 2026-02   | -          | -           |

## Migration Strategy

### For API Consumers

When migrating to a new API version:

1. **Review the Migration Guide**: Each new version will include a comprehensive migration guide
2. **Test in Parallel**: Both versions will run simultaneously during the deprecation period
3. **Update Gradually**: Update your integration at your own pace during the 6-month window
4. **Monitor Deprecation Headers**: API responses include deprecation information in headers

### Deprecation Response Headers

Deprecated API versions will include the following headers in responses:

```http
Deprecation: true
Sunset: Sat, 31 Aug 2026 23:59:59 GMT
Link: </api/v2>; rel="successor-version"
```

## Breaking Changes Documentation

All breaking changes will be documented in the migration guide with:

- Description of the change
- Reason for the change
- Before/after examples
- Migration steps
- Code snippets showing the new approach

## Future Version Planning

### Potential v2 Features (Tentative)

When a v2 is introduced, it may include:

- Enhanced filtering and sorting capabilities
- GraphQL endpoint alongside REST
- Improved batch operation support
- Webhooks for real-time notifications
- Enhanced rate limiting with per-endpoint controls

*Note: This is not a commitment but rather potential areas of improvement that might warrant a version bump.*

## API Design Principles

To minimize the need for breaking changes, we follow these principles:

1. **Additive Changes**: Prefer adding optional fields over modifying existing ones
2. **Backwards Compatibility**: Design new features to work with existing data
3. **Field Evolution**: Use nullable/optional fields to allow gradual adoption
4. **Stable Identifiers**: Never change the meaning of resource IDs or enums
5. **Clear Contracts**: Maintain comprehensive API documentation and OpenAPI specs

## Communication Channels

API version announcements and deprecation notices will be communicated through:

- Release notes in this repository
- API response headers (for active deprecations)
- OpenAPI documentation updates
- GitHub repository announcements

## Questions and Support

For questions about API versioning, deprecations, or migrations:

1. Check the migration guide for your version
2. Review the OpenAPI documentation at `/api/v1/docs` (default)
3. Open an issue in the GitHub repository
4. Consult the AGENTS.md file for technical implementation details

Note: Swagger/OpenAPI docs are exposed for each configured base path (e.g., `/api/v2/docs` when `/api/v2` is enabled).

Example configuration:

```toml
[api]
base_path = "/api/v1"
additional_base_paths = ["/api/v2"]
```

## Version History

### v1 (February 2026)

**Initial Release**

- User authentication and management
- Account management (checking, savings, investment accounts)
- PiggyPulse creation and management
- PiggyPulse periods with date ranges
- Category management (income/outgoing)
- Transaction tracking
- Vendor management
- Multi-currency support
- Dashboard analytics
- Rate limiting
- Cursor-based pagination
- OpenAPI/Swagger documentation
