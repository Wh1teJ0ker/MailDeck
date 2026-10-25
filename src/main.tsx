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
    // 参考配方:轻玻璃 blur(10px)+saturate(1.25),质感由背后氛围画提供
    root.style.setProperty("--md-panel", a.panel);
    root.style.setProperty("--md-panel-strong", a.panelStrong);
    root.style.setProperty("--md-stroke", a.stroke);
    root.style.setProperty("--md-backdrop", "blur(10px) saturate(1.25)");
    root.style.setProperty("--md-text", a.text);
    root.style.setProperty("--md-text-sub", a.textSub);
    root.style.setProperty("--md-shadow", "0 18px 50px rgba(6,18,40,0.45)");
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
          colorBgContainer: a.dark ? "rgba(16,32,60,0.5)" : "rgba(255,255,255,0.6)",
          colorBgElevated: a.dark ? "rgba(14,30,55,0.88)" : "rgba(255,255,255,0.94)",
          colorText: a.text,
          colorTextSecondary: a.textSub,
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
