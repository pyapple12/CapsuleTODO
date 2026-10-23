import { flushPromises, mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import ArchiveOverlay from "../src/components/ArchiveOverlay.vue";

describe("归档板收起态命中（FIX016.15）", () => {
  let wrapper: ReturnType<typeof mount> | undefined;

  beforeEach(() => {
    const board = document.createElement("div");
    board.id = "board";
    document.body.append(board);
  });

  afterEach(() => {
    wrapper?.unmount();
    wrapper = undefined;
    document.getElementById("board")?.remove();
  });

  it("关闭后透明遮罩不再把 mousedown 翻回开板", async () => {
    wrapper = mount(ArchiveOverlay, { props: { items: [] } });
    const vm = wrapper.vm as unknown as {
      toggle: () => Promise<void>;
      close: () => void;
    };

    await vm.toggle();
    await flushPromises();
    await nextTick();
    expect(wrapper.find(".archive-overlay").classes()).toContain("open");

    vm.close();
    await nextTick();
    expect(wrapper.find(".archive-overlay").classes()).not.toContain("open");

    await wrapper.find(".archive-overlay").trigger("mousedown");
    await flushPromises();
    expect(wrapper.find(".archive-overlay").classes()).not.toContain("open");
  });
});
