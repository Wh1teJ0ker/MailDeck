export interface AccountDto {
  id: number;
  email: string;
  display_name: string;
  color: string;
  provider: string;
  enabled: boolean;
  last_sync_at: number | null;
  last_error: string | null;
}

export interface ProviderDto {
  id: string;
  name: string;
  auth_mode: string;
  available: boolean;
  hint: string;
}

export interface AppSettingsDto {
  credential_store: string;
}

export interface CredentialSwitchResultDto {
  migrated: number;
  failed: string[];
}

export interface UpdateInfoDto {
  available: boolean;
  version: string;
  notes: string;
}

export interface MailboxDto {
  id: number;
  account_id: number;
  name_raw: string;
  name: string;
  role: "inbox" | "sent" | "drafts" | "trash" | "spam" | "other";
}

export interface MessageDto {
  id: number;
  mailbox_id: number;
  uid: number;
  subject: string | null;
  sender: string | null;
  sender_addr: string | null;
  recipient: string | null;
  date: number | null;
  seen: boolean;
  has_attachment: boolean;
}

export interface AttachmentDto {
  id: number;
  name: string;
  mime: string;
  size: number;
}

export interface BodyDto {
  subject: string | null;
  sender: string | null;
  sender_addr: string | null;
  recipient: string | null;
  date: number | null;
  seen: boolean;
  mailbox_name: string;
  mailbox_role: string;
  account_name: string;
  account_color: string;
  text: string | null;
  html: string | null;
  attachments: AttachmentDto[];
}

export interface SyncOutcomeDto {
  account_id: number;
  new_messages: number;
  error: string | null;
}

export interface MailboxUnreadDto {
  account_id: number;
  mailbox_id: number;
  role: string;
  unread: number;
}
