import { create } from "zustand";
import { persist } from "zustand/middleware";

/** 界面语言 */
export type Lang = "zh-CN" | "en-US";

/** 主题预设:仅黑白两套,token 完整描述整套视觉(驱动明暗切换)。 */
export interface Accent {
  id: string;
  label: string;
  /** 明暗算法选择(AntD dark/light)与组件分支 */
  dark: boolean;
  /** 强调渐变(选中态/主按钮)与主色 */
  primary: string;
  from: string;
  to: string;
  /** 页面基底 */
  bg: string;
  /** 弥散层背景(CSS background 值) */
  ambient: string;
  /** 玻璃面 */
  glass: string;
  /** 强玻璃面(弹窗/阅读区) */
  glassStrong: string;
  /** 玻璃描边 */
  stroke: string;
}

export const ACCENTS: Accent[] = [
  {
    id: "graphite",
    label: "石墨 · 黑",
    dark: true,
    primary: "#c7cdd6",
    from: "#454c56",
    to: "#0e1116",
    bg: "#101318",
    ambient:
      "radial-gradient(1100px 620px at 12% -12%, rgba(255,255,255,0.05), transparent 60%)," +
      "linear-gradient(160deg, rgba(38,44,52,0.35) 0%, transparent 45%, rgba(20,24,30,0.4) 100%)",
    glass: "rgba(34,39,46,0.72)",
    glassStrong: "rgba(28,32,38,0.92)",
    stroke: "rgba(255,255,255,0.14)",
  },
  {
    id: "paper",
    label: "素白 · 白",
    dark: false,
    primary: "#3a4149",
    from: "#5a636e",
    to: "#2b3138",
    bg: "#eef0f3",
    ambient:
      "radial-gradient(1100px 620px at 12% -12%, rgba(255,255,255,0.75), transparent 60%)," +
      "linear-gradient(160deg, rgba(255,255,255,0.5) 0%, transparent 45%, rgba(190,196,205,0.35) 100%)",
    glass: "rgba(255,255,255,0.74)",
    glassStrong: "rgba(255,255,255,0.94)",
    stroke: "rgba(20,26,34,0.10)",
  },
];

export interface SettingsState {
  language: Lang;
  accent: string;
  setLanguage: (l: Lang) => void;
  setAccent: (id: string) => void;
}

export const defaultSettings = {
  language: "zh-CN" as Lang,
  accent: "graphite",
};

export function getAccent(id: string): Accent {
  return ACCENTS.find((a) => a.id === id) ?? ACCENTS[0];
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      ...defaultSettings,
      setLanguage: (l) => set({ language: l }),
      setAccent: (id) => {
        if (ACCENTS.some((a) => a.id === id)) set({ accent: id });
      },
    }),
    { name: "maildeck-settings" }
  )
);
