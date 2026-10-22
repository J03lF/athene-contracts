/**
 * Docs wire shapes — a workspace's markdown pages and the collections that
 * group them. Mirrored by hand in `rust/athene-contracts/src/docs.rs`.
 *
 * Every response is wrapped as `{ data: … }`; the page list adds
 * `meta: { count, total }`. Ids are UUID strings, timestamps RFC 3339.
 *
 * Server rules: title 1..200 chars on one line; body at most 48 KB of UTF-8
 * (and at most 56 KB once JSON-escaped); the slug is generated from the
 * title at creation, unique per workspace (`-2`, `-3`, … on a clash) and
 * never changes on rename. Deleting a page archives it; deleting a
 * collection leaves its pages unfiled.
 */

/** The most a page body may hold, in UTF-8 bytes. */
export const DOC_MAX_BODY_BYTES = 48 * 1024;
export const DOC_MAX_TITLE_CHARS = 200;

export interface DocCollectionResponse {
  id: string;
  workspace_id: string;
  name: string;
  description: string | null;
  /** A short token or emoji the client renders; free text, ≤ 32 chars. */
  icon: string | null;
  position: number;
  /** Live (not archived) pages filed in this collection. */
  doc_count: number;
  created_by: string;
  created_at: string;
  updated_at: string;
}

export interface CreateDocCollectionRequest {
  /** 1..80 chars. */
  name: string;
  /** ≤ 500 chars. */
  description?: string | null;
  /** ≤ 32 chars. */
  icon?: string | null;
}

/** An absent key leaves that part as it is; `null` clears it. */
export interface UpdateDocCollectionRequest {
  name?: string;
  description?: string | null;
  icon?: string | null;
  position?: number;
}

/** A page in a list: everything but the body. */
export interface DocSummaryResponse {
  id: string;
  workspace_id: string;
  collection_id: string | null;
  project_id: string | null;
  title: string;
  slug: string;
  /** Plain-text preview of the body, ≤ 200 chars (markdown stripped). */
  excerpt: string;
  created_by: string;
  updated_by: string;
  /** `null` when the editor's user record is gone. */
  updated_by_name: string | null;
  created_at: string;
  updated_at: string;
}

export interface DocListMeta {
  /** Rows in this response. */
  count: number;
  /** Rows matching the filter, ignoring `limit`. */
  total: number;
}

export interface DocListResponse {
  data: DocSummaryResponse[];
  meta: DocListMeta;
}

/** A whole page. */
export interface DocResponse {
  id: string;
  workspace_id: string;
  collection_id: string | null;
  project_id: string | null;
  title: string;
  slug: string;
  /** Markdown source. Render it sanitised — it is user input. */
  body_md: string;
  created_by: string;
  created_by_name: string | null;
  updated_by: string;
  updated_by_name: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateDocRequest {
  /** 1..200 chars, one line. */
  title: string;
  /** Defaults to empty. ≤ 48 KB. */
  body_md?: string;
  collection_id?: string | null;
  project_id?: string | null;
}

/**
 * An absent key leaves that part as it is; `null` unfiles / unlinks.
 * The slug never changes.
 */
export interface UpdateDocRequest {
  title?: string;
  body_md?: string;
  collection_id?: string | null;
  project_id?: string | null;
}

/** Query of `GET /work/workspaces/:id/docs`. */
export interface DocListQuery {
  /** A collection id, or `'unfiled'`. Absent = every page. */
  collection?: string;
  /** Case-insensitive substring of title or body, ≤ 100 chars. */
  q?: string;
  /** `'updated'` (default, newest first) or `'title'`. */
  sort?: 'updated' | 'title';
  /** 1..200, default 50. */
  limit?: number;
}
