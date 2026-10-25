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
    bg: "#0d1015",
    ambient:
      "radial-gradient(900px 560px at 15% -8%, rgba(90,120,165,0.40), transparent 62%)," +
      "radial-gradient(820px 620px at 105% 18%, rgba(56,74,105,0.42), transparent 58%)," +
      "radial-gradient(760px 700px at 42% 118%, rgba(96,82,120,0.30), transparent 62%)," +
      "radial-gradient(500px 380px at 78% 68%, rgba(64,84,112,0.22), transparent 65%)",
    glass: "rgba(30,36,46,0.48)",
    glassStrong: "rgba(24,29,37,0.66)",
    stroke: "rgba(255,255,255,0.16)",
  },
  {
    id: "paper",
    label: "素白 · 白",
    dark: false,
    primary: "#3a4149",
    from: "#5a636e",
    to: "#2b3138",
    bg: "#e8ebf1",
    ambient:
      "radial-gradient(900px 560px at 15% -8%, rgba(150,175,230,0.55), transparent 62%)," +
      "radial-gradient(820px 620px at 105% 18%, rgba(180,160,225,0.42), transparent 58%)," +
      "radial-gradient(760px 700px at 42% 118%, rgba(140,205,195,0.38), transparent 62%)," +
      "radial-gradient(500px 380px at 78% 68%, rgba(255,214,170,0.35), transparent 65%)",
    glass: "rgba(255,255,255,0.55)",
    glassStrong: "rgba(255,255,255,0.78)",
    stroke: "rgba(255,255,255,0.75)",
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
