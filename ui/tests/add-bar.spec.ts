import { flushPromises, mount } from "@vue/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import AddBar from "../src/components/AddBar.vue";

describe("AddBar 并发添加（FIX015.28）", () => {
  let wrapper: ReturnType<typeof mount> | undefined;

  beforeEach(() => {
    invokeMock.mockReset();
  });

  afterEach(() => {
    wrapper?.unmount();
    wrapper = undefined;
    vi.restoreAllMocks();
  });

  it("在飞锁只放行一次；请求期间的新草稿不被清空", async () => {
    let release!: () => void;
    invokeMock.mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          release = resolve;
        }),
    );
    wrapper = mount(AddBar);
    const input = wrapper.find<HTMLInputElement>("input.input");
    await input.setValue("第一条");

    await input.trigger("keydown.enter");
    await input.trigger("keydown.enter");
    expect(invokeMock).toHaveBeenCalledTimes(1);

    await input.setValue("第二条");
    release();
    await flushPromises();

    expect(wrapper.emitted("added")).toHaveLength(1);
    expect(input.element.value).toBe("第二条");
  });
});
