import { useRef, useState, type CSSProperties } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { App as AntApp, Avatar, Button, Empty, Input, Segmented, Spin } from "antd";
import { PaperClipOutlined, ReloadOutlined, SearchOutlined, SettingOutlined, CheckOutlined } from "@ant-design/icons";
import { useVirtualizer } from "@tanstack/react-virtual";
import { api } from "../api";
import { useT } from "../i18n";
import { useUiStore } from "../store";
import { GRADIENT, PRIMARY, GLOW } from "../theme";
import type { MessageDto } from "../types";
import { formatSender, formatTime } from "../utils/format";
import SettingsModal from "./SettingsModal";

const PAGE = 100;
const ROW_HEIGHT = 72;

export default function MessageList() {
  const qc = useQueryClient();
  const { message } = AntApp.useApp();
  const t = useT();
  const { mailboxId, unreadOnly, query, selectedId, setUnreadOnly, setQuery, select } = useUiStore();
  const [limit, setLimit] = useState(PAGE);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const parentRef = useRef<HTMLDivElement>(null);

  const { data: messages = [], isLoading, isFetching } = useQuery({
    queryKey: ["messages", { mailboxId, unreadOnly, query, limit }],
    queryFn: () => api.listMessages({ mailboxId, unreadOnly, query, offset: 0, limit }),
  });

  const { data: accounts = [] } = useQuery({ queryKey: ["accounts"], queryFn: api.listAccounts });
  const { data: allMailboxes = [] } = useQuery({
    queryKey: ["mailboxes-flat", accounts.map((a) => a.id).join(",")],
    queryFn: async () => {
      const lists = await Promise.all(accounts.map((a) => api.listMailboxes(a.id)));
      return lists.flat();
    },
    enabled: accounts.length > 0,
  });
  const accountOf = (id: number) => accounts.find((a) => a.id === id);
  const mailboxOf = (id: number) => allMailboxes.find((b) => b.id === id);

  const virtualizer = useVirtualizer({
    count: messages.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 8,
  });

  const syncAll = useMutation({
    mutationFn: () => api.syncAll(),
    onSuccess: (outcomes) => {
      const failed = outcomes.filter((o) => o.error);
      const total = outcomes.reduce((s, o) => s + o.new_messages, 0);
      if (failed.length > 0) {
        message.warning(t("msg.syncPartial", { n: total, failed: failed.length }));
      } else {
        message.success(t("msg.syncDone", { n: total }));
      }
      qc.invalidateQueries({ queryKey: ["messages"] });
      qc.invalidateQueries({ queryKey: ["unread"] });
      qc.invalidateQueries({ queryKey: ["accounts"] });
    },
    onError: (e) => message.error(String(e)),
  });

  // 一键已读:把当前视图的全部未读标为已读
  const markAll = useMutation({
    mutationFn: () => api.markAllSeen(mailboxId),
    onSuccess: (count) => {
      if (count > 0) message.success(t("msg.markAllDone", { n: count }));
      else message.info(t("msg.markAllEmpty"));
      qc.invalidateQueries({ queryKey: ["messages"] });
      qc.invalidateQueries({ queryKey: ["unread"] });
    },
    onError: (e) => message.error(String(e)),
  });

  const scopeLabel = mailboxId != null
    ? mailboxOf(mailboxId)?.name ?? "文件夹"
    : unreadOnly
      ? t("nav.unread")
      : t("scope.all");

  // 卡片上的账号标签:文件夹视图下跨账号,需要标注归属
  const colorOfMessage = (m: MessageDto) => {
    const mb = mailboxOf(m.mailbox_id);
    return accountOf(mb?.account_id ?? -1)?.color ?? "#9aa3b2";
  };
  const accountNameOf = (m: MessageDto) => {
    const mb = mailboxOf(m.mailbox_id);
    return accountOf(mb?.account_id ?? -1)?.display_name ?? "账号";
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 0 }}>
      {/* 搜索与同步(设置入口在顶栏右侧,与参考图一致) */}
      <div style={{ display: "flex", gap: 8, marginBottom: 10 }}>
        <Input
          allowClear
          prefix={<SearchOutlined style={{ color: "#b8b8c0" }} />}
          placeholder={t("list.search")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          style={{ borderRadius: 10, flex: 1, background: "var(--md-glass)", border: "1px solid var(--md-stroke)", backdropFilter: "var(--md-backdrop)", boxShadow: "var(--md-inner-hi)" }}
        />
        <Button
          icon={<ReloadOutlined />}
          loading={syncAll.isPending}
          onClick={() => syncAll.mutate()}
          style={{ borderRadius: 10 }}
        />
        <Button
          icon={<SettingOutlined />}
          onClick={() => setSettingsOpen(true)}
          title={t("list.settings")}
          style={{ borderRadius: 10 }}
        />
      </div>

      {/* 过滤与范围 */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          marginBottom: 10,
          padding: "0 2px",
        }}
      >
        <Segmented
          size="small"
          value={unreadOnly ? "unread" : "all"}
          onChange={(v) => setUnreadOnly(v === "unread")}
          options={[
            { label: "全部", value: "all" },
            { label: "未读", value: "unread" },
          ]}
        />
        <Button
          size="small"
          type="text"
          icon={<CheckOutlined />}
          loading={markAll.isPending}
          onClick={() => markAll.mutate()}
          title={t("list.markAllTip")}
        >
          {t("list.markAll")}
        </Button>
        <span style={{ flex: 1 }} />
        <span style={{ fontSize: 12, color: "var(--md-text-sub)" }}>{scopeLabel}</span>
        {isFetching && !isLoading && <Spin size="small" />}
      </div>

      {/* 卡片列表 */}
      <div
        ref={parentRef}
        style={{ flex: 1, overflow: "auto", minHeight: 0, margin: "0 -4px", padding: "0 4px" }}
      >
        {isLoading ? (
          <div style={{ padding: 32, textAlign: "center" }}>
            <Spin />
          </div>
        ) : messages.length === 0 ? (
          <div
            style={{
              background: "var(--md-glass)",
              border: "1px solid var(--md-stroke)",
              borderRadius: 12,
              backdropFilter: "var(--md-backdrop)",
              boxShadow: "var(--md-inner-hi)",
              marginTop: 8,
            }}
          >
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={query ? t("list.emptyQuery") : t("list.empty")}
              style={{ padding: 32 }}
            />
          </div>
        ) : (
          <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
            {virtualizer.getVirtualItems().map((vi) => {
              const m = messages[vi.index];
              return (
                <div
                  key={m.id}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    width: "100%",
                    height: vi.size,
                    transform: `translateY(${vi.start}px)`,
                    paddingBottom: 8,
                  }}
                >
                  <MessageCard
                    m={m}
                    color={colorOfMessage(m)}
                    accountName={accountNameOf(m)}
                    selected={m.id === selectedId}
                    onClick={() => select(m.id)}
                  />
                </div>
              );
            })}
          </div>
        )}
      </div>

      {messages.length >= limit && (
        <Button
          block
          type="text"
          onClick={() => setLimit((v) => v + PAGE)}
          style={{ marginTop: 4, borderRadius: 10 }}
        >
          {t("list.earlier")}
        </Button>
      )}

      <SettingsModal open={settingsOpen} onClose={() => setSettingsOpen(false)} />
    </div>
  );
}

