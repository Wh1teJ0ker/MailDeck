import { beforeEach, describe, expect, it, vi } from "vitest";

// mock Tauri invoke,断言 api 层的命令名与参数映射
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

import { invoke } from "@tauri-apps/api/core";
import { api } from "../src/api";

const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
  mockInvoke.mockReset();
  mockInvoke.mockResolvedValue(undefined);
});

describe("api 层命令映射", () => {
  it("list_messages 传递过滤参数,null 表示全部收件箱视图", async () => {
    await api.listMessages({ mailboxId: null, unreadOnly: true, query: "会议", offset: 0, limit: 100 });
    expect(mockInvoke).toHaveBeenCalledWith("list_messages", {
      accountId: null,
      mailboxId: null,
      unreadOnly: true,
      query: "会议",
      offset: 0,
      limit: 100,
    });
  });

  it("list_messages 支持指定邮箱视图", async () => {
    await api.listMessages({ mailboxId: 12, unreadOnly: false, query: "", offset: 20, limit: 50 });
    expect(mockInvoke).toHaveBeenCalledWith("list_messages", {
      accountId: null,
      mailboxId: 12,
      unreadOnly: false,
      query: "",
      offset: 20,
      limit: 50,
    });
  });

  it("list_mailboxes 按账号拉取文件夹", async () => {
    await api.listMailboxes(3);
    expect(mockInvoke).toHaveBeenCalledWith("list_mailboxes", { accountId: 3 });
  });

  it("mark_all_seen 按当前视图一键已读", async () => {
    await api.markAllSeen(null);
    expect(mockInvoke).toHaveBeenCalledWith("mark_all_seen", { mailboxId: null });

    await api.markAllSeen(12);
    expect(mockInvoke).toHaveBeenCalledWith("mark_all_seen", { mailboxId: 12 });
  });

  it("add_account 以 camelCase 传递表单字段", async () => {
    await api.addAccount({
      email: "a@qq.com",
      authCode: "code",
      displayName: "工作",
      color: "#1677ff",
    });
    expect(mockInvoke).toHaveBeenCalledWith("add_account", {
      email: "a@qq.com",
      authCode: "code",
      displayName: "工作",
      color: "#1677ff",
    });
  });

  it("open_message / set_seen / unread_counts 传参正确", async () => {
    await api.openMessage(9);
    expect(mockInvoke).toHaveBeenCalledWith("open_message", { id: 9 });

    await api.setSeen(9, false);
    expect(mockInvoke).toHaveBeenCalledWith("set_seen", { id: 9, seen: false });

    await api.unreadCounts();
    expect(mockInvoke).toHaveBeenCalledWith("unread_counts");
  });

  it("账号管理命令传参正确", async () => {
    await api.syncAccount(1);
    expect(mockInvoke).toHaveBeenCalledWith("sync_account", { id: 1 });

    await api.syncAll();
    expect(mockInvoke).toHaveBeenCalledWith("sync_all");

    await api.removeAccount(2);
    expect(mockInvoke).toHaveBeenCalledWith("remove_account", { id: 2 });

    await api.setAccountEnabled(2, false);
    expect(mockInvoke).toHaveBeenCalledWith("set_account_enabled", { id: 2, enabled: false });

    await api.listAccounts();
    expect(mockInvoke).toHaveBeenCalledWith("list_accounts");
  });

  it("按类型添加账号时传递 providerType", async () => {
    await api.addAccount({
      providerType: "qq",
      email: "a@qq.com",
      authCode: "code",
    });
    expect(mockInvoke).toHaveBeenCalledWith("add_account", {
      providerType: "qq",
      email: "a@qq.com",
      authCode: "code",
      displayName: null,
      color: null,
    });
  });

  it("编辑账号只传显示名与颜色", async () => {
    await api.updateAccount({ id: 5, displayName: "新名", color: "#ff5500" });
    expect(mockInvoke).toHaveBeenCalledWith("update_account", {
      id: 5,
      displayName: "新名",
      color: "#ff5500",
    });
  });

  it("服务商类型与更新相关命令映射正确", async () => {
    await api.listProviders();
    expect(mockInvoke).toHaveBeenCalledWith("list_providers");

    await api.appVersion();
    expect(mockInvoke).toHaveBeenCalledWith("app_version");

    await api.checkUpdate();
    expect(mockInvoke).toHaveBeenCalledWith("check_update");

    await api.installUpdate();
    expect(mockInvoke).toHaveBeenCalledWith("install_update");

    await api.relaunch();
    expect(mockInvoke).toHaveBeenCalledWith("relaunch");
  });
});
