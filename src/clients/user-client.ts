import type {
  UpdateProfileRequest,
  UpdateUserSettingsRequest,
  UserProfileResponse,
  UserSettingsResponse,
} from '../payloads/user';
import type { AtheneHttpClient, RequestOptions } from '../http';

interface ApiEnvelope<T> {
  data: T;
}

function unwrap<T>(envelope: ApiEnvelope<T>): T {
  return envelope.data;
}

/** The signed-in person's settings row and profile (`/user/*`). */
export class UserApiClient {
  constructor(private readonly http: AtheneHttpClient) {}

  getSettings(init: RequestOptions = {}): Promise<UserSettingsResponse> {
    return this.http
      .get<ApiEnvelope<UserSettingsResponse>>('/user/settings', { signal: init.signal })
      .then(unwrap);
  }

  /**
   * Only the keys present are applied; `null` puts a setting back to its
   * default. Answers the whole row as stored.
   */
  updateSettings(
    body: UpdateUserSettingsRequest,
    init: RequestOptions = {},
  ): Promise<UserSettingsResponse> {
    return this.http
      .put<ApiEnvelope<UserSettingsResponse>>('/user/settings', body, { signal: init.signal })
      .then(unwrap);
  }

  getProfile(init: RequestOptions = {}): Promise<UserProfileResponse> {
    return this.http
      .get<ApiEnvelope<UserProfileResponse>>('/user/profile', { signal: init.signal })
      .then(unwrap);
  }

  updateProfile(
    body: UpdateProfileRequest,
    init: RequestOptions = {},
  ): Promise<UserProfileResponse> {
    return this.http
      .put<ApiEnvelope<UserProfileResponse>>('/user/profile', body, { signal: init.signal })
      .then(unwrap);
  }
}
