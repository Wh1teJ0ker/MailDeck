/**
 * AntD ColorPicker 的表单值既可能是字符串(初始值未被修改时),
 * 也可能是 Color 对象(用户取色后)。这里统一转为十六进制字符串。
 */
export function toHexString(
  value: string | { toHexString: () => string } | null | undefined,
  fallback = "#1677ff"
): string {
  if (typeof value === "string" && value) return value;
  if (value && typeof value === "object" && typeof value.toHexString === "function") {
    return value.toHexString();
  }
  return fallback;
}
