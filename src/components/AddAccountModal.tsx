import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { App as AntApp, Alert, Avatar, Button, ColorPicker, Form, Input, Modal, Tag, Typography } from "antd";
import { MailOutlined } from "@ant-design/icons";
import { api } from "../api";
import { useT } from "../i18n";

export interface AddAccountModalProps {
  open: boolean;
  onClose: () => void;
  onAdded: () => void;
}

interface FormValues {
  email: string;
  authCode: string;
  displayName?: string;
  color?: { toHexString: () => string };
}

const FALLBACK_COLOR: Record<string, string> = {
  qq: "#12b7f5",
  "163": "#c8d32c",
  "126": "#2fbd6b",
  exmail: "#2a6bef",
  icloud: "#8b939d",
  sohu: "#e8a20c",
  sina: "#e6162d",
  aliyun: "#ff6a00",
  gmail: "#188038",
  outlook: "#0078d4",
};

export default function AddAccountModal({ open, onClose, onAdded }: AddAccountModalProps) {
  const t = useT();
  const qc = useQueryClient();
  const { message } = AntApp.useApp();
  const [form] = Form.useForm<FormValues>();
  const [submitting, setSubmitting] = useState(false);
  const [gmailAuthing, setGmailAuthing] = useState(false);
  const [providerType, setProviderType] = useState("qq");

  // 服务商注册表(后端唯一事实源):available 决定能否选择
  const { data: providers = [] } = useQuery({
    queryKey: ["providers"],
    queryFn: api.listProviders,
    staleTime: Infinity,
  });
  const available = providers.filter((p) => p.available);
  const reserved = providers.filter((p) => !p.available);
  const selected = useMemo(
    () => providers.find((p) => p.id === providerType),
    [providers, providerType]
  );
  const isGmail = providerType === "gmail";
  // 固定后缀:输入前缀即可自动补全;空 suffix = 自定义域名,需完整输入
  const suffix = selected?.suffix ?? "";

  useEffect(() => {
    if (open) {
      setProviderType("qq");
      setSubmitting(false);
      setGmailAuthing(false);
    }
  }, [open]);

  const invalidateAll = () => {
    qc.invalidateQueries({ queryKey: ["accounts"] });
    qc.invalidateQueries({ queryKey: ["unread"] });
    qc.invalidateQueries({ queryKey: ["messages"] });
    qc.invalidateQueries({ queryKey: ["mailboxes"] });
  };

  const handleFinish = async (values: FormValues) => {
    if (!selected) return;
    setSubmitting(true);
    try {
      const email = normalizeEmail(values.email, suffix);
      const account = await api.addAccount({
        providerType,
        email,
        authCode: values.authCode,
        displayName: values.displayName,
        color: values.color?.toHexString(),
      });
      message.success(t("msg.accountAdded", { name: account.display_name }));
      invalidateAll();
      form.resetFields();
      onAdded();
      onClose();
    } catch (e) {
      message.error(String(e));
    } finally {
      setSubmitting(false);
    }
  };

  const handleGmailAuth = async () => {
    setGmailAuthing(true);
    try {
      const account = await api.addAccountGmail();
      message.success(t("msg.accountAdded", { name: account.display_name }));
      invalidateAll();
      form.resetFields();
      onAdded();
      onClose();
    } catch (e) {
      message.error(String(e));
    } finally {
      setGmailAuthing(false);
    }
  };

  return (
    <Modal
      title={t("add.title")}
      open={open}
      onCancel={onClose}
      footer={null}
      width={640}
      destroyOnClose
    >
      {/* 服务商网格卡片 */}
      <Typography.Text type="secondary" style={{ fontSize: 12 }}>
        {t("add.type")}
      </Typography.Text>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(3, 1fr)",
          gap: 8,
          margin: "10px 0 16px",
        }}
      >
        {[...available, ...reserved].map((p) => {
          const active = p.id === providerType;
          return (
            <div
              key={p.id}
              onClick={() => {
                if (p.available) setProviderType(p.id);
              }}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                padding: "8px 10px",
                borderRadius: 10,
                cursor: p.available ? "pointer" : "not-allowed",
                border: active ? "2px solid var(--md-primary)" : "1px solid var(--md-stroke)",
                background: active ? "rgba(127,127,127,0.10)" : undefined,
                opacity: p.available ? 1 : 0.45,
              }}
            >
              <Avatar
                shape="square"
                size={28}
                icon={<MailOutlined />}
                style={{
                  background: FALLBACK_COLOR[p.id] ?? "#8b939d",
                  borderRadius: 8,
                  flexShrink: 0,
                  fontSize: 13,
                }}
              />
              <div style={{ minWidth: 0 }}>
                <Typography.Text strong={active} ellipsis style={{ fontSize: 12.5, display: "block" }}>
                  {p.name}
                </Typography.Text>
                <Typography.Text type="secondary" style={{ fontSize: 10.5 }}>
                  {p.available ? p.auth_mode : "即将支持"}
                </Typography.Text>
              </div>
            </div>
          );
        })}
      </div>

      {/* 所选服务商操作指引 */}
      {selected && (
        <Alert type="info" showIcon style={{ marginBottom: 16 }} message={selected.hint} />
      )}

      {selected?.available && isGmail ? (
        <>
          <Alert type="warning" showIcon style={{ marginBottom: 16 }} message={t("add.gmailHint")} />
          <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 12 }}>
            <Avatar shape="square" size={36} style={{ background: FALLBACK_COLOR.gmail, borderRadius: 10 }}>
              G
            </Avatar>
            <div>
              <Typography.Text strong>Gmail</Typography.Text>
              <Typography.Text type="secondary" style={{ display: "block", fontSize: 11 }}>
                {t("settings.gmailClientIdPlaceholder")}
              </Typography.Text>
            </div>
            <span style={{ flex: 1 }} />
            <Button type="primary" loading={gmailAuthing} onClick={handleGmailAuth}>
              {gmailAuthing ? t("add.gmailAuthing") : t("add.gmailAuth")}
            </Button>
          </div>
          <Typography.Text type="secondary" style={{ fontSize: 11.5 }}>
            {t("add.gmailNeedClientId")}
          </Typography.Text>
        </>
      ) : selected?.available ? (
        <Form form={form} layout="vertical" onFinish={handleFinish}>
          <Form.Item
            name="email"
            label={t("add.email")}
            rules={[
              { required: true, message: t("add.emailRequired") },
              { type: "email", message: t("add.emailInvalid") },
            ]}
            extra={suffix ? undefined : t("add.emailFullRequired")}
          >
            <Input
              placeholder={suffix ? `user@${suffix}` : t("add.emailFullRequired")}
              prefix={<MailOutlined style={{ color: "var(--md-text-sub)" }} />}
              autoComplete="off"
            />
          </Form.Item>
          {suffix && (
            <Typography.Text type="secondary" style={{ fontSize: 11.5, display: "block", marginTop: -12, marginBottom: 12 }}>
              只输入 @ 前面的账号即可,提交时自动补全 <Tag style={{ borderRadius: 6 }}>{`@${suffix}`}</Tag>
            </Typography.Text>
          )}
          <Form.Item
            name="authCode"
            label={t("add.passwordLabel", { auth: selected.auth_mode })}
            rules={[{ required: true, message: t("add.codeRequired") }]}
            extra={t("add.authCodeExtra")}
          >
            <Input.Password
              placeholder={selected.auth_mode === "应用专用密码" ? "xxxx-xxxx-xxxx-xxxx" : "16 位授权码"}
              autoComplete="new-password"
            />
          </Form.Item>
          <div style={{ display: "flex", gap: 12, alignItems: "flex-end" }}>
            <Form.Item name="displayName" label={t("add.displayName")} style={{ flex: 1 }}>
              <Input placeholder={t("add.displayNamePlaceholder")} />
            </Form.Item>
            <Form.Item name="color" label={t("add.color")} initialValue={FALLBACK_COLOR[providerType] ?? "#1677ff"}>
              <ColorPicker showText />
            </Form.Item>
          </div>
        </Form>
      ) : null}

      <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
        <Button onClick={onClose}>{t("cancel")}</Button>
        <Button
          type="primary"
          disabled={!selected?.available || submitting || gmailAuthing}
          loading={submitting || gmailAuthing}
          onClick={() => (isGmail ? handleGmailAuth() : form.submit())}
          style={{ background: "var(--md-gradient)", border: "none" }}
        >
          {isGmail ? t("add.gmailAuth") : t("add.ok")}
        </Button>
      </div>
    </Modal>
  );
}

/** 提交前规范化邮箱:trim、小写;固定后缀类型缺 @ 时自动补全后缀。 */
function normalizeEmail(input: string, suffix: string): string {
  const v = input.trim().toLowerCase();
  if (!v.includes("@") && suffix) return `${v}@${suffix}`;
  return v;
}
