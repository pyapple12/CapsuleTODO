import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";

/**
 * 前端测试配置（PATCH002.65，来源 = 系列项目 CapsulePulse `ui/tests` 比对）：
 * jsdom 环境 + `setup.ts` 补 jsdom 缺项；用例住 `ui/tests/`，入口 `npm test`（= `vitest run`）。
 */
export default defineConfig({
  plugins: [vue()],
  test: {
    environment: "jsdom",
    include: ["ui/tests/**/*.spec.ts"],
    setupFiles: ["ui/tests/setup.ts"],
  },
});
