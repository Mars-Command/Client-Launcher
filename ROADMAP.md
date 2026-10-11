# Roadmap

Completed release work is recorded in [CHANGELOG.md](CHANGELOG.md). This roadmap
tracks active and remaining product work only.

Use the [MoSCoW verification checklist](VERIFICATION-CHECKLIST.md) for cross-batch
release and operational acceptance checks.

## Repositories

All project repositories live in the [Mars-Command](https://github.com/Mars-Command) organisation:

- [Client-Launcher](https://github.com/Mars-Command/Client-Launcher): this desktop launcher (Milestones 1, 3, 5 and client-side parts of 2 and 4).
- [Server-Backend](https://github.com/Mars-Command/Server-Backend): API for authentication, mod storage, profiles, submissions, moderation, and roles (Milestones 2, 3 and 4).
- [Website](https://github.com/Mars-Command/Website): website login, mod/profile management, community and donation pages (Milestones 2, 3 and 4).

## Milestone 1: Personal client mods

- [x] Add a mod selection/upload flow for a user's isolated Mars installation.
- [x] Validate supported file type, size, duplicate/conflicting files, and Minecraft/NeoForge compatibility before install; show clear errors and compatibility warnings.
- [x] Install and remove user mods in the isolated instance without changing the signed base pack or shared `.minecraft/mods` directory.
- [x] Preserve user-installed mods during normal signed pack updates, and make their status visible in integrity results.

## Milestone 2: Remote mod library

- [ ] Implement website-based account login and a secure desktop authentication handoff through the backend.
  - [x] Implement GitHub OAuth/session endpoints and expiring, single-use desktop approval/polling with separate community credentials.
  - [x] Implement website identity confirmation, account switching controls, and terminal-style success/failure output.
  - [x] Validate website/backend handoff locally with a synthetic identity; verify replay rejection and desktop bearer access.
  - [ ] Verify live GitHub OAuth, production cookie/redirect configuration, and the native packaged launcher.
  - [ ] Automatically return/focus the open launcher after website approval.
- [ ] Add authenticated mod submission to remote storage with metadata for author, version, game/loader compatibility, source, and license.
  - [x] Reserve owner-scoped immutable release metadata and upload a bounded raw JAR into private quarantine.
  - [x] Provide submission status, retry, refresh, and withdrawal controls in the website and desktop client.
  - [ ] Enable a permitted production scanner and public publication after operational acceptance.
- [x] Represent each uploaded mod version as an immutable release capsule: bind its exact artifact checksum to immutable release metadata and references to versioned provenance, compatibility, and scan attestations; keep release identity and ownership separate if identical bytes are deduplicated.
- [x] Store uploaded files in a private server-managed bucket; isolate unvalidated files from downloadable files.
- [x] Add durable queued upload processing with bounded private storage, expiry, and visible pending/failure states.
- [x] Validate submitted bytes before publication eligibility; retain backend-observed checksums and version history for future verified downloads.
- [ ] Require a source mod-page URL and a completed, acceptable backend virus scan for every published mod.
- [ ] Add owner-authorized CRUD endpoints shared by the website and client for mods and mod profiles.
  - [x] Implement and test owner-authorized profile metadata CRUD through the shared API and both interfaces.
  - [x] Implement owner-authorized capsule reservation, upload, status, retry, and withdrawal APIs with immutable artifact metadata.
- [ ] Keep profiles private unless explicitly submitted for publication; publish only profiles whose referenced mods pass validation.
  - [x] Keep newly created/copied profiles private and make edits withdraw public visibility.
  - [x] Reject publication when scanning is unavailable, including empty profiles; do not treat user-provided hashes as scan results.
  - [ ] Enable validated publication once artifact storage and scanning are implemented.
- [ ] Publish validated submissions automatically and provide report and takedown controls.
- [ ] Persist report/takedown incident metadata and deliver create/update/delete events to Discord with retryable webhook delivery.
- [x] Keep community files separate from the signed base-pack manifest and provide clear upload limits and failure messages.

### Capsule publication pipeline

- [x] Define the capsule schema, immutable release identity, and owner-scoped references to deduplicated artifacts.
- [x] Define revisioned capsule states and allowed transitions, including quarantine, scanning, acceptance eligibility, rejection, expiry, and withdrawal.
- [x] Make reservation, upload replay, scan retry, and withdrawal operations idempotent.
- [x] Attach versioned scan evidence to the exact artifact digest; preserve provider, result, verdict, summary, timestamps, and policy version used for each eligibility decision.
- [x] Reject stale worker leases, scan evidence, revisions, and out-of-order transitions; recover abandoned work without changing capsule identity.
- [ ] Publish through one atomic state transition only after the digest, ownership, metadata, current policy, and required attestations pass.
- [ ] Define rescan, evidence-expiry, withdrawal, takedown, and republishing behavior without rewriting historical decisions.

## Milestone 3: Community tab

- [x] Add an account icon to the client title bar that initiates the website login flow.
- [ ] Add a community tab to browse and search published client mods, with compatibility and provenance details.
- [ ] Browse and download public mod profiles, with an option to edit a downloaded profile as a separate personal profile without changing its public source.
  - [x] Implement public profile metadata search and independent private metadata copies in both interfaces; test using seeded public fixtures.
  - [ ] Implement verified mod-file download/install from a public profile.
    - [ ] Download a specific published release capsule into bounded isolated staging; verify the backend-published SHA-256 and compatibility before making it launchable.
    - [ ] Activate a fully verified capsule atomically and retain the prior verified version for explicit rollback; never modify the signed base pack or shared `.minecraft/mods`.
- [ ] Allow users to add more client mods to personal profiles and optionally submit their own profile for publication.
- [ ] Let users install and remove community mods into their own isolated Mars installation.
- [ ] Verify downloaded files against their published checksums and warn or block when compatibility or integrity checks fail.
- [ ] Show installed mod versions and make updates/removals explicit and reversible.
- [x] Add private owner upload and scan-queue status controls without exposing a public artifact route.
- [ ] Add reporting controls and applicable takedown notifications.

### Verified client activation

- [ ] Define launcher-owned staging, immutable verified-generation, and activation-receipt layouts.
- [ ] Verify the capsule digest, metadata, and defined compatibility requirements before activation.
- [ ] Activate a complete generation atomically without modifying the signed base pack or shared `.minecraft/mods`.
- [ ] Reconcile interrupted staging, activation, and rollback when the launcher starts.
- [ ] Define a bounded post-activation health signal and do not mark a generation current until it passes.
- [ ] Retain and explicitly expose the previous verified generation for bounded, user-controlled rollback.
- [ ] Define client behavior when an installed capsule is later withdrawn, expires, or is taken down.

## Milestone 4: Donation-based server role

- [x] Add a community-tab donation button linking to website login or the configured GitHub Sponsors page.
  - [ ] Configure and verify the actual sponsorship recipient in deployment.
- [ ] Link the user's Mars account to a verified GitHub identity and verify that account's sponsorship without downloading the entire sponsor list.
- [ ] Grant and revoke the Sponsor role from backend-verified sponsorship status; keep authorization server-side.
- [ ] Handle delayed, refunded, or revoked donations without granting roles from client-reported status.
- [ ] Define Sponsor benefits after the core features are implemented successfully.

## Milestone 5: Client update completion

- [x] Complete the Update Client flow around the existing stable-release check: clearly present the available installer, require the user's confirmation, and explain how to finish installation.
- [x] Document supported release assets and recovery behavior when a release or download is unavailable.

## Agreed product requirements (2026-10-07)

### Accounts and profiles

- Production origins are `https://mars.nexusgit.info` (website) and `https://api.nexusgit.info` (backend); deployment and live OAuth verification remain pending.
- Clicking the client account icon starts an authentication request with the backend and opens the website at `/auth/login`.
- If a website session already exists, show the user's profile picture with "Continue as" and "Log in as a different account" actions.
- Show terminal-style success or failure output on the website and return the user to the open application.
- Both the website and client use the same backend endpoints for owner-authorized CRUD operations.
- Submitting a profile makes it public; users may choose not to submit. Editing a downloaded profile creates a personal profile, not an edit to another author's published profile.
- Implementation must bind the website approval to the initiating client using expiring, single-use authorization and replay protection; never put reusable access tokens in redirect URLs.

### Upload storage and scanning

- Batch 3 uses a private server filesystem bucket with SQLite queueing on persistent local disk. The quarantine, retained-artifact, and evidence paths are absolute, startup-validated, private, and must not overlap static, package, or public-release directories. This is intentionally a single-host design; unsupported shared or network filesystems are excluded.
- A future edge/object-storage implementation must preserve the private-quarantine, durable-queue, digest-binding, expiry, and publication-policy guarantees; ordinary CDN caching is not upload staging.
- Bucket placement alone is not an upload security boundary: require scoped upload authorization, private quarantine, size/type/archive limits, and controlled access to published files.
- Every published mod requires a mod-page URL and an acceptable completed virus scan performed by the backend. A completed scan alone is not proof that a file is safe.
- The default production scanner is disabled and fails closed: it records private `scan_blocked` evidence with `scanning_not_configured`, never acceptable evidence. No scanner credential is exposed to the website or client.
- VirusTotal integration is deferred until usage rights, file-submission disclosure, account limits, and a permitted plan are confirmed. If selected, configure its URL/key only in the backend environment.
- Budget all VirusTotal requests, including uploads, hash lookups, and analysis polling. Deduplicate by content checksum, apply a defined scan-freshness policy, and enforce shared quota limits across workers.
- The public API documents 4 requests/minute and 500 requests/day. The proposed 15,500 requests/month cap remains an additional planning limit to verify against the actual account.
- Quota exhaustion or scanner failure leaves files pending and unavailable for publication; show the reason rather than bypassing validation.
- Before adopting VirusTotal, confirm permitted use and file-submission rights/disclosure. Its public API prohibits commercial products/services and business workflows that do not contribute new files. Select a permitted plan or alternative provider if needed.
- Reference: [VirusTotal API restrictions](https://docs.virustotal.com/reference/public-vs-premium-api), [file submission](https://docs.virustotal.com/reference/files-scan), [analysis retrieval](https://docs.virustotal.com/reference/analysis), and [analysis object](https://docs.virustotal.com/reference/analyses-object).

### Community release capsules (agreed 2026-10-08)

- Treat each published mod version as an immutable capsule binding its exact artifact checksum to immutable release metadata. Store provenance, compatibility, and scan results as versioned attestations associated with that digest rather than rewriting the capsule.
- A capsule must not become downloadable or installable until every publication requirement passes; scanner, storage, or quota failures leave it pending and unavailable.
- Artifact deduplication may share bytes or scan work, but must not merge authorship, release history, permissions, or source-page requirements.
- Publication decisions record the exact policy version and attestations used. Later rescans or policy changes add new evidence and may withdraw availability without erasing the historical decision.
- The client stages a selected capsule separately from the signed base pack, verifies its published checksum and defined compatibility requirements, and exposes it to launches only after an all-or-nothing activation and bounded health check. Keep prior verified versions available for user-controlled rollback.
- Client activation receipts record the capsule digest, verified generation, activation outcome, and rollback predecessor so interrupted operations can be reconciled deterministically.
- The capsule's backend record is the trusted reference for which bytes and evidence belong to a release. A separate cryptographic signing format/key-distribution design is not selected yet and must not be implied by checksum verification alone.

### Reports and takedowns

- Store incident metadata in a backend incident folder and mirror events to a private moderation Discord channel through a backend-only webhook.
- Include the reason, affected mod(s)/profile, report count, and upload author's username. Reporting is anonymous by default to others; retain the reporter's identity privately for follow-up.
- Requested incident filename identifiers combine the first three username characters, a mod-name component, `rep` for reports or `inc` for takedowns, and random alphanumeric characters, with a maximum identifier length of 16 characters. Confirm component allocation and whether the extension counts toward this limit before implementation; sanitize components and handle collisions.
- Report and takedown metadata must generate webhook events when created, updated, or deleted. Retain an event payload for deletion delivery and retry failures without losing the incident operation.
- Takedowns remove the affected mod(s) or profile completely from server storage while retaining incident metadata. Account for staged files, replicas, CDN caches, and shared-file references in the deletion design.
- Notify involved users of a takedown when the report reason is strictly longer than 50 characters. Confirm the recipients and handling of multiple reports before implementation.
- Discord is a notification/moderation surface, not the sole durable incident record; keep reporter identity out of public output and restrict metadata access.

### Sponsorship

- Use GitHub Sponsors, with the client reading its Sponsor role from the backend.
- Do not match arbitrary usernames against email addresses: verify ownership of a linked GitHub account and use authenticated, account-specific sponsorship checks supported by GitHub's API.
- Keep GitHub credentials and role assignment on the backend; synchronize cancellations/revocations with verified events and reconciliation.
- Confirm the sponsorship recipient, GitHub authentication permissions, and eligibility rules for recurring versus one-time sponsorships before implementation. Sponsor benefits are deferred.
- Reference: [GitHub Sponsors GraphQL API](https://docs.github.com/en/sponsors/integrating-with-github-sponsors/getting-started-with-the-sponsors-graphql-api).

## Remaining implementation decisions

- GitHub OAuth is selected; the automatic desktop return/focus mechanism still needs a decision.
- Future edge/object-storage provider, if the single-host filesystem bucket is replaced.
- Permitted scanning provider/plan, accepted verdict and freshness policies, and verified account quotas.
- Whether release records need an additional cryptographic signature beyond authenticated backend delivery and checksum verification.
- Exact compatibility admission criteria: metadata agreement, dependency resolution, archive inspection, bounded test launch, or a defined combination.
- Client behavior for already-installed capsules that are later withdrawn, expire, or are taken down, including offline launch and rollback rules.
- Moderation permissions, incident naming allocation, notification recipients, and complete deletion semantics.
- GitHub sponsorship recipient, verified account linking, and sponsorship eligibility.

## Existing foundations

The launcher already verifies and syncs the signed base pack into an isolated game directory, checks stable GitHub releases for client installers, and opens the selected installer for the user. Community mods must remain user-controlled and separate from that trusted base pack.

## Completed release history

See [CHANGELOG.md](CHANGELOG.md) for accepted Batch 1 authentication/profile
foundations, Batch 2 release-candidate hardening, and the validated Batch 3
private capsule-ingestion foundation. These releases did not complete the remaining
Milestone 2, 3, or 4 outcomes.
