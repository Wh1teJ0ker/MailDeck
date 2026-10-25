import { beforeEach, describe, expect, it } from "vitest";
import { ACCENTS, defaultSettings, useSettingsStore } from "../src/settings";

describe("设置状态(useSettingsStore)", () => {
  beforeEach(() => {
    useSettingsStore.setState({ ...defaultSettings });
  });

  it("默认:简体中文 + 深色玻璃 + 雨夜背景", () => {
    const s = useSettingsStore.getState();
    expect(s.language).toBe("zh-CN");
    expect(s.accent).toBe("dark");
  });

  it("切换语言", () => {
    useSettingsStore.getState().setLanguage("en-US");
    expect(useSettingsStore.getState().language).toBe("en-US");
  });

  it("内置玻璃色调两套(深/浅);设置无效值被忽略", () => {
    expect(ACCENTS.map((a) => a.id)).toEqual(["dark", "light"]);
    useSettingsStore.getState().setAccent("light");
    expect(useSettingsStore.getState().accent).toBe("light");
    useSettingsStore.getState().setAccent("rainbow");
    expect(useSettingsStore.getState().accent).toBe("light");
  });

  it("两套主题 token 完整(面板/描边/文字)", () => {
    for (const a of ACCENTS) {
      expect(a.primary).toMatch(/^#/);
      expect(a.panel).toContain("linear-gradient");
      expect(a.panelStrong).toContain("linear-gradient");
      expect(a.stroke).toMatch(/^rgba/);
      expect(a.text).toMatch(/^#/);
      expect(a.textSub).toMatch(/^#/);
      expect(a.dark).toBeTypeOf("boolean");
      expect(a.label.length).toBeGreaterThan(0);
    }
  });
});
