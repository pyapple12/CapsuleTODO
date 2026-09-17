# CapsuleTODO — 玻璃质感的桌面 Todo 看板

[![Version](https://img.shields.io/badge/Version-0.1.1.1-blue.svg)](core/Cargo.toml)
[![Rust](https://img.shields.io/badge/Rust-1.96-orange.svg)](https://www.rust-lang.org)
[![Phase](https://img.shields.io/badge/Phase-二期完成-brightgreen.svg)](CapsuleTODO_plan.md)

极简的桌面 Todo 小程序：固定在桌面、以玻璃为基底，只呈现 Todo 清单供用户勾选，随手查看随手勾选。二期规划临时剪贴板（玻璃板上以气泡提示 + 白板，满 5 个气泡提醒清理），三期规划 AI 规范 Todo 与临时内容（均仅记录待细化）。三端（Windows / macOS / Linux）通用，技术栈沿系列项目 CapsulePulse。

| 卖点     | 说明                                                    |
| -------- | ------------------------------------------------------- |
| 固定桌面 | 常驻桌面一隅的玻璃小板，随手查看随手勾选（一期）        |
| 极简勾选 | 只做 Todo 清单与勾选交互，无多余功能                    |
| 玻璃基底 | 沿系列玻璃材质：平时透明常驻、聚焦真磨砂，深浅色自适应  |
| 分期演进 | 二期剪贴板气泡 + 白板、三期 AI 规范内容（仅记录待细化） |

> 当前状态：**二期完成（V0.1.1.1）**——PL004 页签导航与气泡（捕获剪贴板/点击复制回/满 5 提醒二态清空）、PL005 白板（防抖自动保存）、PL006 收口（A002 审计 + FIX002 修复闭环 + 版本推进 0.1.1）全部收口（用户目验全过，45 项测试全绿，门禁七项绿）；一次性提交 V0.1.1.1 待执行。方案见 `z.plan.md` 附录 PL004–PL006 与 A002，任务见 `x.progress.md`。

## 技术栈

| 层   | 选型                           | 说明                                                                         |
| ---- | ------------------------------ | ---------------------------------------------------------------------------- |
| 框架 | Tauri 2（Rust + 系统 WebView） | 三端玻璃插件成熟、包体小（~10MB）                                            |
| 语言 | Rust（后端全部业务逻辑）       | 学习目标：状态机/SQLite 表达自然、可单测                                     |
| 前端 | Vue 3 + TypeScript + Vite      | 与系列一致，只做展示                                                         |
| 存储 | rusqlite（SQLite）             | 零配置本地库，`data/todo.db` 双落址（dev=项目根 / release=exe 同级）         |
| 玻璃 | DWM 焦点联动材质               | 沿 CapsulePulse 配方：平时 alpha 透明常驻、聚焦 Acrylic 真磨砂，深浅色自适应 |

## 项目目标（除产品外）

- **学习 Rust**：全部业务逻辑放 Rust 侧（状态机、持久化、业务规则），前端只做展示——Rust 占比高、可单测
- 系列定位：CapsuleRetro（包装游戏）→ CapsulePlan（落定计划）→ CapsulePulse（记录时间）→ **CapsuleTODO（桌面清单）**

## 快速开始

（规划态——工程随一期落地后生效）

### 环境要求

- Rust toolchain + Node.js（26+）；Tauri CLI 经 npm devDep `@tauri-apps/cli` 随 `npm install` 就位
- Windows 10/11（当前开发与验证实机；macOS/Linux 适配延后至 Windows 版成熟后 [problems#1]）

### 构建与测试

```bash
cargo test        # Rust 层全量测试（不依赖 UI）
npm run tauri dev # 开发运行（透明玻璃窗口；前端产物内嵌自包含，改前端后先 npm run build）
npm run build     # 前端构建校验（含 vue-tsc）
```

## 架构一句话

Todo 状态机、持久化与业务规则全部在 Rust 侧实现并可用 `cargo test` 直测；Vue 前端只做展示与命令转发——业务逻辑零含量。

## 项目结构

（规划态，随一期落地回改）

```
CapsuleTODO/
├── core/                 # Tauri 2 后端（版本单一来源 Cargo.toml；沿系列目录风格）
├── ui/                   # Vue 前端（只做展示）
├── configs/              # 用户参数 config.json（运行时写入，gitignore）
├── data/                 # 运行时数据（gitignore，运行时自建）
├── assets/               # 图标等静态资源
├── AGENTS.md             # 项目规范（AI 协作必读）
├── CapsuleTODO_plan.md   # 总体规划（分期）
├── z.plan.md             # 方案与审计归档
├── x.progress.md         # 任务清单
├── y.problems.md / w.study.md
└── .agents/skills/       # 项目自建 skill（audit-project / audit-report / progress-task）
```

## 文档地图

- `CapsuleTODO_plan.md`：总体规划与分期划分
- `AGENTS.md`：工程原则、代码规范、提交规范（AI 协作必读）
- `z.plan.md`：专题方案与审计归档
- `x.progress.md`：任务清单与进度
- `y.problems.md`：问题与远期改进备忘录
- `w.study.md`：项目分析报告
