import type { ApiErrorCode } from '../generated/common/ApiErrorCode';
import type { Assert, Covers } from '../internal/covers';

export type { ApiErrorCode } from '../generated/common/ApiErrorCode';

// The envelopes are generic and stay hand-written: ts-rs would export one
// copy per instantiation.
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

/** The gateway's own error codes by name (`ApiErrorCode` in `common.rs`). */
export const API_ERROR_CODES = {
  gatewayRequestFailed: 'ATH-API-GW-001',
  validationFailed: 'ATH-API-VAL-001',
  invalidRequest: 'ATH-API-REQ-001',
  rateLimitExceeded: 'ATH-API-RATE-001',
  unauthorized: 'ATH-API-AUTH-001',
  forbidden: 'ATH-API-AUTH-002',
} as const satisfies Record<string, ApiErrorCode>;

// Compile-time: the names above cover every generated code.
type _Complete = Assert<
  Covers<ApiErrorCode, Array<(typeof API_ERROR_CODES)[keyof typeof API_ERROR_CODES]>>
>;
