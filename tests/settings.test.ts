import { beforeEach, describe, expect, it } from "vitest";
import { ACCENTS, defaultSettings, useSettingsStore } from "../src/settings";

describe("设置状态(useSettingsStore)", () => {
  beforeEach(() => {
    useSettingsStore.setState({ ...defaultSettings });
  });

  it("默认:简体中文 + 黑(石墨)主题", () => {
    const s = useSettingsStore.getState();
    expect(s.language).toBe("zh-CN");
    expect(s.accent).toBe("graphite");
  });

  it("切换语言", () => {
    useSettingsStore.getState().setLanguage("en-US");
    expect(useSettingsStore.getState().language).toBe("en-US");
  });

  it("仅黑白两套主题;切换有效;未注册的忽略", () => {
    expect(ACCENTS.map((a) => a.id)).toEqual(["graphite", "paper"]);
    useSettingsStore.getState().setAccent("paper");
    expect(useSettingsStore.getState().accent).toBe("paper");
    useSettingsStore.getState().setAccent("rainbow");
    expect(useSettingsStore.getState().accent).toBe("paper");
  });

  it("两套主题的 token 字段完整(可驱动明暗切换)", () => {
    for (const a of ACCENTS) {
      expect(a.primary).toMatch(/^#[0-9a-fA-F]{6}$/);
      expect(a.from).toMatch(/^#[0-9a-fA-F]{6}$/);
      expect(a.to).toMatch(/^#[0-9a-fA-F]{6}$/);
      expect(a.dark).toBeTypeOf("boolean");
      expect(a.bg).toMatch(/^#/);
      expect(a.glass).toMatch(/^rgba/);
      expect(a.stroke).toMatch(/^rgba/);
      expect(a.label.length).toBeGreaterThan(0);
    }
  });
});
