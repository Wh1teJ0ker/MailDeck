import { useCallback } from "react";
import zhCN from "./zh-CN";
import enUS from "./en-US";
import { useSettingsStore, type Lang } from "../settings";

export type { Lang };

export const dicts: Record<Lang, Record<string, string>> = {
  "zh-CN": { ...zhCN },
  "en-US": { ...enUS },
};

/** 取文案:目标语言缺失回退中文基准,再缺失回退键名;支持 {name} 插值。 */
export function translate(
  lang: Lang,
  key: string,
  params?: Record<string, string | number>
): string {
  const d = dicts[lang] ?? dicts["zh-CN"];
  let s = d[key] ?? dicts["zh-CN"][key] ?? key;
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      s = s.replaceAll(`{${k}}`, String(v));
    }
  }
  return s;
}

/** 组件内使用:t("nav.all") / t("msg.syncDone", { n: 3 }) */
export function useT() {
  const language = useSettingsStore((s) => s.language);
  return useCallback(
    (key: string, params?: Record<string, string | number>) =>
      translate(language, key, params),
    [language]
  );
}
