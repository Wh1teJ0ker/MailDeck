import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { App as AntApp, Alert, Button, ColorPicker, Form, Input, Modal, Radio, Tag, Typography } from "antd";
import { api } from "../api";
import { useT } from "../i18n";
import { toHexString } from "../utils/color";

export interface AddAccountModalProps {
  open: boolean;
  onClose: () => void;
  onAdded: () => void;
}

interface FormValues {
  email: string;
  authCode: string;
  displayName?: string;
  color?: string | { toHexString: () => string };
}

export default function AddAccountModal({ open, onClose, onAdded }: AddAccountModalProps) {
  const [form] = Form.useForm<FormValues>();
  const qc = useQueryClient();
  const { message } = AntApp.useApp();
  const t = useT();
  const [submitting, setSubmitting] = useState(false);
  const [gmailAuthing, setGmailAuthing] = useState(false);
  const [providerType, setProviderType] = useState("qq");

  // 邮箱类型来自后端注册表:available 决定能否选择,后续接入新服务商无需改这里
  const { data: providers = [] } = useQuery({
    queryKey: ["providers"],
    queryFn: api.listProviders,
    staleTime: Infinity,
  });
  const selected = providers.find((p) => p.id === providerType);

  useEffect(() => {
    if (open) {
      setProviderType("qq");
      setSubmitting(false);
    }
  }, [open]);

  const handleGmailAuth = async () => {
    setGmailAuthing(true);
    try {
      const account = await api.addAccountGmail();
      message.success(t("msg.accountAdded", { name: account.display_name }));
      qc.invalidateQueries({ queryKey: ["accounts"] });
      qc.invalidateQueries({ queryKey: ["unread"] });
      qc.invalidateQueries({ queryKey: ["messages"] });
      form.resetFields();
      onAdded();
      onClose();
    } catch (e) {
      message.error(String(e));
    } finally {
      setGmailAuthing(false);
    }
  };

  const handleFinish = async (values: FormValues) => {
    setSubmitting(true);
    try {
      const account = await api.addAccount({
        providerType,
        email: values.email,
        authCode: values.authCode,
        displayName: values.displayName,
        color: toHexString(values.color),
      });
      message.success(t("msg.accountAdded", { name: account.display_name }));
      qc.invalidateQueries({ queryKey: ["accounts"] });
      qc.invalidateQueries({ queryKey: ["unread"] });
      qc.invalidateQueries({ queryKey: ["messages"] });
      form.resetFields();
      onAdded();
      onClose();
    } catch (e) {
      message.error(String(e));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Modal
      title={t("add.title")}
      open={open}
      onCancel={onClose}
      okText={providerType === "gmail" ? t("add.gmailAuth") : t("add.ok")}
      cancelText={t("cancel")}
      okButtonProps={{ disabled: !selected?.available }}
      confirmLoading={submitting || (providerType === "gmail" && gmailAuthing)}
      onOk={() => (providerType === "gmail" ? handleGmailAuth() : form.submit())}
      destroyOnClose
    >
      <Form.Item label={t("add.type")} style={{ marginBottom: 12 }}>
        <Radio.Group
          value={providerType}
          onChange={(e) => setProviderType(e.target.value)}
          optionType="button"
          buttonStyle="solid"
        >
          {providers.map((p) => (
            <Radio.Button key={p.id} value={p.id} disabled={!p.available}>
              {p.name}
              {!p.available && " · 即将支持"}
            </Radio.Button>
          ))}
        </Radio.Group>
      </Form.Item>

      {selected && (
        <Alert type="info" showIcon style={{ marginBottom: 16 }} message={selected.hint} />
      )}

      {selected?.available && providerType === "gmail" ? (
        <>
          <Alert type="info" showIcon style={{ marginBottom: 16 }} message={t("add.gmailHint")} />
          <Form form={form} layout="vertical" onFinish={handleFinish} initialValues={{ color: "#188038" }}>
            <Form.Item name="color" label={t("add.color")} initialValue="#188038">
              <ColorPicker />
            </Form.Item>
          </Form>
        </>
      ) : selected?.available ? (
        <Form form={form} layout="vertical" onFinish={handleFinish} initialValues={{ color: "#1677ff" }}>
          <Form.Item
            name="email"
            label={t("add.email")}
            rules={[
              { required: true, message: t("add.emailRequired") },
              { type: "email", message: t("add.emailInvalid") },
            ]}
          >
            <Input placeholder="example@qq.com" autoComplete="off" />
          </Form.Item>
          <Form.Item
            name="authCode"
            label={t("add.authCode")}
            rules={[{ required: true, message: t("add.codeRequired") }]}
            extra={t("add.authCodeExtra")}
          >
            <Input.Password placeholder="16 位授权码" autoComplete="new-password" />
          </Form.Item>
          <Form.Item name="displayName" label={t("add.displayName")}>
            <Input placeholder="默认取邮箱前缀" />
          </Form.Item>
          <Form.Item name="color" label={t("add.color")}>
            <ColorPicker />
          </Form.Item>
        </Form>
      ) : (
        selected && (
          <Typography.Paragraph type="secondary">
            {t("add.comingSoon", { name: selected.name, auth: selected.auth_mode })} <Tag color="blue">QQ 邮箱</Tag>
          </Typography.Paragraph>
        )
      )}
    </Modal>
  );
}
