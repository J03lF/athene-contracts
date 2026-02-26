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
  request_id?: string;
}
