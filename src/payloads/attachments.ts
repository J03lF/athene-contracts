/**
 * Attachments of tickets and comments (W4-B), generated from
 * `rust/athene-contracts/src/attachments.rs`. Upload: the raw file as the
 * body of `POST /api/v1/work/tickets/{id}/attachments` (or
 * `…/comments/{comment_id}/attachments`), its name in
 * {@link ATTACHMENT_NAME_HEADER}; download: `GET …/attachments/{attachment_id}/content`.
 */
export type { AttachmentContentQuery } from '../generated/attachments/AttachmentContentQuery';
export type { AttachmentExpected } from '../generated/attachments/AttachmentExpected';
export type { AttachmentKind } from '../generated/attachments/AttachmentKind';
export type { AttachmentListResponse } from '../generated/attachments/AttachmentListResponse';
export type { AttachmentResponse } from '../generated/attachments/AttachmentResponse';
export type { AttachmentStatus } from '../generated/attachments/AttachmentStatus';
export type { ListAttachmentsQuery } from '../generated/attachments/ListAttachmentsQuery';
export type { RenameAttachmentRequest } from '../generated/attachments/RenameAttachmentRequest';
export { ATTACHMENT_NAME_HEADER, ATTACHMENT_TYPES } from '../generated/attachments/attachment-types';
