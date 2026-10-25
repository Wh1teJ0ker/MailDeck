import { create } from "zustand";

export type View = "mail" | "settings";

export interface UiState {
  /** null = 收件箱视图(全部账号的收件箱);指定 = 精确文件夹视图 */
  mailboxId: number | null;
  unreadOnly: boolean;
  query: string;
  selectedId: number | null;
  /** 主视图切换 */
  view: View;
  setMailbox: (id: number | null) => void;
  setUnreadOnly: (v: boolean) => void;
  setQuery: (v: string) => void;
  select: (id: number | null) => void;
  setView: (v: View) => void;
}

export const initialUiState: Pick<
  UiState,
  "mailboxId" | "unreadOnly" | "query" | "selectedId" | "view"
> = {
  mailboxId: null,
  unreadOnly: false,
  query: "",
  selectedId: null,
  view: "mail",
};

export const useUiStore = create<UiState>((set) => ({
  ...initialUiState,
  // 切换视图即切换数据源,已选中的邮件不再有效
  setMailbox: (id) => set({ mailboxId: id, selectedId: null }),
  setUnreadOnly: (v) => set({ unreadOnly: v }),
  setQuery: (v) => set({ query: v }),
  select: (id) => set({ selectedId: id }),
  setView: (v) => set({ view: v }),
}));
