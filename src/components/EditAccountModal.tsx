import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { App as AntApp, ColorPicker, Form, Input, Modal } from "antd";
import { api } from "../api";
import { useT } from "../i18n";
import { toHexString } from "../utils/color";
import type { AccountDto } from "../types";

export interface EditAccountModalProps {
  account: AccountDto | null;
  onClose: () => void;
}

interface FormValues {
  displayName: string;
  color?: string | { toHexString: () => string };
}

export default function EditAccountModal({ account, onClose }: EditAccountModalProps) {
  const [form] = Form.useForm<FormValues>();
  const qc = useQueryClient();
  const { message } = AntApp.useApp();
  const t = useT();
  const [submitting, setSubmitting] = useState(false);

  const handleFinish = async (values: FormValues) => {
    if (!account) return;
    setSubmitting(true);
    try {
      await api.updateAccount({
        id: account.id,
        displayName: values.displayName,
        color: toHexString(values.color, account.color),
      });
      message.success(t("msg.accountUpdated"));
      qc.invalidateQueries({ queryKey: ["accounts"] });
      qc.invalidateQueries({ queryKey: ["messages"] });
      onClose();
    } catch (e) {
      message.error(String(e));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Modal
      title={t("edit.title")}
      open={account != null}
      onCancel={onClose}
      onOk={() => form.submit()}
      confirmLoading={submitting}
      okText={t("btn.save")}
      cancelText={t("cancel")}
      destroyOnClose
    >
      {account && (
        <>
          <Form.Item label={t("edit.email")} style={{ marginBottom: 12 }}>
            <Input value={account.email} disabled />
          </Form.Item>
          <Form
            form={form}
            layout="vertical"
            onFinish={handleFinish}
            initialValues={{ displayName: account.display_name, color: account.color }}
            key={`${account.id}-${account.display_name}-${account.color}`}
          >
            <Form.Item
              name="displayName"
              label={t("edit.displayName")}
              rules={[{ required: true, message: t("add.nameRequired") }]}
            >
              <Input />
            </Form.Item>
            <Form.Item name="color" label="账号颜色">
              <ColorPicker />
            </Form.Item>
          </Form>
        </>
      )}
    </Modal>
  );
}
