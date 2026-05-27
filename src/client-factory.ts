import { FetchAtheneClient, type AtheneHttpClient } from './http';
import { AuthApiClient } from './clients/auth-client';
import { WorkApiClient } from './clients/work-client';
import { DashboardApiClient } from './clients/dashboard-client';
import { ActivityApiClient } from './clients/activity-client';

export interface AtheneClientOptions {
  baseUrl: string;
  token?: string;
}

export class AtheneClient {
  readonly auth: AuthApiClient;
  readonly work: WorkApiClient;
  readonly dashboard: DashboardApiClient;
  readonly activity: ActivityApiClient;

  constructor(http: AtheneHttpClient) {
    this.auth = new AuthApiClient(http);
    this.work = new WorkApiClient(http);
    this.dashboard = new DashboardApiClient(http);
    this.activity = new ActivityApiClient(http);
  }

  static create(options: AtheneClientOptions): AtheneClient {
    const headers: Record<string, string> = {};
    if (options.token) {
      headers.Authorization = `Bearer ${options.token}`;
    }
    const http = new FetchAtheneClient(options.baseUrl, headers);
    return new AtheneClient(http);
  }
}
