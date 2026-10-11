# Changelog

All notable validated project changes are recorded here. This file is a release
history for the coordinated client, website, and backend work; active product work
remains in [ROADMAP.md](ROADMAP.md).

Release acceptance checks for Batches 1-3 are tracked in the
[MoSCoW verification checklist](VERIFICATION-CHECKLIST.md).

## 1.4.0 / Batch 3 - 2026-10-10

### Added

- Private capsule submission foundations across the client, website, and backend:
  immutable owner-scoped reservations, bounded raw-JAR uploads, backend-observed
  SHA-256 binding, status refresh, eligible retry, and withdrawal.
- Private filesystem quarantine/retained-artifact/evidence storage, SQLite-backed
  job leases/retries/expiry, hardened bounded JAR validation, and digest-based
  deduplication that preserves release ownership and history.
- A separate backend worker and versioned digest-bound scan evidence. The default
  scanner is deliberately disabled and produces private `scan_blocked` /
  `scanning_not_configured` results.
- Website and native client submission/status controls with strict response
  validation, progress reporting, account-change cleanup, and no credential or
  local-path exposure to the frontend.

### Changed

- Client release version to **1.4.0**, website package version to **0.2.0**, and
  backend/OpenAPI version to **0.3.0**.
- The capsule contract now uses revisioned operational states and owner-only
  endpoints for reservation, upload, list/read, retry, and withdrawal.

### Validation

- Backend: **115** pytest tests, compile validation, OpenAPI equality/version
  checks, Pylance diagnostics, and whitespace checks passed.
- Website: **95** tests, lint, production build, release checks, and whitespace
  checks passed.
- Client: **78** Rust tests, **14** frontend tests, Rust format/check, production
  frontend build, Tauri debug build, version checks, and whitespace checks passed.

### Deferred

- A permitted production scanner/provider, public publication/download, verified
  installation/activation, moderation/takedown operations, and live deployed
  website/backend/native end-to-end acceptance.

## 1.3.0 / Batch 2 release candidate

### Added

- Synchronized client **1.3.0**, website **0.1.0**, and backend/OpenAPI **0.2.0**
  release surfaces with version checks and release validation.
- Local Windows MSI/NSIS build validation and isolated website/backend smoke tests.

### Deferred

- Live GitHub OAuth, native packaged-client approval/polling, production
  origin/cookie/proxy verification, persistent storage operations, deployment,
  tagging, and publication.

## Batch 1 authentication and profile metadata foundation

### Added

- GitHub OAuth/session and single-use desktop approval/polling foundations.
- Owner-authorized private/public profile metadata operations shared by website and
  client, with Rust-memory desktop credentials and fail-closed scan gating.
- Synthetic local integration coverage for ownership, replay denial, scan refusal,
  logout, and HTTPS-only source validation.

### Deferred

- Live GitHub identity validation, native packaged-runtime acceptance, automatic
  launcher return/focus, artifact uploads, public publication/download, scanning,
  moderation, and Sponsor role synchronization.
