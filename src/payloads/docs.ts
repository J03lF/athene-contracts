/**
 * Docs wire types: a workspace's markdown pages and the collections that
 * group them. Generated from `rust/athenos-contracts/src/docs.rs` by ts-rs;
 * the limits below mirror the constants there.
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

export type { CreateDocCollectionRequest } from '../generated/docs/CreateDocCollectionRequest';
export type { CreateDocRequest } from '../generated/docs/CreateDocRequest';
export type { DocCollectionResponse } from '../generated/docs/DocCollectionResponse';
export type { DocListMeta } from '../generated/docs/DocListMeta';
export type { DocListQuery } from '../generated/docs/DocListQuery';
export type { DocListResponse } from '../generated/docs/DocListResponse';
export type { DocResponse } from '../generated/docs/DocResponse';
export type { DocSort } from '../generated/docs/DocSort';
export type { DocSummaryResponse } from '../generated/docs/DocSummaryResponse';
export type { UpdateDocCollectionRequest } from '../generated/docs/UpdateDocCollectionRequest';
export type { UpdateDocRequest } from '../generated/docs/UpdateDocRequest';
export type { DocEdit } from '../generated/docs/DocEdit';
export type { DocExpected } from '../generated/docs/DocExpected';
