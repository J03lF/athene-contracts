/**
 * Admin wire types (`/admin/*`, `/maintenance/status`).
 *
 * Generated from the Rust contracts in `rust/athenos-contracts/src/admin.rs`
 * by ts-rs; do not add hand-written shapes here. After changing a Rust type,
 * run `npm run generate` and commit `src/generated/`.
 *
 * `RegistrationMode` is generated too (`src/generated/auth/`) but is exported
 * from `./auth`.
 */
export type { FeatureFlagRequest } from '../generated/admin/FeatureFlagRequest';
export type { UpdateFeatureFlagRequest } from '../generated/admin/UpdateFeatureFlagRequest';
export type { ToggleRequest } from '../generated/admin/ToggleRequest';
export type { FeatureFlagResponse } from '../generated/admin/FeatureFlagResponse';
export type { CreateApiKeyRequest } from '../generated/admin/CreateApiKeyRequest';
export type { ApiKeyResponse } from '../generated/admin/ApiKeyResponse';
export type { ApiKeyCreatedResponse } from '../generated/admin/ApiKeyCreatedResponse';
export type { ApiKeyRevokedResponse } from '../generated/admin/ApiKeyRevokedResponse';
export type { MaintenanceRequest } from '../generated/admin/MaintenanceRequest';
export type { UpdateSettingsRequest } from '../generated/admin/UpdateSettingsRequest';
export type { SettingsResponse } from '../generated/admin/SettingsResponse';
export type { MaintenanceStatusResponse } from '../generated/admin/MaintenanceStatusResponse';
export type { TerminateSessionsResponse } from '../generated/admin/TerminateSessionsResponse';
export type { AuditEntryResponse } from '../generated/admin/AuditEntryResponse';
export type { CacheRefreshResponse } from '../generated/admin/CacheRefreshResponse';
export type { OverviewCounts } from '../generated/admin/OverviewCounts';
export type { OverviewResponse } from '../generated/admin/OverviewResponse';
