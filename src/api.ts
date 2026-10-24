import { invoke } from "@tauri-apps/api/core";
import type {
  AccountDto,
  BodyDto,
  MailboxDto,
  MailboxUnreadDto,
  MessageDto,
  ProviderDto,
  SyncOutcomeDto,
  UpdateInfoDto,
} from "./types";

export const api = {
  listAccounts: () => invoke<AccountDto[]>("list_accounts"),

  listProviders: () => invoke<ProviderDto[]>("list_providers"),

  listMailboxes: (accountId: number) =>
    invoke<MailboxDto[]>("list_mailboxes", { accountId }),

  addAccount: (p: {
    providerType: string;
    email: string;
    authCode: string;
    displayName?: string;
    color?: string;
  }) =>
    invoke<AccountDto>("add_account", {
      providerType: p.providerType,
      email: p.email,
      authCode: p.authCode,
      displayName: p.displayName ?? null,
      color: p.color ?? null,
    }),

  removeAccount: (id: number) => invoke<void>("remove_account", { id }),

  setAccountEnabled: (id: number, enabled: boolean) =>
    invoke<void>("set_account_enabled", { id, enabled }),

  updateAccount: (p: { id: number; displayName: string; color: string }) =>
    invoke<AccountDto>("update_account", {
      id: p.id,
      displayName: p.displayName,
      color: p.color,
    }),

  syncAccount: (id: number) => invoke<SyncOutcomeDto>("sync_account", { id }),

  syncAll: () => invoke<SyncOutcomeDto[]>("sync_all"),

  listMessages: (p: {
    mailboxId: number | null;
    unreadOnly: boolean;
    query: string;
    offset: number;
    limit: number;
  }) =>
    invoke<MessageDto[]>("list_messages", {
      accountId: null,
      mailboxId: p.mailboxId,
      unreadOnly: p.unreadOnly,
      query: p.query,
      offset: p.offset,
      limit: p.limit,
    }),

  openMessage: (id: number) => invoke<BodyDto>("open_message", { id }),

  setSeen: (id: number, seen: boolean) => invoke<void>("set_seen", { id, seen }),

  markAllSeen: (mailboxId: number | null) =>
    invoke<number>("mark_all_seen", { mailboxId }),

  unreadCounts: () => invoke<MailboxUnreadDto[]>("unread_counts"),

  // 设置:检查更新与在线更新
  appVersion: () => invoke<string>("app_version"),

  checkUpdate: () => invoke<UpdateInfoDto>("check_update"),

  installUpdate: () => invoke<void>("install_update"),

  relaunch: () => invoke<void>("relaunch"),
};
