import type {
  AuthChallengeResponse,
  AuthSessionResponse,
  AuthUserResponse,
  FeatureFlagListResponse,
  LoginConfirmRequest,
  LoginRequest,
  LogoutResponse,
  PasswordConfirmResponse,
  PasswordResetConfirmRequest,
  PasswordResetRequest,
  PasswordResetResponse,
  RegisterConfirmRequest,
  RegisterRequest,
  RegistrationPolicyResponse,
  SessionListResponse,
} from '../payloads/auth';
import type { AtheneHttpClient } from '../http';

export class AuthApiClient {
  constructor(private readonly http: AtheneHttpClient) {}

  login(request: LoginRequest): Promise<AuthChallengeResponse> {
    return this.http.post('/auth/login/request', request);
  }

  confirmLogin(request: LoginConfirmRequest): Promise<AuthSessionResponse> {
    return this.http.post('/auth/login/confirm', request);
  }

  register(request: RegisterRequest): Promise<AuthChallengeResponse> {
    return this.http.post('/auth/register/request', request);
  }

  confirmRegister(request: RegisterConfirmRequest): Promise<AuthSessionResponse> {
    return this.http.post('/auth/register/confirm', request);
  }

  me(): Promise<AuthUserResponse> {
    return this.http.get('/auth/me');
  }

  registrationPolicy(): Promise<RegistrationPolicyResponse> {
    return this.http.get('/auth/registration');
  }

  logout(): Promise<LogoutResponse> {
    return this.http.post('/auth/logout', {});
  }

  passwordReset(request: PasswordResetRequest): Promise<PasswordResetResponse> {
    return this.http.post('/auth/password/reset', request);
  }

  confirmPasswordReset(request: PasswordResetConfirmRequest): Promise<PasswordConfirmResponse> {
    return this.http.post('/auth/password/confirm', request);
  }

  listSessions(): Promise<SessionListResponse> {
    return this.http.get('/auth/sessions');
  }

  revokeSession(sessionId: string): Promise<LogoutResponse> {
    return this.http.post(`/auth/sessions/${sessionId}/revoke`, {});
  }

  revokeAllSessions(): Promise<LogoutResponse> {
    return this.http.post('/auth/sessions/revoke-all', {});
  }

  featureFlags(): Promise<FeatureFlagListResponse> {
    return this.http.get('/auth/features');
  }
}
