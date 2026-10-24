import { Layout } from "antd";
import AccountSidebar from "./components/AccountSidebar";
import MessageList from "./components/MessageList";
import MessageReader from "./components/MessageReader";

export default function App() {
  return (
    <Layout style={{ height: "100vh", background: "#f3f4f6" }}>
      <Layout.Sider
        theme="light"
        width={236}
        style={{ background: "#fbfbfd", borderRight: "1px solid #ececf0", overflow: "hidden" }}
      >
        <AccountSidebar />
      </Layout.Sider>
      <Layout style={{ background: "#f3f4f6" }}>
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
  );
}
