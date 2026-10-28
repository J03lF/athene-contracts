import { FetchAthenosClient, type AthenosHttpClient } from './http';
import { AuthApiClient } from './clients/auth-client';
import { UserApiClient } from './clients/user-client';
import { WorkApiClient } from './clients/work-client';
import { DashboardApiClient } from './clients/dashboard-client';
import { ActivityApiClient } from './clients/activity-client';

export interface AthenosClientOptions {
  baseUrl: string;
  token?: string;
}

export class AthenosClient {
  readonly auth: AuthApiClient;
  readonly user: UserApiClient;
  readonly work: WorkApiClient;
  readonly dashboard: DashboardApiClient;
  readonly activity: ActivityApiClient;

  constructor(http: AthenosHttpClient) {
    this.auth = new AuthApiClient(http);
    this.user = new UserApiClient(http);
    this.work = new WorkApiClient(http);
    this.dashboard = new DashboardApiClient(http);
    this.activity = new ActivityApiClient(http);
  }

  static create(options: AthenosClientOptions): AthenosClient {
    const headers: Record<string, string> = {};
    if (options.token) {
      headers.Authorization = `Bearer ${options.token}`;
    }
    const http = new FetchAthenosClient(options.baseUrl, headers);
    return new AthenosClient(http);
  }
}
