import { FetchAtheneClient, type AtheneHttpClient } from './http';
import { AuthApiClient } from './clients/auth-client';

export interface AtheneClientOptions {
  baseUrl: string;
  token?: string;
}

export class AtheneClient {
  readonly auth: AuthApiClient;

  constructor(http: AtheneHttpClient) {
    this.auth = new AuthApiClient(http);
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
