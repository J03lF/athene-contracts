export interface ApiResponse<T> {
  data: T;
}

export interface ListMeta {
  count: number;
}

export interface ListResponse<T> {
  data: T[];
  meta: ListMeta;
}

export interface ApiError {
  code: string;
  message: string;
  details?: unknown;
  request_id?: string;
  requestId?: string;
}

export interface ApiErrorEnvelope {
  error: ApiError;
}

export const API_ERROR_CODES = {
  gatewayRequestFailed: 'ATH-API-GW-001',
  validationFailed: 'ATH-API-VAL-001',
  invalidRequest: 'ATH-API-REQ-001',
  rateLimitExceeded: 'ATH-API-RATE-001',
  unauthorized: 'ATH-API-AUTH-001',
  forbidden: 'ATH-API-AUTH-002',
} as const;

export type ApiErrorCode = (typeof API_ERROR_CODES)[keyof typeof API_ERROR_CODES];
