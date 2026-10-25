import { create } from "zustand";
import { persist } from "zustand/middleware";

/** 界面语言 */
export type Lang = "zh-CN" | "en-US";

/** 主题色预设:石墨玻璃语言,primary 供 AntD token,from/to 组成品牌渐变 */
export interface Accent {
  id: string;
  label: string;
  primary: string;
  from: string;
  to: string;
}

export const ACCENTS: Accent[] = [
  { id: "graphite", label: "石墨", primary: "#c7cdd6", from: "#454c56", to: "#0e1116" },
  { id: "teal", label: "青碧", primary: "#14b8a6", from: "#17c0aa", to: "#0c8f80" },
  { id: "amber", label: "琥珀", primary: "#d97706", from: "#fbbf24", to: "#b45309" },
  { id: "rose", label: "蔷薇", primary: "#e11d63", from: "#fb7185", to: "#be1244" },
  { id: "azure", label: "天蓝", primary: "#1677ff", from: "#3d8bfd", to: "#1554e0" },
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
