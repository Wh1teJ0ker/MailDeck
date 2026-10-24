import React, { useEffect } from "react";
import ReactDOM from "react-dom/client";
import { App as AntApp, ConfigProvider } from "antd";
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

/** 主题与语言的总装:AntD token、locale 与 CSS 变量全部跟随设置。 */
function ThemedApp() {
  const language = useSettingsStore((s) => s.language);
  const accentId = useSettingsStore((s) => s.accent);

  useEffect(() => {
    const a = getAccent(accentId);
    const root = document.documentElement;
    root.style.setProperty("--md-gradient", `linear-gradient(135deg, ${a.from} 0%, ${a.to} 100%)`);
    root.style.setProperty("--md-gradient-hover", `linear-gradient(135deg, ${a.from} 0%, ${a.to} 60%)`);
    root.style.setProperty("--md-primary", a.primary);
    root.style.setProperty("--md-glow", `0 4px 12px ${a.to}59`);
  }, [accentId]);

  return (
    <ConfigProvider
      locale={language === "en-US" ? enUS : zhCN}
      theme={{
        token: {
          colorPrimary: getAccent(accentId).primary,
          borderRadius: 8,
          colorBgLayout: "#f3f4f6",
          colorText: "#26262b",
          colorTextSecondary: "#8a8a93",
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
