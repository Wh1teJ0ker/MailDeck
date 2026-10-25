import { useEffect, useRef, useState } from "react";
import { App as AntApp, Alert, Button, Descriptions, Form, Input, Modal, Radio, Select, Space, Typography } from "antd";
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
  const [credStore, setCredStore] = useState<string | null>(null);
  const [switchingCred, setSwitchingCred] = useState(false);
  const [background, setBackground] = useState<string>("rain");
  const [glassTone, setGlassTone] = useState<string>("dark");
  const [hasCustomBg, setHasCustomBg] = useState(false);
  const [credStatus, setCredStatus] = useState<{ exists: boolean; unlocked: boolean } | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);
  const [passModalOpen, setPassModalOpen] = useState(false);
  const [pendingMode, setPendingMode] = useState<string | null>(null);
  const [unlockOpen, setUnlockOpen] = useState(false);
  const refreshCredStatus = () => {
    api.credFileStatus().then(setCredStatus).catch(() => setCredStatus(null));
  };

  useEffect(() => {
    if (open) {
      setInfo(null);
      setError(null);
      api.appVersion().then(setVersion).catch(() => setVersion(""));
      api
        .getAppSettings()
        .then((s) => {
          setCredStore(s.credential_store);
          setBackground(s.background);
          setGlassTone(s.glass_tone);
          setHasCustomBg(s.background === "custom");
        })
        .catch(() => undefined);
      refreshCredStatus();
    }
  }, [open]);

  const doSwitch = async (mode: string, passphrase?: string) => {
    setSwitchingCred(true);
    try {
      const r = await api.setCredentialStore(mode, passphrase);
      setCredStore(mode);
      if (r.failed.length > 0) {
        message.warning(t("settings.credMigrateFailed", { n: r.failed.length }));
      } else {
        message.success(t("settings.credSwitched", { n: r.migrated }));
      }
      refreshCredStatus();
    } catch (e) {
      message.error(String(e));
    } finally {
      setSwitchingCred(false);
    }
  };

  const handleCredStoreChange = (mode: string) => {
    if (mode === "file") {
      // 切到文件模式:先弹口令设置
      setPendingMode(mode);
      setPassModalOpen(true);
    } else {
      doSwitch(mode);
    }
  };

  const applyBackground = async (mode: string) => {
    try {
      await api.setBackground(mode);
      setBackground(mode);
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleUpload = async (file: File) => {
    const buf = new Uint8Array(await file.arrayBuffer());
    try {
      await api.uploadBackground(Array.from(buf));
      setHasCustomBg(true);
      setBackground("custom");
      message.success(t("settings.bgUploaded"));
      window.dispatchEvent(new CustomEvent("maildeck:background-changed"));
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleGlassTone = async (tone: string) => {
    try {
      await api.setGlassTone(tone);
      setGlassTone(tone);
      window.dispatchEvent(new CustomEvent("maildeck:background-changed"));
    } catch (e) {
      message.error(String(e));
    }
  };

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

      {/* 背景:内置氛围画 / 自定义图片 */}
      <Typography.Text type="secondary" style={{ fontSize: 12 }}>
        {t("settings.background")}
      </Typography.Text>
      <div style={{ margin: "8px 0 20px", display: "flex", alignItems: "center", gap: 12 }}>
        <Space size={8} wrap>
          {[
            { id: "rain", label: t("settings.bgRain") },
            { id: "mist", label: t("settings.bgMist") },
            { id: "custom", label: t("settings.bgCustom") },
          ].map((b) => {
            const active = background === b.id;
            return (
              <Button
                key={b.id}
                onClick={() => (b.id === "custom" && !hasCustomBg ? fileRef.current?.click() : applyBackground(b.id))}
                type={active ? "primary" : "default"}
                ghost={active}
                size="small"
              >
                {b.label}
                {b.id === "custom" && hasCustomBg ? " ✓" : ""}
              </Button>
            );
          })}
          <Button size="small" onClick={() => fileRef.current?.click()}>
            {t("settings.bgUpload")}
          </Button>
          <Typography.Text type="secondary" style={{ fontSize: 11 }}>
            {t("settings.bgUploadTip")}
          </Typography.Text>
          <input
            ref={fileRef}
            type="file"
            accept="image/jpeg,image/png"
            style={{ display: "none" }}
            onChange={(e) => {
              const f = e.target.files?.[0];
              if (f) handleUpload(f);
              e.target.value = "";
            }}
          />
        </Space>
      </div>

      {/* 玻璃色调:自定义图片时可选深/浅 */}
      {background === "custom" && (
        <div style={{ margin: "0 0 20px", display: "flex", alignItems: "center", gap: 12 }}>
          <span style={{ width: 64, flexShrink: 0, fontSize: 12 }}>{t("settings.bgGlassTone")}</span>
          <Radio.Group
            size="small"
            value={glassTone}
            onChange={(e) => handleGlassTone(e.target.value)}
            options={[
              { label: t("settings.toneDark"), value: "dark" },
              { label: t("settings.toneLight"), value: "light" },
            ]}
            optionType="button"
          />
        </div>
      )}

      {/* 安全:凭证存储方式 */}
      <Typography.Text type="secondary" style={{ fontSize: 12 }}>
        {t("settings.security")}
      </Typography.Text>
      <div style={{ margin: "8px 0 8px", display: "flex", alignItems: "center", gap: 12 }}>
        <span style={{ width: 64, flexShrink: 0 }}>{t("settings.credentialStore")}</span>
        <Select
          value={credStore ?? "keyring"}
          onChange={(v) => handleCredStoreChange(v)}
          loading={switchingCred}
          style={{ width: 240 }}
          options={[
            { value: "keyring", label: t("settings.credKeyring") },
            { value: "file", label: t("settings.credFile") },
          ]}
        />
      </div>
      {credStore === "file" && (
        <>
          <Alert type="warning" showIcon style={{ marginBottom: 8 }} message={t("settings.credFileWarning")} />
          <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 20 }}>
            <Typography.Text type="secondary" style={{ fontSize: 12, flex: 1 }}>
              {credStatus?.unlocked ? t("settings.credUnlocked") : t("settings.credLocked")}
            </Typography.Text>
            {credStatus?.unlocked ? (
              <Button size="small" onClick={async () => { await api.lockCredFile(); refreshCredStatus(); }}>
                {t("settings.credLock")}
              </Button>
            ) : (
              <Button size="small" type="primary" onClick={() => setUnlockOpen(true)}>
                {t("settings.credUnlock")}
              </Button>
            )}
          </div>
        </>
      )}

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

      <PassphraseSetupModal
        open={passModalOpen}
        onCancel={() => { setPassModalOpen(false); setPendingMode(null); }}
        onConfirm={async (pass) => {
          setPassModalOpen(false);
          await doSwitch(pendingMode ?? "file", pass);
          setPendingMode(null);
        }}
      />
      <UnlockModal
        open={unlockOpen}
        onCancel={() => setUnlockOpen(false)}
        onUnlocked={() => { setUnlockOpen(false); refreshCredStatus(); }}
      />
    </Modal>
  );
}

