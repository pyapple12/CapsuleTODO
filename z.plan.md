# 项目方案与审计归档（z.plan.md）

> 文件职责：方案文档与审计归档。与 `x.progress.md`（任务清单）、`CapsuleTODO_plan.md`（总体规划）分工：总体规划不动，方案演进与审计记录都落在本文件。
> 结构：一、已完成 ✅ → 二、待完成 → 三、主题规划（按需）→ 四、审计观察项豁免定案清单 → 附录 PL{NNN}（专题方案，立项时创建）→ 附录 A{NNN}（审计报告，由 audit-report 归档环节生成）。

## 一、已完成 ✅

（暂无——立项初期，无收口任务）

## 二、待完成

**一期工程已立项（2026-09-17 用户定案）**：PL001 玻璃壳与最小清单闭环 → PL002 清单持久化与内容管理 → PL003 桌面固定与一期收口，方案见附录 PL001–PL003，任务拆条见 x.progress.md；PL003 收口后进行首轮全量审计（A001，audit-project 触发）。

一期定案要点：窗口 300×400 固定尺寸、置顶、全屏应用盖住板子（让位）、位置记忆、任务栏/Alt+Tab 暂不隐藏；一期支持最小增删（编辑缓）；勾选后折叠进"已完成"区 + 删除线；开机自启一期不做（y.problems#4）。

## 三、主题规划

- （按需）跨附录主题出现时在此登记；远期与遗留项见 y.problems.md

## 四、审计观察项豁免定案清单

> 豁免唯一权威源：已定案项审计时（audit-project）不再重复报告。新定案条目由归档环节（audit-report）经用户确认后追加。分级：①**永久豁免**——不再报告不再讨论；②**条件豁免**——标注触发条件，条件变化时重新评估。

（暂无定案条目）

---

## 附录 PL001：玻璃壳与最小清单闭环（2026-09-17 立项）

> 背景：CapsuleTODO 代码零起步。产品核心洞察 = "桌面一角的玻璃小板，Todo 随手勾"——PL001 用最薄一刀打穿"玻璃壳 + 清单勾选"链路（沿 CapsulePulse PL001 方法论）。与 Pulse 不同：玻璃材质配方已在系列打穿（Pulse PL010/PL011 定案），本期**无判死判定点**，玻璃观感目验确认即可。
> 关键决策（2026-09-17 用户定案）：窗口 300×400 固定尺寸（resizable false）；勾选后条目折叠进"已完成"区 + 删除线；一期支持最小增删（编辑缓到二期）；置顶 + 全屏让位（PL003 落地）。
> 目标：收口时交付"玻璃小板 + 内存态清单勾选闭环"——透明玻璃窗 + 清单展示与勾选 + 完成态删除线与分区；状态机 cargo test 全绿，门禁全绿。
> 方案要点：
>
> - 工程骨架沿系列：core/（Cargo.toml version 0.1.0 单一来源 + tauri.conf.json version 省略 + capabilities）+ ui/（Vue3 + TS + Vite）；依赖版本锚定沿 Pulse 实证（typescript ^5.9.3 防 TS7×vue-tsc 不兼容、vite ^8.2.2、vue ^3.5.42、@tauri-apps/api ^2.11.1、@tauri-apps/cli ^2.11.4、tauri 2、thiserror 2.0.20、serde 1.0.229 derive）
> - tauri.conf.json：productName CapsuleTODO、identifier com.pyapple12.capsule-todo、窗口 300×400 transparent + decorations false + resizable false、frontendDist ../dist + beforeDevCommand npm run build（dev 无 HMR 系列定案）
> - 玻璃：lib.rs 焦点联动材质（extern dwmapi 直连：Focused(true)→DWMSBT_TRANSIENTWINDOW=3、false→DWMSBT_NONE=1（非 0，0=AUTO），平时不挂背板纯 alpha 透明）+ window-focus 事件；App.vue 全窗单层玻璃 + 分态纱（透明态 30% 浅白/纯黑、磨砂态 0%）+ 亮边/text-shadow 令牌对照 Pulse 配方表移植 + prefers-color-scheme 双主题；拖动 = 全局 mousedown + 交互元素白名单 + startDragging（沿 Pulse PL010 定案）
> - 业务：core/src/todo.rs 纯逻辑（禁 import tauri）——TodoItem { id: i64, text: String, done: bool } + TodoList（add/toggle/remove/sorted_view，thiserror：EmptyText/TooLong/NotFound；文本非空且 ≤100 字符；排序 = 未完成在前按 id 升序、已完成在后）；TDD 先红后绿
> - 命令层：core/src/commands/（mod.rs AppContext { list: Mutex\<TodoList\> } + poison 锁助手 + CommandError；todo.rs 四命令 todo_add/todo_toggle/todo_remove/todo_list，核心抽自由函数脱离 tauri::State 直测）；PL001 内存态 + lib.rs 预置示例条目（目验用，PL002 接真实数据后删除）
> - 前端：types.ts 镜像 DTO（serde 单一来源）；App.vue + components/TodoList.vue（清单行 + 自绘复选框 + 完成态删除线 + 进行中/已完成分区）
>
> 状态：🚧 已立项待执行（任务清单见 x.progress.md PL001）

