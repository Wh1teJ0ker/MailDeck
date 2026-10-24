import { describe, expect, it } from "vitest";
import { formatSender, formatSize, formatTime } from "../src/utils/format";

describe("formatTime", () => {
  it("空值与非法值返回占位符", () => {
    expect(formatTime(null)).toBe("—");
    expect(formatTime(undefined)).toBe("—");
    expect(formatTime(0)).toBe("—");
  });

  it("合法时间戳输出 YYYY-MM-DD HH:mm 格式", () => {
    // 2026-10-24 10:30:00 +0800 => 1774206600
    const out = formatTime(1774206600);
    expect(out).toMatch(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}$/);
    expect(out).toContain(":");
  });

  it("秒级时间戳按本地时区渲染", () => {
    const ts = Math.floor(Date.now() / 1000);
    const d = new Date(ts * 1000);
    const pad = (n: number) => String(n).padStart(2, "0");
    const expectStr = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(
      d.getHours()
    )}:${pad(d.getMinutes())}`;
    expect(formatTime(ts)).toBe(expectStr);
  });
});

describe("formatSize", () => {
  it("各数量级单位正确", () => {
    expect(formatSize(0)).toBe("0 B");
    expect(formatSize(512)).toBe("512 B");
    expect(formatSize(1024)).toBe("1.0 KB");
    expect(formatSize(1536)).toBe("1.5 KB");
    expect(formatSize(1024 * 1024)).toBe("1.0 MB");
    expect(formatSize(2.5 * 1024 * 1024)).toBe("2.5 MB");
    expect(formatSize(3 * 1024 * 1024 * 1024)).toBe("3.0 GB");
  });

  it("B 级不出现小数", () => {
    expect(formatSize(100)).not.toContain(".");
  });
});

describe("formatSender", () => {
  it("优先显示名,其次地址,最后占位", () => {
    expect(formatSender("张三", "z@qq.com")).toBe("张三");
    expect(formatSender("", "z@qq.com")).toBe("z@qq.com");
    expect(formatSender(null, null)).toBe("(未知发件人)");
  });

  it("空白显示名回退到地址", () => {
    expect(formatSender("   ", "z@qq.com")).toBe("z@qq.com");
  });
});
