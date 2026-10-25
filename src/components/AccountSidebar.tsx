import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { App as AntApp, Avatar, Badge, Button, Dropdown, Typography } from "antd";
import {
  InboxOutlined,
  MailOutlined,
  PlusOutlined,
  MoreOutlined,
  ReloadOutlined,
  PauseCircleOutlined,
  PlayCircleOutlined,
  EditOutlined,
  DeleteOutlined,
  SendOutlined,
  FileTextOutlined,
  EyeInvisibleOutlined,
  FolderOutlined,
  DownOutlined,
  RightOutlined,
  LoadingOutlined,
} from "@ant-design/icons";
import { api } from "../api";
import { useT } from "../i18n";
import type { AccountDto, MailboxDto } from "../types";
import { useUiStore } from "../store";
import { formatTime } from "../utils/format";
import { GRADIENT, PRIMARY, GLOW } from "../theme";
import AddAccountModal from "./AddAccountModal";
import EditAccountModal from "./EditAccountModal";

const ROLE_ORDER = ["inbox", "sent", "drafts", "trash", "spam"] as const;
const ROLE_KEY: Record<string, string> = {
  inbox: "folder.inbox",
  sent: "folder.sent",
  drafts: "folder.drafts",
  trash: "folder.trash",
  spam: "folder.spam",
};

const FOLDER_ICONS: Record<string, React.ReactNode> = {
  inbox: <InboxOutlined style={{ fontSize: 14 }} />,
  sent: <SendOutlined style={{ fontSize: 14 }} />,
  drafts: <FileTextOutlined style={{ fontSize: 14 }} />,
  trash: <DeleteOutlined style={{ fontSize: 14 }} />,
  spam: <EyeInvisibleOutlined style={{ fontSize: 14 }} />,
  other: <FolderOutlined style={{ fontSize: 14 }} />,
};

/** 侧栏文件夹条目排序:五大角色在前按序,其余(其他文件夹)按名字跟在后面 */
function sortFolders(boxes: MailboxDto[]): MailboxDto[] {
  const rank = (r: string) => {
    const i = ROLE_ORDER.indexOf(r as (typeof ROLE_ORDER)[number]);
    return i === -1 ? ROLE_ORDER.length : i;
  };
  return [...boxes].sort((a, b) => rank(a.role) - rank(b.role) || a.name.localeCompare(b.name));
}

