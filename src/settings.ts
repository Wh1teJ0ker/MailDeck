import { create } from "zustand";
import { persist } from "zustand/middleware";

/** 界面语言 */
export type Lang = "zh-CN" | "en-US";

/** 玻璃色调(深/浅),与背景图解耦;token 完整驱动一套玻璃视觉。 */
export interface Accent {
  id: string;
  label: string;
  /** 明暗(AntD 算法) */
  dark: boolean;
  primary: string;
  from: string;
  to: string;
  panel: string;
  panelStrong: string;
  stroke: string;
  text: string;
  textSub: string;
}

export const ACCENTS: Accent[] = [
  {
    id: "dark",
    label: "深色玻璃",
    dark: true,
    primary: "#a9c8f2",
    from: "rgba(140,175,225,0.42)",
    to: "rgba(30,64,120,0.52)",
    panel:
      "linear-gradient(118deg, rgba(255,255,255,0.09) 0%, rgba(255,255,255,0.03) 26%, transparent 52%)," +
      "linear-gradient(135deg, rgba(14,30,52,0.40), rgba(24,48,76,0.30) 55%, rgba(18,36,64,0.42))",
    panelStrong:
      "linear-gradient(118deg, rgba(255,255,255,0.10) 0%, rgba(255,255,255,0.04) 30%, transparent 55%)," +
      "linear-gradient(135deg, rgba(12,26,48,0.66), rgba(20,42,70,0.58))",
    stroke: "rgba(206,226,255,0.30)",
    text: "#eaf2ff",
    textSub: "#b7c8e0",
  },
  {
    id: "light",
    label: "浅色玻璃",
    dark: false,
    primary: "#41608c",
    from: "rgba(255,255,255,0.55)",
    to: "rgba(150,175,210,0.45)",
    panel:
      "linear-gradient(118deg, rgba(255,255,255,0.5) 0%, rgba(255,255,255,0.16) 28%, transparent 55%)," +
      "linear-gradient(135deg, rgba(255,255,255,0.52), rgba(238,244,252,0.38) 55%, rgba(226,236,250,0.46))",
    panelStrong:
      "linear-gradient(118deg, rgba(255,255,255,0.55) 0%, rgba(255,255,255,0.2) 30%, transparent 55%)," +
      "linear-gradient(135deg, rgba(255,255,255,0.8), rgba(244,248,255,0.72))",
    stroke: "rgba(255,255,255,0.75)",
    text: "#1b2a44",
    textSub: "#54688a",
  },
];

export interface SettingsState {
  language: Lang;
  /** 玻璃色调 id:dark / light */
  accent: string;
  setLanguage: (l: Lang) => void;
  setAccent: (id: string) => void;
}

export const defaultSettings = {
  language: "zh-CN" as Lang,
  accent: "dark",
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
