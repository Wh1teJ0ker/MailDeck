import { describe, expect, it } from "vitest";
import { toHexString } from "../src/utils/color";

describe("toHexString(ColorPicker 表单值 → 十六进制色值)", () => {
  it("字符串原样返回", () => {
    expect(toHexString("#1677ff")).toBe("#1677ff");
    expect(toHexString("#ff5500")).toBe("#ff5500");
  });

  it("ColorPicker 对象调用 toHexString()", () => {
    expect(toHexString({ toHexString: () => "#00b96b" })).toBe("#00b96b");
  });

  it("空值与非法输入回退到默认色", () => {
    expect(toHexString(undefined)).toBe("#1677ff");
    expect(toHexString(null)).toBe("#1677ff");
    expect(toHexString("")).toBe("#1677ff");
  });

  it("支持自定义回退色", () => {
    expect(toHexString(null, "#ff5500")).toBe("#ff5500");
  });

  it("非字符串非对象时不抛异常,返回回退色", () => {
    expect(toHexString(123 as unknown as string)).toBe("#1677ff");
  });
});
