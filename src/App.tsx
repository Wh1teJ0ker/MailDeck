import { useEffect, useState } from "react";
import { Layout } from "antd";
import { convertFileSrc } from "@tauri-apps/api/core";
import { getAccent, useSettingsStore } from "./settings";
import { api } from "./api";
import AccountSidebar from "./components/AccountSidebar";
import MessageList from "./components/MessageList";
import MessageReader from "./components/MessageReader";
import bgRain from "./assets/bg-rain.svg";
import bgMist from "./assets/bg-mist.svg";

const BUILTIN_BG: Record<string, string> = { rain: bgRain, mist: bgMist };

/** 玻璃色调 token(深/浅),与背景图解耦 */
const GLASS_TONES: Record<string, { panel: string; panelStrong: string; stroke: string; text: string; textSub: string }> = {
  dark: {
    panel:
      "linear-gradient(118deg, rgba(255,255,255,0.09) 0%, rgba(255,255,255,0.03) 26%, transparent 52%)," +
      "linear-gradient(135deg, rgba(14,30,52,0.40), rgba(24,48,76,0.30) 55%, rgba(18,36,64,0.42))",
    panelStrong:
      "linear-gradient(118deg, rgba(255,255,255,0.10) 0%, rgba(255,255,255,0.04) 30%, transparent 55%)," +
      "linear-gradient(135deg, rgba(12,26,48,0.66), rgba(20,42,70,0.58))",
    stroke: "rgba(206,226,255,0.30)",
    text: "#eaf2ff",
    textSub: "#b7c8e0",
  },
  light: {
    panel:
      "linear-gradient(118deg, rgba(255,255,255,0.5) 0%, rgba(255,255,255,0.16) 28%, transparent 55%)," +
      "linear-gradient(135deg, rgba(255,255,255,0.52), rgba(238,244,252,0.38) 55%, rgba(226,236,250,0.46))",
    panelStrong:
      "linear-gradient(118deg, rgba(255,255,255,0.55) 0%, rgba(255,255,255,0.2) 30%, transparent 55%)," +
      "linear-gradient(135deg, rgba(255,255,255,0.8), rgba(244,248,255,0.72))",
    stroke: "rgba(255,255,255,0.75)",
    text: "#1b2a44",
    textSub: "#54688a",
  },
};

export default function App() {
  const accentId = useSettingsStore((s) => s.accent);
  // 背景模式与玻璃色调来自后端设置(可自定义图片)
  const [background, setBackground] = useState<string>("rain");
  const [customBg, setCustomBg] = useState<string | null>(null);

  useEffect(() => {
    const reload = () => {
      api
        .getAppSettings()
        .then((s) => {
          setBackground(s.background);
          if (s.background === "custom") {
            api
              .readCustomBackground()
              .then((bytes) => {
                if (bytes) setCustomBg(`data:image/png;base64,${btoa(String.fromCharCode(...bytes))}`);
              })
              .catch(() => undefined);
          } else {
            setCustomBg(null);
          }
        })
        .catch(() => undefined);
    };
    window.addEventListener("maildeck:background-changed", reload);
    return () => window.removeEventListener("maildeck:background-changed", reload);
  }, []);

  useEffect(() => {
    api
      .getAppSettings()
      .then((s) => {
        setBackground(s.background);
        if (s.background === "custom") {
          api
            .readCustomBackground()
            .then((bytes) => {
              if (bytes) setCustomBg(`data:image/png;base64,${btoa(String.fromCharCode(...bytes))}`);
            })
            .catch(() => undefined);
        }
      })
      .catch(() => undefined);
  }, []);

  const bgSrc = background === "custom" && customBg ? customBg : BUILTIN_BG[background] ?? bgRain;
  const tone = GLASS_TONES[getAccent(accentId).id] ?? GLASS_TONES.dark;

  return (
    <div style={{ height: "100vh", position: "relative", overflow: "hidden" }}>
      {/* 整屏氛围画:内置雨玻璃 / 自定义图片 */}
      <div
        style={{
          position: "absolute",
          inset: 0,
          backgroundImage: `url("${bgSrc}")`,
          backgroundSize: "cover",
          backgroundPosition: "center",
        }}
      />
      <Layout style={{ height: "100vh", background: "transparent", position: "relative", padding: 14, gap: 14 }}>
        <Layout.Sider
          theme="light"
          width={248}
          style={{
            background: tone.panel,
            backdropFilter: "var(--md-backdrop)",
            WebkitBackdropFilter: "var(--md-backdrop)",
            border: "1px solid " + tone.stroke,
            boxShadow: "var(--md-shadow)",
            borderRadius: 18,
            height: "calc(100vh - 28px)",
            overflow: "hidden",
          }}
        >
          <AccountSidebar />
        </Layout.Sider>
        <Layout style={{ background: "transparent", minWidth: 0 }}>
          <Layout.Content style={{ display: "flex", gap: 14, minWidth: 0 }}>
            <div
              style={{
                width: "min(420px, 38vw)",
                minWidth: 300,
                display: "flex",
                flexDirection: "column",
              }}
            >
              <MessageList />
            </div>
            <div style={{ flex: 1, minWidth: 0, display: "flex", flexDirection: "column" }}>
              <MessageReader />
            </div>
          </Layout.Content>
        </Layout>
      </Layout>
    </div>
  );
}