## 附录 PL002：清单持久化与内容管理（2026-09-17 立项）

> 背景：PL001 闭环为内存态，重启即失。PL002 落存储与增删管理，达成"操作即持久化、重启恢复"。
> 关键决策（2026-09-17 用户定案）：一期支持最小增删——添加 + 直接删除（不做确认框：条目价值低、误删可承受，KISS；使用中觉得误删风险高再翻案）；编辑缓到二期随剪贴板议。
> 方案要点：
>
> - 存储：core/src/storage.rs——StorageError（thiserror：Sqlite/Io）+ open(path)（父目录自建）/open_in_memory() + 建表 `todos(id INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT NOT NULL, done INTEGER NOT NULL DEFAULT 0)`（不做 created_at——排序按 id 即创建序，纯逻辑不依赖真实时间，KISS）+ add/set_done/remove/list（全参数化绑定禁拼接）；list 排序 = `ORDER BY done ASC, id ASC`
> - 落址：core/src/paths.rs——runtime_root()（dev = CARGO_MANIFEST_DIR 项目根 / release = current_exe 父目录，cfg 分支）+ data_dir() 自建，db 落 `data/todo.db`（双落址沿系列）；测试一律注入路径/内存库，禁触真实 data/
> - 接线：**单一事实源 = db**——PL002 起 TodoList 内存结构退役（todo.rs 收敛为 TodoItem + TodoError + validate_text 校验助手，排序移交 SQL），AppContext 只剩 storage: Mutex\<Storage\>（锁序 todo → storage 单向）；四命令改走 storage；启动 open 失败严格报错退出（首启无文件 = 建表正常态，无需白名单；库损坏不进白名单）
> - 前端：AddBar.vue 顶部输入行（回车/按钮双触发、空文本禁用、失败红色错误行可见反馈沿 Pulse FIX001.5 模式）；TodoItem 悬停 × 删除；已完成折叠区（默认收起 + "已完成 N" 计数徽章）；空态文案
>
> 状态：🚧 已立项待执行（任务清单见 x.progress.md PL002）

## 附录 PL003：桌面固定与一期收口（2026-09-17 立项）

> 背景：一期体验收口——"固定在桌面"的产品语义落地（置顶常驻 + 全屏让位），加位置记忆与单实例，一期全量收口。
> 关键决策（2026-09-17 用户定案）：置顶；**全屏应用要盖住板子**（让位）；300×400 固定尺寸；位置记忆要；任务栏/Alt+Tab 暂不隐藏（保持默认显示，no-op 定案记录）；开机自启一期不做（登记 y.problems#4）。
> 方案要点：
>
> - 置顶：tauri.conf.json 窗口配置 `alwaysOnTop: true`
> - 全屏让位：新建 core/src/fullscreen.rs——判定纯函数 is_covering(win_rect, monitor_rect) -> bool（覆盖 ≥95% 判全屏，阈值常量注释依据，注入矩形可单测）+ 后台线程 1s 轮询（std::thread，extern "system" user32 直连 GetForegroundWindow/GetWindowRect/MonitorFromWindow/GetMonitorInfoW，零新依赖沿系列 extern 模式）：前台窗覆盖板子所在显示器 → run_on_main_thread 摘 topmost，退出 → 挂回（状态变化才调用，避免每秒重设）。倾向摘 topmost 而非 hide：活动全屏窗口在 z 序非 topmost 区之顶，摘除后自然被盖；live 实测若不足以被盖再降级 hide/show
> - 位置记忆：新建 core/src/settings.rs（沿 Pulse JSON 原子写模式：同目录 .tmp 写入 + rename 替换）——WindowSettings { x: i32, y: i32 }（尺寸固定不入配置；serde default）落 configs/config.json；启动 load（无文件→默认主屏右下距边 40px；越界兜底：不在任一显示器→回默认）→ set_position；窗口销毁/退出时保存（拖动中不写盘）；"文件不存在回默认"登记 AGENTS 容错白名单（三要素）
> - 单实例：tauri-plugin-single-instance = "2"（沿 Pulse PL004.5），builder 首位注册，二次启动唤起主窗并聚焦
> - 收口：全量门禁 + 一期验收清单逐项（置顶常驻/全屏让位/勾选折叠删除线/增删/重启持久/位置记忆/双开防）+ README/AGENTS/计划书状态回写
>
> 状态：🚧 已立项待执行（任务清单见 x.progress.md PL003；收口后触发首轮全量审计 A001）
