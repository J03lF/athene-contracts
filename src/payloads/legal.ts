/**
 * The public site info (`GET /public/site`) and the person's consent to the
 * legal texts (`GET` / `POST /user/legal-consent`), W9.
 *
 * Generated from `rust/athenos-contracts/src/legal.rs` by ts-rs; the codes
 * and limits of the consent, the account, the quotas and the announcements
 * come from the Rust constants (`legal/legal-constants.ts`).
 */
export type { LegalInfo } from '../generated/legal/LegalInfo';
export type { PublicSiteInfo } from '../generated/legal/PublicSiteInfo';
export type { LegalConsentStatus } from '../generated/legal/LegalConsentStatus';
export type { AcceptLegalConsentRequest } from '../generated/legal/AcceptLegalConsentRequest';
export * from '../generated/legal/legal-constants';
