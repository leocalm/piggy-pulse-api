---
name: Bug report
about: Report a defect in PiggyPulse API behavior
title: "bug: "
labels: bug
assignees: ""
---

# What happened?

## Expected behavior

## Actual behavior

## Steps to reproduce

1.
2.
3.

## Impact

- Severity: low / medium / high
- Frequency: always / often / sometimes / rare

## Environment

- Commit/branch:
- Rust version (`rustc -V`):
- DB: Postgres version:
- Deployment: local / docker-compose / production

## Logs / Error output

```text
paste relevant logs here (redact secrets)
```

## Notes / Suspected cause

## Definition of done (implementation)

- [ ] Fix is covered by tests (unit/integration/e2e as appropriate)
- [ ] No user-scope leaks (all queries scoped by `current_user.id`)
- [ ] Error handling matches `AppError` patterns
- [ ] Run:
  - [ ] `cargo fmt --check`
  - [ ] `cargo clippy --workspace --all-targets -- -D warnings`
  - [ ] `cargo build --verbose`
  - [ ] `cargo test --verbose`

