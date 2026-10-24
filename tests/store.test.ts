import { beforeEach, describe, expect, it } from "vitest";
import { initialUiState, useUiStore } from "../src/store";

describe("UI 状态(useUiStore)", () => {
  beforeEach(() => {
    // merge 而非 replace:replace 会把 action 一并清掉
    useUiStore.setState(initialUiState);
  });

  it("初始状态为全部邮件视图、不过滤、无选中", () => {
    const s = useUiStore.getState();
    expect(s.mailboxId).toBeNull();
    expect(s.unreadOnly).toBe(false);
    expect(s.query).toBe("");
    expect(s.selectedId).toBeNull();
  });

  it("切换文件夹视图会清空选中的邮件", () => {
    useUiStore.getState().select(42);
    useUiStore.getState().setMailbox(9);
    expect(useUiStore.getState().mailboxId).toBe(9);
    expect(useUiStore.getState().selectedId).toBeNull();
  });

  it("切回全部邮件视图同样清空选中", () => {
    useUiStore.getState().setMailbox(9);
    useUiStore.getState().select(42);
    useUiStore.getState().setMailbox(null);
    expect(useUiStore.getState().mailboxId).toBeNull();
    expect(useUiStore.getState().selectedId).toBeNull();
  });

  it("未读过滤与搜索词独立更新", () => {
    useUiStore.getState().setUnreadOnly(true);
    useUiStore.getState().setQuery("会议");
    const s = useUiStore.getState();
    expect(s.unreadOnly).toBe(true);
    expect(s.query).toBe("会议");
    useUiStore.getState().setQuery("");
    expect(useUiStore.getState().query).toBe("");
    expect(useUiStore.getState().unreadOnly).toBe(true);
  });
});
