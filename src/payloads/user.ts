/**
 * User wire types (`/user/settings`, `/user/profile`).
 *
 * Generated from the Rust contracts in `rust/athenos-contracts/src/user.rs`
 * by ts-rs; do not add hand-written wire shapes here. After changing a Rust
 * type, run `npm run generate` and commit `src/generated/`.
 *
 * A settings field that can be reset is `key?: T | null`: omit the key to
 * leave the value alone, send `null` to go back to the default (`Patch<T>`
 * in Rust).
 */
export type { DashboardWidgetLayout } from '../generated/user/DashboardWidgetLayout';
export type { Density } from '../generated/user/Density';
export type { InboxState } from '../generated/user/InboxState';
export type { SavedView } from '../generated/user/SavedView';
export type { Theme } from '../generated/user/Theme';
export type { UpdateProfileRequest } from '../generated/user/UpdateProfileRequest';
export type { UpdateUserSettingsRequest } from '../generated/user/UpdateUserSettingsRequest';
export type { UserProfileResponse } from '../generated/user/UserProfileResponse';
export type { UserSettingsResponse } from '../generated/user/UserSettingsResponse';

/**
 * The most the merged `preferences` object may hold, serialized
 * (`USER_PREFERENCES_MAX_BYTES` in `user.rs`).
 */
export const USER_PREFERENCES_MAX_BYTES = 16 * 1024;
