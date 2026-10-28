/**
 * Watchers and notifications (W4-H): per person, event and channel a delivery
 * (immediate, daily, weekly, off), auto-watch rules, workspace defaults and the
 * digests. Routes: `/user/notifications`, `/user/notifications/digests`,
 * `/work/workspaces/{id}/notification-defaults`.
 *
 * Generated from `rust/athenos-contracts/src/notifications.rs` by ts-rs.
 */
export type { AutoWatchDefaultResponse } from '../generated/notifications/AutoWatchDefaultResponse';
export type { AutoWatchPatch } from '../generated/notifications/AutoWatchPatch';
export type { AutoWatchRule } from '../generated/notifications/AutoWatchRule';
export type { AutoWatchSettingResponse } from '../generated/notifications/AutoWatchSettingResponse';
export type { DefaultDelivery } from '../generated/notifications/DefaultDelivery';
export type { Delivery } from '../generated/notifications/Delivery';
export type { DigestCadence } from '../generated/notifications/DigestCadence';
export type { DigestEventResponse } from '../generated/notifications/DigestEventResponse';
export type { DigestListQuery } from '../generated/notifications/DigestListQuery';
export type { DigestListResponse } from '../generated/notifications/DigestListResponse';
export type { DigestResponse } from '../generated/notifications/DigestResponse';
export type { DigestSchedulePatch } from '../generated/notifications/DigestSchedulePatch';
export type { DigestScheduleResponse } from '../generated/notifications/DigestScheduleResponse';
export type { DigestTicketResponse } from '../generated/notifications/DigestTicketResponse';
export type { InboxDigest } from '../generated/notifications/InboxDigest';
export type { EffectiveDelivery } from '../generated/notifications/EffectiveDelivery';
export type { EventDefaultResponse } from '../generated/notifications/EventDefaultResponse';
export type { EventDeliveryPatch } from '../generated/notifications/EventDeliveryPatch';
export type { EventSettingResponse } from '../generated/notifications/EventSettingResponse';
export type { NotificationChannel } from '../generated/notifications/NotificationChannel';
export type { NotificationEvent } from '../generated/notifications/NotificationEvent';
export type { NotificationEventsPatch } from '../generated/notifications/NotificationEventsPatch';
export type { NotificationSettingsQuery } from '../generated/notifications/NotificationSettingsQuery';
export type { NotificationSettingsResponse } from '../generated/notifications/NotificationSettingsResponse';
export type { SettingSource } from '../generated/notifications/SettingSource';
export type { UpdateNotificationSettingsRequest } from '../generated/notifications/UpdateNotificationSettingsRequest';
export type { UpdateWorkspaceNotificationDefaultsRequest } from '../generated/notifications/UpdateWorkspaceNotificationDefaultsRequest';
export type { WorkspaceNotificationDefaultsResponse } from '../generated/notifications/WorkspaceNotificationDefaultsResponse';
export {
  AUTO_WATCH_RULES,
  DEFAULT_AUTO_WATCH,
  DEFAULT_DELIVERY,
  DIGEST_DEFAULT_TIME,
  DIGEST_DEFAULT_WEEKDAY,
  DIGEST_LIST_MAX,
  DUE_SOON_DEFAULT_HOURS,
  DUE_SOON_MAX_HOURS,
  DUE_SOON_MIN_HOURS,
  NOTIFICATION_EVENTS,
} from '../generated/notifications/notification-constants';
