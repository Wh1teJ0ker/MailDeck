import { beforeEach, describe, expect, it } from "vitest";
import { ACCENTS, defaultSettings, useSettingsStore } from "../src/settings";

describe("设置状态(useSettingsStore)", () => {
  beforeEach(() => {
    useSettingsStore.setState({ ...defaultSettings });
  });

  it("默认:简体中文 + 石墨主题(玻璃设计语言默认)", () => {
    const s = useSettingsStore.getState();
    expect(s.language).toBe("zh-CN");
    expect(s.accent).toBe("graphite");
  });

  it("切换语言", () => {
    useSettingsStore.getState().setLanguage("en-US");
    expect(useSettingsStore.getState().language).toBe("en-US");
  });

  it("切换主题色(仅接受已注册的预设)", () => {
    useSettingsStore.getState().setAccent("teal");
    expect(useSettingsStore.getState().accent).toBe("teal");
    // 未注册的 accent 被忽略
    useSettingsStore.getState().setAccent("rainbow");
    expect(useSettingsStore.getState().accent).toBe("teal");
  });

  it("主题色预设齐全且字段完整", () => {
    expect(ACCENTS.length).toBeGreaterThanOrEqual(4);
    for (const a of ACCENTS) {
      expect(a.primary).toMatch(/^#[0-9a-fA-F]{6}$/);
      expect(a.from).toMatch(/^#[0-9a-fA-F]{6}$/);
      expect(a.to).toMatch(/^#[0-9a-fA-F]{6}$/);
      expect(a.label.length).toBeGreaterThan(0);
    }
  });
});
