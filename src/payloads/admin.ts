export interface FeatureFlagRequest {
  key: string;
  description: string;
  enabled: boolean;
  requires_api_key?: boolean;
}

export interface ToggleRequest {
  enabled: boolean;
}

export interface FeatureFlagResponse {
  key: string;
  description: string;
  enabled: boolean;
  requires_api_key: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateApiKeyRequest {
  name: string;
  expires_in: string;
}

export interface ApiKeyResponse {
  id: string;
  name: string;
  prefix: string;
  status: string;
  created_at: string;
  expires_at?: string | null;
  last_used_at?: string | null;
}

export interface ApiKeyCreatedResponse {
  id: string;
  key: string;
  name: string;
  prefix: string;
  expires_at?: string | null;
}

export interface MaintenanceRequest {
  enabled: boolean;
  message?: string | null;
  until?: string | null;
  force_logout_at?: string | null;
  dismiss_seconds?: number | null;
  warn_minutes?: number | null;
}

export interface UpdateSettingsRequest {
  registration_mode?: string | null;
  max_users?: number | null;
  session_timeout_minutes?: number | null;
  allowed_email_domains?: string[] | null;
  maintenance_mode?: boolean | null;
  maintenance_message?: string | null;
  maintenance_until?: string | null;
}

export interface SettingsResponse {
  registration_mode: string;
  max_users: number;
  session_timeout_minutes: number;
  allowed_email_domains: string[];
  maintenance_mode: boolean;
  maintenance_message?: string | null;
  maintenance_until?: string | null;
  maintenance_force_logout_at?: string | null;
  maintenance_dismiss_seconds: number;
  maintenance_warn_minutes: number;
}

export interface MaintenanceStatusResponse {
  active: boolean;
  message?: string | null;
  force_logout_at?: string | null;
  until?: string | null;
  dismiss_seconds: number;
  warn_minutes: number;
}

export interface TerminateSessionsResponse {
  terminated_count: number;
}

export interface OverviewResponse {
  total_users: number;
  active_sessions: number;
  active_api_keys: number;
  feature_flags_enabled: number;
  feature_flags_total: number;
}
