import { create } from "zustand";
import { persist } from "zustand/middleware";

/** 界面语言 */
export type Lang = "zh-CN" | "en-US";

/** 主题色预设:primary 供 AntD token,from/to 组成品牌渐变 */
export interface Accent {
  id: string;
  label: string;
  primary: string;
  from: string;
  to: string;
}

export const ACCENTS: Accent[] = [
  { id: "blue", label: "极地蓝", primary: "#1677ff", from: "#3d8bfd", to: "#1554e0" },
  { id: "teal", label: "青碧", primary: "#0d9488", from: "#2dd4bf", to: "#0f766e" },
  { id: "violet", label: "鸢尾紫", primary: "#7c5cf5", from: "#9b7bff", to: "#5b3ddc" },
  { id: "rose", label: "蔷薇", primary: "#e11d63", from: "#fb7185", to: "#be1244" },
  { id: "amber", label: "琥珀", primary: "#d97706", from: "#fbbf24", to: "#b45309" },
];

export interface SettingsState {
  language: Lang;
  accent: string;
  setLanguage: (l: Lang) => void;
  setAccent: (id: string) => void;
}

export const defaultSettings = {
  language: "zh-CN" as Lang,
  accent: "blue",
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
