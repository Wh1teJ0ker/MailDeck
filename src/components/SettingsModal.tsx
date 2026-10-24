import { useEffect, useState } from "react";
import { App as AntApp, Alert, Button, Descriptions, Modal, Select, Space, Typography } from "antd";
import { api } from "../api";
import { useT } from "../i18n";
import { ACCENTS, getAccent, useSettingsStore } from "../settings";
import type { UpdateInfoDto } from "../types";

export interface SettingsModalProps {
  open: boolean;
  onClose: () => void;
}

export default function SettingsModal({ open, onClose }: SettingsModalProps) {
  const t = useT();
  const { message, modal } = AntApp.useApp();
  const language = useSettingsStore((s) => s.language);
  const accentId = useSettingsStore((s) => s.accent);
  const setLanguage = useSettingsStore((s) => s.setLanguage);
  const setAccent = useSettingsStore((s) => s.setAccent);

  const [version, setVersion] = useState("");
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [info, setInfo] = useState<UpdateInfoDto | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setInfo(null);
      setError(null);
      api.appVersion().then(setVersion).catch(() => setVersion(""));
    }
  }, [open]);

  const handleCheck = async () => {
    setChecking(true);
    setError(null);
    setInfo(null);
    try {
      setInfo(await api.checkUpdate());
    } catch (e) {
      setError(String(e));
    } finally {
      setChecking(false);
    }
  };

  const handleInstall = async () => {
    setInstalling(true);
    try {
      await api.installUpdate();
      modal.confirm({
        title: t("settings.relailTitle"),
        content: t("settings.relaunchContent"),
        okText: t("settings.restartNow"),
        cancelText: t("settings.restartLater"),
        onOk: async () => {
          await api.relaunch();
        },
      });
    } catch (e) {
      message.error(String(e));
    } finally {
      setInstalling(false);
    }
  };

  return (
    <Modal title={t("settings.title")} open={open} onCancel={onClose} footer={null} destroyOnClose>
      {/* 通用:语言 + 主题色 */}
      <Typography.Text type="secondary" style={{ fontSize: 12 }}>
        {t("settings.general")}
      </Typography.Text>
      <div style={{ margin: "8px 0 20px", display: "flex", flexDirection: "column", gap: 12 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <span style={{ width: 64, flexShrink: 0 }}>{t("settings.language")}</span>
          <Select
            value={language}
            onChange={(v) => setLanguage(v)}
            style={{ width: 180 }}
            options={[
              { value: "zh-CN", label: "简体中文" },
              { value: "en-US", label: "English" },
            ]}
          />
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <span style={{ width: 64, flexShrink: 0 }}>{t("settings.accent")}</span>
          <Space size={8} wrap>
            {ACCENTS.map((a) => {
              const active = a.id === accentId;
              return (
                <button
                  key={a.id}
                  title={a.label}
                  onClick={() => setAccent(a.id)}
                  style={{
                    width: 30,
                    height: 30,
                    borderRadius: 10,
                    border: "none",
                    cursor: "pointer",
                    background: `linear-gradient(135deg, ${a.from} 0%, ${a.to} 100%)`,
                    outline: active ? `2px solid ${a.primary}` : "1px solid rgba(0,0,0,0.08)",
                    outlineOffset: active ? 2 : 0,
                    boxShadow: active ? `0 4px 12px ${a.to}59` : undefined,
                  }}
                />
              );
            })}
          </Space>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            {getAccent(accentId).label}
          </Typography.Text>
        </div>
      </div>

      {/* 版本与更新 */}
      <Typography.Text type="secondary" style={{ fontSize: 12 }}>
        {t("settings.version")}
      </Typography.Text>
      <div style={{ margin: "8px 0 16px", display: "flex", alignItems: "center", gap: 12 }}>
        <span style={{ fontVariantNumeric: "tabular-nums" }}>v{version || "—"}</span>
        <Button size="small" loading={checking} onClick={handleCheck}>
          {t("settings.checkUpdate")}
        </Button>
      </div>

      {error && (
        <Alert
          type="warning"
          showIcon
          style={{ marginBottom: 12 }}
          message={`${t("settings.checkFail")}${error}`}
        />
      )}
      {info && info.available && (
        <>
          <Alert
            type="success"
            showIcon
            style={{ marginBottom: 12 }}
            message={`${t("settings.newVersion")} v${info.version}`}
            description={
              info.notes ? (
                <Typography.Paragraph style={{ marginBottom: 0, whiteSpace: "pre-wrap" }}>
                  {info.notes}
                </Typography.Paragraph>
              ) : undefined
            }
          />
          <Button type="primary" loading={installing} onClick={handleInstall}>
            {t("settings.install")}
          </Button>
        </>
      )}
      {info && !info.available && <Alert type="info" showIcon message={t("settings.upToDate")} />}
    </Modal>
  );
}
