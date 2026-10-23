import { mount } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import DelButton from "../src/components/DelButton.vue";

describe("DelButton 双击冒泡（FIX015.30）", () => {
  let host: HTMLElement | undefined;
  let wrapper: ReturnType<typeof mount> | undefined;

  afterEach(() => {
    wrapper?.unmount();
    wrapper = undefined;
    host?.remove();
    host = undefined;
    vi.restoreAllMocks();
  });

  it("双击删除钮不触发行级 dblclick，单击仍走 press", async () => {
    host = document.createElement("div");
    document.body.append(host);
    const rowDblClick = vi.fn();
    host.addEventListener("dblclick", rowDblClick);
    wrapper = mount(DelButton, {
      attachTo: host,
      props: { confirming: false },
    });

    await wrapper.trigger("dblclick");
    expect(rowDblClick).not.toHaveBeenCalled();

    await wrapper.trigger("click");
    expect(wrapper.emitted("press")).toHaveLength(1);
  });
});
