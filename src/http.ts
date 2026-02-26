export interface AtheneHttpClient {
  get<T>(path: string, config?: RequestInit): Promise<T>;
  post<T>(path: string, body?: unknown, config?: RequestInit): Promise<T>;
}

export class FetchAtheneClient implements AtheneHttpClient {
  constructor(private readonly baseUrl: string, private readonly defaultHeaders: HeadersInit = {}) {}

  async get<T>(path: string, config: RequestInit = {}): Promise<T> {
    return this.request<T>('GET', path, undefined, config);
  }

  async post<T>(path: string, body?: unknown, config: RequestInit = {}): Promise<T> {
    return this.request<T>('POST', path, body, config);
  }

  private async request<T>(method: string, path: string, body?: unknown, config: RequestInit = {}): Promise<T> {
    const headers = new Headers({ 'content-type': 'application/json', ...this.defaultHeaders, ...config.headers });
    const response = await fetch(`${this.baseUrl}${path}`, {
      method,
      headers,
      body: body !== undefined ? JSON.stringify(body) : undefined,
      ...config,
    });
    if (!response.ok) {
      throw await AtheneHttpError.fromResponse(response);
    }
    if (response.status === 204) {
      return undefined as T;
    }
    return (await response.json()) as T;
  }
}

export class AtheneHttpError extends Error {
  constructor(readonly status: number, message: string, readonly details?: unknown) {
    super(message);
  }

  static async fromResponse(response: Response): Promise<AtheneHttpError> {
    try {
      const payload = await response.json();
      const message = payload?.error?.message || payload?.message || response.statusText;
      return new AtheneHttpError(response.status, message, payload);
    } catch {
      return new AtheneHttpError(response.status, response.statusText);
    }
  }
}
