import type { ApiResponse, ListResponse } from './common';
import type { FeatureFlagState } from '../generated/auth/FeatureFlagState';
import type { RegistrationMode } from '../generated/auth/RegistrationMode';

/**
 * Auth wire types. `RegistrationMode` and `FeatureFlagState` are generated
 * from `rust/athene-contracts/src/auth.rs` (ts-rs); the rest is still
 * hand-written and mirrors that file.
 */
export type { FeatureFlagState } from '../generated/auth/FeatureFlagState';
export type { RegistrationMode } from '../generated/auth/RegistrationMode';

export interface LoginRequest {
  email: string;
  password: string;
}

export interface LoginConfirmRequest {
  challenge_id: string;
  pin: string;
}

export interface RegisterRequest {
  email: string;
  display_name: string;
  password: string;
  api_key?: string;
}

export interface RegisterConfirmRequest {
  challenge_id: string;
  pin: string;
}

export interface PasswordResetRequest {
  email: string;
}

export interface PasswordResetConfirmRequest {
  token: string;
  new_password: string;
}

export interface AuthChallenge {
  challenge_id: string;
  expires_at: string;
}

export interface AuthUser {
  id: string;
  email: string;
  display_name: string;
  role: UserRole;
  status: UserStatus;
  last_login_at?: string | null;
}

export type UserRole = 'admin' | 'operator' | 'user' | 'guest';
export type UserStatus = 'active' | 'inactive' | 'locked' | 'pending_verification';

export interface AuthSession {
  token: string;
  expires_at: string;
  user: AuthUser;
}

export interface RegistrationPolicy {
  registration_mode: RegistrationMode;
  allowed_email_domains: string[];
  max_users: number;
}

export interface SessionInfo {
  id: string;
  ip_address?: string | null;
  user_agent?: string | null;
  created_at: string;
  last_activity: string;
  is_current: boolean;
}

/**
 * A verification code sent again (`POST /auth/challenge/{id}/resend`).
 * The previous code of the challenge stops working.
 */
export interface ChallengeResend {
  challenge_id: string;
  expires_at: string;
  /** Seconds until another resend is allowed; 0 when none are left. */
  retry_after_seconds: number;
  resends_remaining: number;
}

/**
 * `error.details` of a refused resend (HTTP 429).
 * `retry_after_seconds` is null once every resend is used.
 */
export interface ChallengeResendRefusal {
  reason: 'too_soon' | 'exhausted';
  retry_after_seconds: number | null;
  resends_remaining: number;
}

export type AuthChallengeResponse = ApiResponse<AuthChallenge>;
export type ChallengeResendResponse = ApiResponse<ChallengeResend>;
export type AuthSessionResponse = ApiResponse<AuthSession>;
export type AuthUserResponse = ApiResponse<AuthUser>;
export type RegistrationPolicyResponse = ApiResponse<RegistrationPolicy>;
export type SessionListResponse = ListResponse<SessionInfo>;
export type FeatureFlagListResponse = ListResponse<FeatureFlagState>;

export interface LogoutResponseData {
  revoked: boolean;
}

export type LogoutResponse = ApiResponse<LogoutResponseData>;

export interface PasswordResetResponseData {
  requested: boolean;
}

export type PasswordResetResponse = ApiResponse<PasswordResetResponseData>;

export interface PasswordConfirmResponseData {
  reset: boolean;
}

export type PasswordConfirmResponse = ApiResponse<PasswordConfirmResponseData>;
