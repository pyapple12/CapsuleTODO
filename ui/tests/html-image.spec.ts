import { describe, expect, it } from "vitest";

import { htmlHasImage, htmlImageCount } from "../src/composables/htmlImage";

describe("htmlImage（FIX017.3）：HTML 图片标签判定唯一事实源", () => {
  it("识别 img/image/大写与自闭合", () => {
    expect(htmlHasImage("<img src='x'>")).toBe(true);
    expect(htmlHasImage("<image src='x'>")).toBe(true);
    expect(htmlHasImage("<IMG SRC='x'>")).toBe(true);
    expect(htmlHasImage("<img/>")).toBe(true);
    expect(htmlHasImage("<image/>")).toBe(true);
  });

  it("拒绝同前缀自定义标签与 EOF 的未闭合标签", () => {
    expect(htmlHasImage("<img-foo>")).toBe(false);
    expect(htmlHasImage("<img-a src='x'>")).toBe(false);
    expect(htmlHasImage("<imgxx src='x'>")).toBe(false);
    expect(htmlHasImage("<imgx>")).toBe(false);
    expect(htmlHasImage("<img")).toBe(false);
    expect(htmlHasImage("<image")).toBe(false);
  });

  it("只认 ASCII 空白边界，NBSP/垂直制表不算", () => {
    // FIX017.3：与 Rust is_ascii_whitespace 对齐，避免前后端判定漂移。
    expect(htmlHasImage("<img\u00A0src='x'>")).toBe(false);
    expect(htmlHasImage("<image\u00A0src='x'>")).toBe(false);
    expect(htmlHasImage("<img\vsrc='x'>")).toBe(false);
    expect(htmlHasImage("<img\fsrc='x'>")).toBe(true);
    expect(htmlHasImage("<img\rsrc='x'>")).toBe(true);
  });

  it("按真实标签计数", () => {
    expect(htmlImageCount("<img src='a'><image src='b'>")).toBe(2);
    expect(htmlImageCount("<IMG><img-foo><image/>")).toBe(2);
    expect(htmlImageCount("<p>无图</p>")).toBe(0);
  });
});
