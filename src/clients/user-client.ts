import type {
  UpdateProfileRequest,
  UpdateUserSettingsRequest,
  UserProfileResponse,
  UserSettingsResponse,
} from '../payloads/user';
import type {
  AcceptLegalConsentRequest,
  LegalConsentStatus,
  PublicSiteInfo,
} from '../payloads/legal';
import type {
  AccountDeletionCheck,
  DeleteAccountRequest,
  DeleteAccountResponse,
} from '../payloads/account';
import type { AthenosHttpClient, RequestOptions } from '../http';

interface ApiEnvelope<T> {
  data: T;
}

function unwrap<T>(envelope: ApiEnvelope<T>): T {
  return envelope.data;
}

/** The signed-in person's settings row and profile (`/user/*`). */
export class UserApiClient {
  constructor(private readonly http: AthenosHttpClient) {}

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

  /** Which version of the legal texts the person accepted; `required` asks again. */
  getLegalConsent(init: RequestOptions = {}): Promise<LegalConsentStatus> {
    return this.http
      .get<ApiEnvelope<LegalConsentStatus>>('/user/legal-consent', { signal: init.signal })
      .then(unwrap);
  }

  /** Accepts the current version (another one is `400 ATH-LEGAL-001`). */
  acceptLegalConsent(
    body: AcceptLegalConsentRequest,
    init: RequestOptions = {},
  ): Promise<LegalConsentStatus> {
    return this.http
      .post<ApiEnvelope<LegalConsentStatus>>('/user/legal-consent', body, { signal: init.signal })
      .then(unwrap);
  }

  /** What deleting the account would do, and what blocks it. */
  accountDeletionCheck(init: RequestOptions = {}): Promise<AccountDeletionCheck> {
    return this.http
      .get<ApiEnvelope<AccountDeletionCheck>>('/user/account/deletion-check', {
        signal: init.signal,
      })
      .then(unwrap);
  }

  /**
   * Deletes the account. Wrong password: `401 ATH-ACCOUNT-003` (the session
   * stays, do not sign out); blocked: `409 ATH-ACCOUNT-001` / `-002`.
   * Afterwards every session of the person has ended.
   */
  deleteAccount(body: DeleteAccountRequest): Promise<DeleteAccountResponse> {
    return this.http
      .post<ApiEnvelope<DeleteAccountResponse>>('/user/account/delete', body)
      .then(unwrap);
  }
}

/** What the public pages read before anyone signs in (`/public/*`). */
export class PublicApiClient {
  constructor(private readonly http: AthenosHttpClient) {}

  /** The beta banner, the legal texts' version and blanks, and `setup_required`. */
  getSite(init: RequestOptions = {}): Promise<PublicSiteInfo> {
    return this.http
      .get<ApiEnvelope<PublicSiteInfo>>('/public/site', { signal: init.signal })
      .then(unwrap);
  }
}