function PassphraseSetupModal(props: { open: boolean; onCancel: () => void; onConfirm: (pass: string) => void }) {
  const t = useT();
  const [form] = Form.useForm<{ pass: string; confirm: string }>();
  return (
    <Modal
      title={t("settings.credPassphraseSetup")}
      open={props.open}
      onCancel={props.onCancel}
      onOk={() => form.submit()}
      okText={t("confirm")}
      cancelText={t("cancel")}
      destroyOnClose
    >
      <Alert type="warning" showIcon style={{ marginBottom: 16 }} message={t("settings.credFileWarning")} />
      <Form form={form} layout="vertical" onFinish={(v) => {
        if (v.pass.length < 6) return;
        if (v.pass !== v.confirm) return;
        props.onConfirm(v.pass);
      }}>
        <Form.Item
          name="pass"
          label={t("settings.credPassphrase")}
          rules={[
            { required: true },
            { min: 6, message: t("settings.credPassphrasePlaceholder") },
          ]}
        >
          <Input.Password placeholder={t("settings.credPassphrasePlaceholder")} />
        </Form.Item>
        <Form.Item
          name="confirm"
          label={t("settings.credPassphraseConfirm")}
          dependencies={["pass"]}
          rules={[
            { required: true },
            ({ getFieldValue }) => ({
              validator: (_, value) =>
                value === getFieldValue("pass")
                  ? Promise.resolve()
                  : Promise.reject(new Error(t("settings.credPassphraseMismatch"))),
            }),
          ]}
        >
          <Input.Password />
        </Form.Item>
      </Form>
    </Modal>
  );
}

function UnlockModal(props: { open: boolean; onCancel: () => void; onUnlocked: () => void }) {
  const t = useT();
  const { message } = AntApp.useApp();
  const [pass, setPass] = useState("");
  const [busy, setBusy] = useState(false);
  return (
    <Modal
      title={t("settings.credUnlock")}
      open={props.open}
      onCancel={props.onCancel}
      onOk={async () => {
        setBusy(true);
        try {
          await api.unlockCredFile(pass);
          props.onUnlocked();
          setPass("");
        } catch {
          message.error(t("settings.credUnlockFail"));
        } finally {
          setBusy(false);
        }
      }}
      confirmLoading={busy}
      okText={t("confirm")}
      cancelText={t("cancel")}
      destroyOnClose
    >
      <Input.Password value={pass} onChange={(e) => setPass(e.target.value)} />
    </Modal>
  );
}
