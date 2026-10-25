import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { App as AntApp, Avatar, Button, Empty, Segmented, Space, Spin, Tag, Typography } from "antd";
import { MailOutlined, PaperClipOutlined } from "@ant-design/icons";
import { api } from "../api";
import { useT } from "../i18n";
import { useUiStore } from "../store";
import { GRADIENT, GLOW } from "../theme";
import { formatSender, formatSize, formatTime } from "../utils/format";

export default function MessageReader() {
  const qc = useQueryClient();
  const { message } = AntApp.useApp();
  const t = useT();
  const selectedId = useUiStore((s) => s.selectedId);
  const [view, setView] = useState<"html" | "text">("html");

  const { data: body, isLoading, isError, error } = useQuery({
    queryKey: ["body", selectedId],
    queryFn: () => api.openMessage(selectedId!),
    enabled: selectedId != null,
    retry: false,
    staleTime: Infinity,
  });

  // 切换邮件时回到默认视图
  useEffect(() => {
    setView("html");
  }, [selectedId]);

  useEffect(() => {
    if (body) {
      qc.invalidateQueries({ queryKey: ["messages"] });
      qc.invalidateQueries({ queryKey: ["unread"] });
    }
  }, [body?.seen, selectedId]); // eslint-disable-line react-hooks/exhaustive-deps

  const markUnread = useMutation({
    mutationFn: () => api.setSeen(selectedId!, false),
    onSuccess: () => {
      message.success(t("msg.markUnreadDone"));
      qc.invalidateQueries({ queryKey: ["messages"] });
      qc.invalidateQueries({ queryKey: ["unread"] });
      qc.invalidateQueries({ queryKey: ["body", selectedId] });
    },
    onError: (e) => message.error(String(e)),
  });

  if (selectedId == null) {
    return (
      <ReaderCard>
        <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={t("reader.pick")} />
      </ReaderCard>
    );
  }

  if (isLoading) {
    return (
      <ReaderCard>
        <div style={{ padding: 64, textAlign: "center" }}>
          <Spin tip={t("reader.loading")} />
        </div>
      </ReaderCard>
    );
  }

  if (isError || !body) {
    return (
      <ReaderCard>
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={String(error ?? t("reader.loadFail"))}
          style={{ padding: 32 }}
        />
      </ReaderCard>
    );
  }

  const showHtml = view === "html" && body.html;
  const showText = !showHtml && body.text;
  const hasBoth = body.html && body.text;

  return (
    <div
      style={{
        flex: 1,
        display: "flex",
        flexDirection: "column",
        minHeight: 0,
        background: "var(--md-glass-strong)",
        backdropFilter: "blur(28px)",
        WebkitBackdropFilter: "blur(28px)",
        border: "1px solid var(--md-stroke)",
        borderRadius: 14,
        overflow: "hidden",
      }}
    >
      {/* 头部 */}
      <div style={{ padding: "16px 18px 12px", borderBottom: "1px solid var(--md-stroke)" }}>
        <div style={{ display: "flex", alignItems: "flex-start", gap: 12 }}>
          <Avatar
            shape="square"
            size={44}
            style={{
              background: GRADIENT,
              borderRadius: 12,
              fontSize: 19,
              fontWeight: 600,
              flexShrink: 0,
              boxShadow: GLOW,
            }}
          >
            {formatSender(body.sender, body.sender_addr).slice(0, 1).toUpperCase()}
          </Avatar>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Typography.Title level={5} style={{ margin: 0, fontSize: 16, flex: 1, minWidth: 0 }}>
                {body.subject?.trim() || "(无主题)"}
              </Typography.Title>
              <Tag
                color={body.account_color}
                style={{ borderRadius: 6, fontSize: 11, flexShrink: 0 }}
              >
                {body.account_name}
              </Tag>
            </div>
            <Typography.Text type="secondary" style={{ fontSize: 12, display: "block", marginTop: 2 }}>
              {formatSender(body.sender, body.sender_addr)}
              {body.sender_addr && body.sender ? ` · ${body.sender_addr}` : ""}
            </Typography.Text>
            {body.recipient && (
              <Typography.Text type="secondary" style={{ fontSize: 12, display: "block" }}>
                {t("reader.to")}{body.recipient}
              </Typography.Text>
            )}
            {body.attachments.length > 0 && (
              <div style={{ marginTop: 6 }}>
                <Space wrap size={4}>
                  {body.attachments.map((a) => (
                    <Tag
                      key={a.id}
                      icon={<PaperClipOutlined />}
                      style={{ borderRadius: 6, fontSize: 11, background: "#f6f7fb" }}
                    >
                      {a.name} · {formatSize(a.size)}
                    </Tag>
                  ))}
                </Space>
              </div>
            )}
          </div>
          <div style={{ textAlign: "right", flexShrink: 0 }}>
            <Typography.Text type="secondary" style={{ fontSize: 12, display: "block" }}>
              {formatTime(body.date)}
            </Typography.Text>
            <Typography.Text type="secondary" style={{ fontSize: 11, display: "block" }}>
              {body.mailbox_name}
            </Typography.Text>
            <Button
              size="small"
              type="text"
              disabled={!body.seen}
              loading={markUnread.isPending}
              onClick={() => markUnread.mutate()}
              style={{ marginTop: 4, fontSize: 12 }}
            >
              {t("reader.markUnread")}
            </Button>
          </div>
        </div>
      </div>

      {/* 视图切换 */}
      {hasBoth && (
        <div style={{ padding: "10px 18px 0" }}>
          <Segmented
            size="small"
            value={view}
            onChange={(v) => setView(v as "html" | "text")}
            options={[
              { label: "HTML", value: "html" },
              { label: "纯文本", value: "text" },
            ]}
          />
        </div>
      )}

      {/* 正文 */}
      <div style={{ flex: 1, minHeight: 0, padding: 12, paddingTop: hasBoth ? 8 : 12 }}>
        {showHtml ? (
          <iframe
            title="MailDeck mail body"
            sandbox=""
            srcDoc={body.html ?? ""}
            style={{
              width: "100%",
              height: "100%",
              border: "none",
              borderRadius: 10,
              background: "#fff",
            }}
          />
        ) : showText ? (
          <pre
            style={{
              margin: 0,
              padding: 16,
              whiteSpace: "pre-wrap",
              wordBreak: "break-word",
              fontFamily: "inherit",
              fontSize: 14,
              lineHeight: 1.8,
              height: "100%",
              overflow: "auto",
              boxSizing: "border-box",
              background: "rgba(255,255,255,0.03)",
              borderRadius: 8,
            }}
          >
            {body.text}
          </pre>
        ) : (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={t("reader.emptyBody")}
            style={{ marginTop: 64 }}
          />
        )}
      </div>
    </div>
  );
}

function ReaderCard({ children }: { children: React.ReactNode }) {
  return (
    <div
      style={{
        flex: 1,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        background: "var(--md-glass)",
        border: "1px solid var(--md-stroke)",
        borderRadius: 14,
      }}
    >
      {children}
    </div>
  );
}