interface CardProps {
  m: MessageDto;
  color: string;
  accountName: string;
  selected: boolean;
  onClick: () => void;
}

function MessageCard({ m, color, accountName, selected, onClick }: CardProps) {
  const style: CSSProperties = {
    height: "100%",
    display: "flex",
    gap: 10,
    padding: "10px 12px",
    borderRadius: 12,
    cursor: "pointer",
    background: selected ? GRADIENT : "var(--md-glass)",
    backdropFilter: "var(--md-backdrop)",
    WebkitBackdropFilter: "var(--md-backdrop)",
    border: "1px solid var(--md-stroke)",
    boxShadow: selected
      ? "var(--md-glow)"
      : "var(--md-inner-hi), 0 8px 24px rgba(0,0,0,0.22)",
    transition: "box-shadow 0.15s ease, transform 0.15s ease",
  };
  return (
    <div style={style} onClick={onClick}>
      <Avatar
        shape="square"
        size={36}
        style={{
          background: selected ? "rgba(255,255,255,0.25)" : color,
          borderRadius: 10,
          fontSize: 15,
          fontWeight: 600,
          flexShrink: 0,
        }}
      >
        {formatSender(m.sender, m.sender_addr).slice(0, 1).toUpperCase()}
      </Avatar>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <span
            style={{
              fontWeight: m.seen ? 500 : 700,
              fontSize: 13,
              color: selected ? "#fff" : "var(--md-text)",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
              flex: 1,
            }}
          >
            {formatSender(m.sender, m.sender_addr)}
          </span>
          <span
            style={{
              fontSize: 11,
              color: selected ? "rgba(255,255,255,0.8)" : "var(--md-text-sub)",
              flexShrink: 0,
            }}
          >
            {formatTime(m.date)}
          </span>
        </div>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 6,
            marginTop: 2,
          }}
        >
          <span
            style={{
              fontSize: 12.5,
              color: selected ? "rgba(255,255,255,0.92)" : "var(--md-text)",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
              flex: 1,
              fontWeight: m.seen ? 400 : 600,
            }}
          >
            {m.subject?.trim() || "(无主题)"}
          </span>
          {m.has_attachment && (
            <PaperClipOutlined
              style={{ fontSize: 12, color: selected ? "rgba(255,255,255,0.85)" : "#a0a0a8" }}
            />
          )}
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 6, marginTop: 3 }}>
          <span
            style={{
              display: "inline-block",
              fontSize: 10.5,
              lineHeight: "16px",
              padding: "0 6px",
              borderRadius: 6,
              background: selected ? "rgba(255,255,255,0.2)" : `${color}1a`,
              color: selected ? "#fff" : color,
              maxWidth: 120,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {accountName}
          </span>
          {!m.seen && !selected && (
            <span
              style={{
                width: 6,
                height: 6,
                borderRadius: 3,
                background: PRIMARY,
                flexShrink: 0,
              }}
            />
          )}
        </div>
      </div>
    </div>
  );
}
