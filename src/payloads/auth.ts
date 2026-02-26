import { ApiResponse, ListResponse } from './common';

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

export type RegistrationMode = 'disabled' | 'api_key_required' | 'open';

export interface SessionInfo {
  id: string;
  ip_address?: string | null;
  user_agent?: string | null;
  created_at: string;
  last_activity: string;
  is_current: boolean;
}

export interface FeatureFlagState {
  key: string;
  enabled: boolean;
}

export type AuthChallengeResponse = ApiResponse<AuthChallenge>;
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