export default function AccountSidebar() {
  const qc = useQueryClient();
  const { message, modal } = AntApp.useApp();
  const t = useT();
  const [addOpen, setAddOpen] = useState(false);
  const [editing, setEditing] = useState<AccountDto | null>(null);
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const { mailboxId, unreadOnly, setMailbox, setUnreadOnly } = useUiStore();

  const { data: accounts = [] } = useQuery({ queryKey: ["accounts"], queryFn: api.listAccounts });
  // 版本号单一事实源:Cargo.toml → 后端 app_version,界面不做硬编码
  const { data: appVersion = "" } = useQuery({
    queryKey: ["app-version"],
    queryFn: api.appVersion,
    staleTime: Infinity,
  });
  const { data: mailboxesByAccount } = useQuery({
    queryKey: ["mailboxes", accounts.map((a) => a.id).join(",")],
    queryFn: async () => {
      const entries = await Promise.all(
        accounts.map(async (a) => [a.id, await api.listMailboxes(a.id)] as const)
      );
      return Object.fromEntries(entries) as Record<number, MailboxDto[]>;
    },
    enabled: accounts.length > 0,
  });
  const { data: unread = [] } = useQuery({ queryKey: ["unread"], queryFn: api.unreadCounts });

  const unreadByMailbox = useMemo(
    () => new Map(unread.map((u) => [u.mailbox_id, u.unread])),
    [unread]
  );
  const totalUnread = useMemo(
    () => unread.filter((u) => u.role === "inbox").reduce((s, u) => s + u.unread, 0),
    [unread]
  );

  const invalidateAll = () => {
    qc.invalidateQueries({ queryKey: ["accounts"] });
    qc.invalidateQueries({ queryKey: ["mailboxes"] });
    qc.invalidateQueries({ queryKey: ["unread"] });
    qc.invalidateQueries({ queryKey: ["messages"] });
  };

  const syncOne = useMutation({
    mutationFn: (id: number) => api.syncAccount(id),
    onSuccess: (o) => {
      if (o.error) message.warning(`${t("msg.syncFailPrefix")}${o.error}`);
      else message.success(t("msg.syncDone", { n: o.new_messages }));
      invalidateAll();
    },
    onError: (e) => message.error(String(e)),
  });

  const toggleOne = useMutation({
    mutationFn: (a: AccountDto) => api.setAccountEnabled(a.id, !a.enabled),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["accounts"] }),
    onError: (e) => message.error(String(e)),
  });

  const removeOne = useMutation({
    mutationFn: (id: number) => api.removeAccount(id),
    onSuccess: () => {
      message.success(t("msg.accountRemoved"));
      invalidateAll();
    },
    onError: (e) => message.error(String(e)),
  });

  const confirmRemove = (a: AccountDto) => {
    modal.confirm({
      title: t("remove.title", { name: a.display_name }),
      content: t("remove.content"),
      okText: t("remove.ok"),
      okButtonProps: { danger: true },
      cancelText: t("cancel"),
      onOk: () => removeOne.mutateAsync(a.id).catch(() => undefined),
    });
  };

  const accountMenu = (a: AccountDto) => ({
    items: [
      { key: "sync", icon: <ReloadOutlined />, label: t("menu.sync") },
      a.enabled
        ? { key: "pause", icon: <PauseCircleOutlined />, label: t("menu.pause") }
        : { key: "resume", icon: <PlayCircleOutlined />, label: t("menu.resume") },
      { key: "edit", icon: <EditOutlined />, label: t("menu.edit") },
      { type: "divider" as const },
      { key: "remove", icon: <DeleteOutlined />, label: t("menu.remove"), danger: true },
    ],
    onClick: ({ key }: { key: string }) => {
      if (key === "sync") syncOne.mutate(a.id);
      else if (key === "pause" || key === "resume") toggleOne.mutate(a);
      else if (key === "edit") setEditing(a);
      else if (key === "remove") confirmRemove(a);
    },
  });

  const toggleExpand = (id: number) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const navItem = (
    key: string,
    active: boolean,
    icon: React.ReactNode,
    label: string,
    badge: number,
    onClick: () => void,
    indent = false
  ) => (
    <div
      key={key}
      onClick={onClick}
      style={{
        display: "flex",
        alignItems: "center",
        gap: 10,
        padding: indent ? "6px 12px 6px 14px" : "8px 12px",
        borderRadius: 10,
        cursor: "pointer",
        marginBottom: 2,
        color: active ? "#fff" : "#3d3d44",
        fontWeight: active ? 600 : 400,
        fontSize: indent ? 13 : undefined,
        background: active ? GRADIENT : undefined,
        boxShadow: active ? GLOW : undefined,
      }}
    >
      {icon}
      <span style={{ flex: 1 }}>{label}</span>
      {badge > 0 && (
        <Badge
          count={badge}
          size="small"
          color={active ? "#fff" : PRIMARY}
          style={active ? { color: PRIMARY } : undefined}
        />
      )}
    </div>
  );

  return (
    <div style={{ padding: "14px 12px 12px", display: "flex", flexDirection: "column", height: "100%" }}>
      {/* 品牌 */}
      <div style={{ display: "flex", alignItems: "center", gap: 10, padding: "0 8px 14px" }}>
        <div
          style={{
            width: 36,
            height: 36,
            borderRadius: 10,
            background: GRADIENT,
            boxShadow: GLOW,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            flexShrink: 0,
          }}
        >
          <MailOutlined style={{ color: "#fff", fontSize: 18 }} />
        </div>
        <div>
          <Typography.Text strong style={{ fontSize: 15, display: "block", lineHeight: 1.2 }}>
            MailDeck
          </Typography.Text>
          <Typography.Text type="secondary" style={{ fontSize: 11 }}>
            {appVersion ? `${t("brand.slogan")} · v${appVersion}` : t("brand.slogan")}
          </Typography.Text>
        </div>
      </div>

      {/* 智能视图 */}
      <div style={{ padding: "0 8px 4px", fontSize: 11, color: "#a0a0a8" }}>{t("group.views")}</div>
      {navItem(
        "all",
        mailboxId === null && !unreadOnly,
        <InboxOutlined style={{ fontSize: 15 }} />,
        t("nav.all"),
        totalUnread,
        () => {
          setMailbox(null);
          setUnreadOnly(false);
        }
      )}
      {navItem(
        "unread",
        mailboxId === null && unreadOnly,
        <MailOutlined style={{ fontSize: 15 }} />,
        t("nav.unread"),
        0,
        () => {
          setMailbox(null);
          setUnreadOnly(true);
        }
      )}

      {/* 账号分组(与参考图一致:组头 = 头像 + 名字 + 折叠,下为状态行 + 文件夹) */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          padding: "12px 8px 6px",
          fontSize: 11,
          color: "#a0a0a8",
        }}
      >
        {t("group.accounts")}
        <span style={{ marginLeft: 4 }}>({accounts.length})</span>
        <span style={{ flex: 1 }} />
        <Button
          type="text"
          size="small"
          icon={<PlusOutlined />}
          onClick={() => setAddOpen(true)}
          title={t("account.add")}
          style={{ width: 22, height: 22, minWidth: 22, color: "#8a8a93" }}
        />
      </div>

      <div style={{ flex: 1, overflow: "auto" }}>
        {accounts.length === 0 && (
          <Typography.Text type="secondary" style={{ fontSize: 12, padding: "8px 12px", display: "block" }}>
            {t("account.empty")}
          </Typography.Text>
        )}
        {accounts.map((a) => {
          const isOpen = expanded.has(a.id);
          const boxes = sortFolders(mailboxesByAccount?.[a.id] ?? []);
          const syncing = syncOne.isPending && syncOne.variables === a.id;
          return (
            <div key={a.id} style={{ marginBottom: 4 }}>
              {/* 组头:头像 + 名字 + 折叠箭头 + 菜单 */}
              <div
                onClick={() => toggleExpand(a.id)}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 8,
                  padding: "6px 8px",
                  borderRadius: 10,
                  cursor: "pointer",
                  opacity: a.enabled ? 1 : 0.55,
                }}
              >
                <span style={{ color: "#8a8a93", fontSize: 10, width: 10 }}>
                  {isOpen ? <DownOutlined /> : <RightOutlined />}
                </span>
                <Avatar
                  shape="square"
                  size={26}
                  style={{
                    background: a.color,
                    borderRadius: 8,
                    fontSize: 12,
                    fontWeight: 600,
                    flexShrink: 0,
                  }}
                >
                  {a.display_name.slice(0, 1).toUpperCase()}
                </Avatar>
                <Typography.Text ellipsis strong style={{ flex: 1, fontSize: 13 }}>
                  {a.display_name}
                </Typography.Text>
                <Dropdown menu={accountMenu(a)} trigger={["click"]}>
                  <Button
                    type="text"
                    size="small"
                    icon={<MoreOutlined />}
                    style={{ width: 22, height: 22, minWidth: 22 }}
                    onClick={(e) => e.stopPropagation()}
                  />
                </Dropdown>
              </div>

              {isOpen && (
                <>
                  {/* 同步状态行(参考图:组头下) */}
                  <div style={{ padding: "0 12px 4px 30px" }}>
                    {a.last_error ? (
                      <Typography.Text type="danger" ellipsis style={{ fontSize: 11, display: "block" }} title={a.last_error}>
                        {a.last_error}
                      </Typography.Text>
                    ) : syncing ? (
                      <Typography.Text type="secondary" style={{ fontSize: 11 }}>
                        <LoadingOutlined style={{ marginRight: 4 }} />
                        {t("account.syncing")}
                      </Typography.Text>
                    ) : (
                      <Typography.Text type="secondary" style={{ fontSize: 11 }}>
                        {a.last_sync_at ? `${t("account.lastSyncPrefix")}${formatTime(a.last_sync_at)}` : t("account.never")}
                      </Typography.Text>
                    )}
                  </div>

                  {/* 文件夹 */}
                  {boxes.length === 0 ? (
                    <Typography.Text type="secondary" style={{ fontSize: 11, padding: "2px 12px 6px 30px", display: "block" }}>
                      {t("account.syncedAfterFolders")}
                    </Typography.Text>
                  ) : (
                    boxes.map((b) =>
                      navItem(
                        `mb-${b.id}`,
                        mailboxId === b.id,
                        FOLDER_ICONS[b.role] ?? FOLDER_ICONS.other,
                        b.role === "other" ? b.name : t(ROLE_KEY[b.role] ?? b.name),
                        unreadByMailbox.get(b.id) ?? 0,
                        () => setMailbox(b.id),
                        true
                      )
                    )
                  )}
                </>
              )}
            </div>
          );
        })}
      </div>

      <AddAccountModal open={addOpen} onClose={() => setAddOpen(false)} onAdded={invalidateAll} />
      <EditAccountModal account={editing} onClose={() => setEditing(null)} />
    </div>
  );
}
