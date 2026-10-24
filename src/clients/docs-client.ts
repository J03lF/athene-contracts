import type {
  CreateDocCollectionRequest,
  CreateDocRequest,
  DocCollectionResponse,
  DocEdit,
  DocListQuery,
  DocListResponse,
  DocResponse,
  UpdateDocCollectionRequest,
  UpdateDocRequest,
} from '../payloads/docs';
import type { AtheneHttpClient } from '../http';

interface ApiEnvelope<T> {
  data: T;
}

function unwrap<T>(envelope: ApiEnvelope<T>): T {
  return envelope.data;
}

/**
 * Docs API surface — workspace-scoped collections and markdown pages.
 *
 * Every route is membership-checked: a workspace, collection or page the
 * caller cannot see answers 403, whether or not it exists. Deleting is the
 * author's or a workspace owner's/admin's; everything else is any member's.
 */
export class DocsApiClient {
  constructor(private readonly http: AtheneHttpClient) {}

  listCollections(workspaceId: string): Promise<DocCollectionResponse[]> {
    return this.http
      .get<ApiEnvelope<DocCollectionResponse[]>>(
        `/work/workspaces/${workspaceId}/doc-collections`,
      )
      .then(unwrap);
  }

  createCollection(
    workspaceId: string,
    body: CreateDocCollectionRequest,
  ): Promise<DocCollectionResponse> {
    return this.http
      .post<ApiEnvelope<DocCollectionResponse>>(
        `/work/workspaces/${workspaceId}/doc-collections`,
        body,
      )
      .then(unwrap);
  }

  updateCollection(
    id: string,
    body: UpdateDocCollectionRequest,
  ): Promise<DocCollectionResponse> {
    return this.http
      .patch<ApiEnvelope<DocCollectionResponse>>(`/work/doc-collections/${id}`, body)
      .then(unwrap);
  }

  /** Its pages stay, unfiled. */
  deleteCollection(id: string): Promise<void> {
    return this.http.delete(`/work/doc-collections/${id}`).then(() => undefined);
  }

  /** Full envelope: `meta.total` counts every match, ignoring `limit`. */
  list(workspaceId: string, query: DocListQuery = {}): Promise<DocListResponse> {
    const params = new URLSearchParams();
    if (query.collection) params.set('collection', query.collection);
    if (query.q) params.set('q', query.q);
    if (query.sort) params.set('sort', query.sort);
    if (query.limit !== undefined) params.set('limit', String(query.limit));
    const qs = params.toString();
    return this.http.get<DocListResponse>(
      `/work/workspaces/${workspaceId}/docs${qs ? `?${qs}` : ''}`,
    );
  }

  create(workspaceId: string, body: CreateDocRequest): Promise<DocResponse> {
    return this.http
      .post<ApiEnvelope<DocResponse>>(`/work/workspaces/${workspaceId}/docs`, body)
      .then(unwrap);
  }

  get(id: string): Promise<DocResponse> {
    return this.http.get<ApiEnvelope<DocResponse>>(`/work/docs/${id}`).then(unwrap);
  }

  /** 404 when no live page has this slug in the workspace. */
  getBySlug(workspaceId: string, slug: string): Promise<DocResponse> {
    return this.http
      .get<ApiEnvelope<DocResponse>>(
        `/work/workspaces/${workspaceId}/docs/by-slug/${encodeURIComponent(slug)}`,
      )
      .then(unwrap);
  }

  /** A `DocEdit` (patch + `expected`); the bare patch is accepted in stage A. */
  update(id: string, body: DocEdit | UpdateDocRequest): Promise<DocResponse> {
    return this.http
      .patch<ApiEnvelope<DocResponse>>(`/work/docs/${id}`, body)
      .then(unwrap);
  }

  /** Archives the page; its slug stays reserved. */
  delete(id: string): Promise<void> {
    return this.http.delete(`/work/docs/${id}`).then(() => undefined);
  }
}
