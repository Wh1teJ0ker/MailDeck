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
          background:
            "radial-gradient(900px 500px at 8% -10%, rgba(255,255,255,0.07), transparent 60%)," +
            "radial-gradient(800px 600px at 100% 15%, rgba(120,130,150,0.10), transparent 55%)," +
            "radial-gradient(700px 700px at 45% 115%, rgba(90,100,120,0.10), transparent 60%)",
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
