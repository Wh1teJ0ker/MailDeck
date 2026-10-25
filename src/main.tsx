import React, { useEffect } from "react";
import ReactDOM from "react-dom/client";
import { App as AntApp, ConfigProvider, theme as antdTheme } from "antd";
import zhCN from "antd/locale/zh_CN";
import enUS from "antd/locale/en_US";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { getAccent, useSettingsStore } from "./settings";
import App from "./App";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { retry: 1, refetchOnWindowFocus: false },
  },
});

/** 主题与语言总装:AntD dark token、locale、玻璃 CSS 变量全部跟随设置。 */
function ThemedApp() {
  const language = useSettingsStore((s) => s.language);
  const accentId = useSettingsStore((s) => s.accent);

  useEffect(() => {
    const a = getAccent(accentId);
    const root = document.documentElement;
    root.style.setProperty("--md-gradient", `linear-gradient(135deg, ${a.from} 0%, ${a.to} 100%)`);
    root.style.setProperty("--md-gradient-hover", `linear-gradient(135deg, ${a.from} 0%, ${a.to} 60%)`);
    root.style.setProperty("--md-primary", a.primary);
    root.style.setProperty("--md-glow", "0 6px 20px rgba(0,0,0,0.35)");
    root.style.setProperty("--md-base", a.bg);
    root.style.setProperty("--md-ambient", a.ambient);
    root.style.setProperty("--md-glass", a.glass);
    root.style.setProperty("--md-glass-strong", a.glassStrong);
    root.style.setProperty("--md-stroke", a.stroke);
    // 玻璃配方:强模糊 + 饱和度提升(透过玻璃的色彩更浓),内外双层光
    root.style.setProperty("--md-backdrop", "blur(42px) saturate(165%)");
    root.style.setProperty("--md-inner-hi", "inset 0 1px 0 rgba(255,255,255,0.14), inset 0 -1px 0 rgba(255,255,255,0.03)");
    root.style.setProperty("--md-shadow", "0 1px 0 rgba(255,255,255,0.05) inset, 0 14px 40px rgba(0,0,0,0.38)");
    root.style.setProperty("--md-text", a.dark ? "#e9ebee" : "#262b31");
    root.style.setProperty("--md-text-sub", a.dark ? "#9aa3ad" : "#717a85");
  }, [accentId]);

  const a = getAccent(accentId);

  return (
    <ConfigProvider
      locale={language === "en-US" ? enUS : zhCN}
      theme={{
        algorithm: a.dark ? antdTheme.darkAlgorithm : antdTheme.defaultAlgorithm,
        token: {
          colorPrimary: a.primary,
          borderRadius: 10,
          colorBgLayout: "transparent",
          colorBgContainer: a.dark ? "rgba(30,36,46,0.66)" : "rgba(255,255,255,0.72)",
          colorBgElevated: a.dark ? "rgba(26,31,39,0.86)" : "rgba(255,255,255,0.9)",
          colorText: a.dark ? "#e9ebee" : "#262b31",
          colorTextSecondary: a.dark ? "#9aa3ad" : "#717a85",
          fontSize: 13,
        },
      }}
    >
      <AntApp>
        <QueryClientProvider client={queryClient}>
          <App />
        </QueryClientProvider>
      </AntApp>
    </ConfigProvider>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ThemedApp />
  </React.StrictMode>
);
