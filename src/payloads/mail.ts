/**
 * Mail sending in the admin console (W6-E): `GET /admin/mail` (whether mail
 * can go out, how it went last) and `POST /admin/mail/test` (a test mail to
 * the signed-in admin).
 *
 * Generated from `rust/athene-contracts/src/mail.rs` by ts-rs.
 */
export type { MailLanguage } from '../generated/mail/MailLanguage';
export type { MailLastError } from '../generated/mail/MailLastError';
export type { MailNotificationsStatus } from '../generated/mail/MailNotificationsStatus';
export type { MailPasswordSource } from '../generated/mail/MailPasswordSource';
export type { MailQueueStatus } from '../generated/mail/MailQueueStatus';
export type { MailRecipientStatus } from '../generated/mail/MailRecipientStatus';
export type { MailSecurity } from '../generated/mail/MailSecurity';
export type { MailServiceStatus } from '../generated/mail/MailServiceStatus';
export type { MailSkipReason } from '../generated/mail/MailSkipReason';
export type { MailStatusResponse } from '../generated/mail/MailStatusResponse';
export type { MailTransportStatus } from '../generated/mail/MailTransportStatus';
export type { SendTestMailRequest } from '../generated/mail/SendTestMailRequest';
export type { TestMailOutcome } from '../generated/mail/TestMailOutcome';
export type { TestMailResponse } from '../generated/mail/TestMailResponse';
