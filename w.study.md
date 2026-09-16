# 项目分析报告（w.study.md）

> 文件职责：对项目代码与外部依赖的系统性研究存档。结构：1 项目概述 → 2 目录结构与模块职责 → 3 核心设计模式（真实代码片段 + 要点列表）→ 4 代码风格观察 → 5 与其他项目对比。代码未落地的章节留占位，落地后按实际回改。
> 立项态：代码未落地，§2/§4/§5 留占位；§3 预登记系列同栈已知陷阱（继承自 CapsulePulse 实测沉淀，本项目落地相应能力后实证复核并回改）。

## 1. 项目概述

CapsuleTODO：玻璃质感的桌面 Todo 看板。固定在桌面、以玻璃为基底，只呈现 Todo 清单供用户勾选，三端（Windows / macOS / Linux）通用。组件流水线（规划态，随一期落地回改）：

```
[状态] core/src/…（拟）            Todo 状态机（纯 Rust 可单测）——模块名随一期方案定案
[存储] core/src/storage.rs（拟）   rusqlite：清单持久化与查询
[展示] ui/（Vue）                  玻璃板 + 清单勾选，业务逻辑零含量
```

系列定位：CapsuleRetro（包装游戏）→ CapsulePlan（落定计划）→ CapsulePulse（记录时间）→ **CapsuleTODO（桌面清单）**。

## 2. 目录结构与模块职责

（代码未落地——沿系列目录风格规划，落地后按实际回改）

```
core/               # Tauri 2 后端（版本单一来源 Cargo.toml）
  Cargo.toml
  tauri.conf.json   # version 字段省略（回落 Cargo.toml）
  src/
    lib.rs          # 应用装配（拟）
    main.rs         # 薄入口（拟）
    …               # 业务模块与 commands/ 随一期方案落地
ui/                 # Vue 3 + TS + Vite 前端（拟）
configs/            # 程序读的参数（预建占位）
data/               # 运行时数据（gitignore，运行时自建）
```

## 3. 核心设计模式

（立项预登记：以下两条为系列同栈已知陷阱，继承自 CapsulePulse 实测沉淀——本项目落地相应能力后须实证复核并回改；写法 = 来源与实测日期 + 真实片段 + 要点与边界）

### 3.1 Tauri 2 ACL 静默拒——能力白名单外的调用零报错失效（系列实测 2026-09-08，tauri 2.11.5）

CapsulePulse PL001 阶段 B 实测：`data-tauri-drag-region` 声明了拖动区但拖动完全不生效，控制台/WebView 零报错。根因：拖动由前端调 `start_dragging` IPC 命令实现，走 Tauri 2 ACL 白名单；`core:default` 集只含 `core:window:default`，而该集（构建产物 `core/gen/schemas/acl-manifests.json` 可查）全部是只读查询权限（position/size/is-* 系），**不含 `allow-start-dragging`**——IPC 被静默拒绝，无任何诊断输出。

```
capabilities/default.json
  "permissions": ["core:default", "core:window:allow-start-dragging"]  # 修复：显式补授
```

要点与边界：

- **症状学**：ACL 拒绝的统一症状 = 功能静默失效 + 控制台干净；命令（invoke）可能成功而能力（listen/drag）全无，或反之——按"哪个 IPC 命令被拒"逐个对号，不能凭"构建绿"推断功能可用
- **系列案例累计两例**：CapsuleRetro `listen()` 被 ACL 静默拒（缺 `core:event`，2026-09-06）；CapsulePulse 拖动静默失效（缺 `allow-start-dragging`，2026-09-08）——凡新增前端能力调用，必须 live 验证功能生效
- **核查手法**：构建产物 `core/gen/schemas/acl-manifests.json` 是权限事实源——`default_permission.permissions` 数组逐项核对，比查文档快且与本机构建版本一致
- **边界**：只影响前端→Rust 的 IPC 能力面；Rust 侧代码（setup 挂玻璃等）不经 ACL，不受影响

### 3.2 vite publicDir 默认在项目根——子目录 public/ 不生效（系列实测 2026-09-09，vite 8.2.2）

CapsulePulse PL003 提示音 404 无声实测：音频放 `ui/public/`，但 vite 的 `publicDir` 默认 = `<project root>/public`（本项目根为 E:\CodeMission\CapsuleTODO）——`ui/public/` 从不被拷贝进 dist，`/chime.wav` 404，`audio.play()` reject 走降级。

```ts
// 修法：import 打包（哈希文件名进 dist），比配置 publicDir 更稳——显式依赖 + 类型安全（vite/client 内建 mp3 声明）
import chimeUrl from "../assets/house_alarm-clock_loud.mp3";
```

要点与边界：

- 实证手法：构建后 `ls dist/assets` 直接看资源是否落地
- 关联：CapsulePulse 曾因 404 → play() reject → 降级 catch 而实测验证了容错白名单路径（提示音失败降级仅通知）；未打包 exe 的系统通知署名回退 PowerShell 属另一独立问题（y.problems #2，随打包解决）

## 4. 代码风格观察

（暂无——代码未落地；首次审计后按实际记录）

## 5. 与其他项目对比

（暂无——候选对照点：CapsulePulse 的 Tauri 2 工程结构、严格抛错 + thiserror 错误模式、容错白名单实践、DWM 焦点联动玻璃配方；本项目与 CapsulePulse 同为单 crate 非 workspace，版本管理方式同源，落地后可在此建立对照表）
