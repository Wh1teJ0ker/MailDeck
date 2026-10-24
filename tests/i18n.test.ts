import { describe, expect, it } from "vitest";
import { dicts, translate, type Lang } from "../src/i18n";

const LANGS: Lang[] = ["zh-CN", "en-US"];

describe("i18n 词典完整性", () => {
  it("所有语言键位集合完全一致(以 zh-CN 为基准)", () => {
    const zhKeys = Object.keys(dicts["zh-CN"]).sort();
    for (const lang of LANGS) {
      const keys = Object.keys(dicts[lang]).sort();
      expect(keys, `${lang} 键位应与 zh-CN 一致`).toEqual(zhKeys);
    }
  });

  it("没有空翻译", () => {
    for (const lang of LANGS) {
      for (const [k, v] of Object.entries(dicts[lang])) {
        expect(typeof v, `${lang}.${k} 应为字符串`).toBe("string");
        expect(v.length, `${lang}.${k} 不应为空`).toBeGreaterThan(0);
      }
    }
  });
});

describe("translate", () => {
  it("按语言返回对应文案", () => {
    expect(translate("zh-CN", "nav.all")).toBe("全部邮件");
    expect(translate("en-US", "nav.all")).toBe("All Mail");
  });

  it("缺失键回退为键名,不抛异常", () => {
    expect(translate("zh-CN", "no.such.key")).toBe("no.such.key");
    expect(translate("en-US", "no.such.key")).toBe("no.such.key");
  });

  it("支持 {name} 插值", () => {
    expect(translate("zh-CN", "msg.syncDone", { n: 3 })).toBe("同步完成,新邮件 3 封");
    expect(translate("en-US", "msg.syncDone", { n: 3 })).toBe("Sync complete, 3 new message(s)");
  });

  it("未知语言回退到中文基准词典", () => {
    expect(translate("fr-FR" as Lang, "nav.all")).toBe("全部邮件");
  });
});
