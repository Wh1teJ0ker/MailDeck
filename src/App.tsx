import { Layout } from "antd";
import AccountSidebar from "./components/AccountSidebar";
import MessageList from "./components/MessageList";
import MessageReader from "./components/MessageReader";
import { BASE } from "./theme";

export default function App() {
  return (
    <div
      style={{
        height: "100vh",
        background: BASE,
        position: "relative",
        overflow: "hidden",
      }}
    >
      {/* 石墨基底上的中性弥散光(玻璃层背后的色彩来源) */}
      <div
        style={{
          position: "absolute",
          inset: 0,
          background: "var(--md-ambient)",
          pointerEvents: "none",
        }}
      />
      <Layout style={{ height: "100vh", background: "transparent", position: "relative" }}>
        <Layout.Sider
          theme="light"
          width={244}
          style={{
            background: "var(--md-glass)",
            backdropFilter: "blur(28px)",
            WebkitBackdropFilter: "blur(28px)",
            borderRight: "1px solid var(--md-stroke)",
            overflow: "hidden",
          }}
        >
          <AccountSidebar />
        </Layout.Sider>
        <Layout style={{ background: "transparent" }}>
          <Layout.Content style={{ display: "flex", minWidth: 0, padding: 12, gap: 12 }}>
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
