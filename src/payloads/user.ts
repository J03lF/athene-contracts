export interface UserSettingsResponse {
  theme: string;
  language: string;
  shortcuts: unknown;
  notifications_enabled: boolean;
  email_notifications: boolean;
}

export interface UpdateUserSettingsRequest {
  theme?: string | null;
  language?: string | null;
  shortcuts?: unknown;
  notifications_enabled?: boolean | null;
  email_notifications?: boolean | null;
}

export interface UserProfileResponse {
  id: string;
  email: string;
  display_name: string;
  role: string;
  avatar_url?: string | null;
  last_login_at?: string | null;
  created_at: string;
}

export interface UpdateProfileRequest {
  display_name?: string | null;
}
