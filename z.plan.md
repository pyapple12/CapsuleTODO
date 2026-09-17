# 项目方案与审计归档（z.plan.md）

> 文件职责：方案文档与审计归档。与 `x.progress.md`（任务清单）、`CapsuleTODO_plan.md`（总体规划）分工：总体规划不动，方案演进与审计记录都落在本文件。
> 结构：一、已完成 ✅ → 二、待完成 → 三、主题规划（按需）→ 四、审计观察项豁免定案清单 → 附录 PL{NNN}（专题方案，立项时创建）→ 附录 A{NNN}（审计报告，由 audit-report 归档环节生成）。

## 一、已完成 ✅

- **FIX001 第 1 轮审计修复**（2026-09-17 收口，live 经用户确认；提交随一期收口统一执行）→ 附录 A001
- **PL003 桌面固定与一期收口**（2026-09-17 收口，用户目验全过；提交随一期收口统一执行）→ 附录 PL003
- **A001 第 1 轮全量代码审计**（2026-09-17 归档，首轮；修复走 FIX001）→ 附录 A001
- **PL002 清单持久化与内容管理**（2026-09-17 收口，用户目验全过；提交随一期收口统一执行）→ 附录 PL002
- **PL001 玻璃壳与最小清单闭环**（2026-09-17 收口，用户目验全过；提交随一期收口统一执行）→ 附录 PL001

## 二、待完成

**一期已全部收口（2026-09-17）**：PL001 / PL002 / PL003 / FIX001 均已完成（见上）。下一步：①一次性提交推送 V0.1.0.3（用户定案，待执行）②二期临时剪贴板（气泡 + 白板）待立项讨论。
**提交策略（用户定案 2026-09-17）**：一期三 PL 全部完成后一次性提交推送——V0.1.0.3 归一期收口提交。
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
> 状态：✅ 已完成（2026-09-17 收口，用户目验全过；提交随一期收口统一执行——用户定案）

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
> 状态：✅ 已完成（2026-09-17 收口，用户目验全过；提交随一期收口统一执行——用户定案）

## 附录 PL003：桌面固定与一期收口（2026-09-17 立项）

> 背景：一期体验收口——"固定在桌面"的产品语义落地（置顶常驻 + 全屏让位），加位置记忆与单实例，一期全量收口。
> 关键决策（2026-09-17 用户定案）：置顶；**全屏应用要盖住板子**（让位）；300×400 固定尺寸；位置记忆要；任务栏/Alt+Tab 暂不隐藏（保持默认显示，no-op 定案记录）；开机自启一期不做（登记 y.problems#4）。
> 方案要点：
>
> - 置顶：tauri.conf.json 窗口配置 `alwaysOnTop: true`
> - 全屏让位：新建 core/src/fullscreen.rs——判定纯函数 is_covering(win_rect, monitor_rect) -> bool（**逐边包含法定案（2026-09-17 修订）**：win 四边均贴到显示器边缘 ±8px 容差判全屏——面积占比法会把小任务栏显示器的最大化窗口误判为全屏，包含法不会；注入矩形可单测）+ 后台线程 1s 轮询（std::thread，extern "system" user32 直连 GetForegroundWindow/GetWindowRect/MonitorFromWindow/GetMonitorInfoW，零新依赖沿系列 extern 模式）：前台窗覆盖板子所在显示器 → 摘 topmost，退出 → 挂回（状态变化才调用，避免每秒重设）。倾向摘 topmost 而非 hide：活动全屏窗口在 z 序非 topmost 区之顶，摘除后自然被盖；live 实测若不足以被盖再降级 hide/show
> - 位置记忆：新建 core/src/settings.rs（沿 Pulse JSON 原子写模式：同目录 .tmp 写入 + rename 替换）——WindowSettings { x: i32, y: i32 }（尺寸固定不入配置；serde default）落 configs/config.json；启动 load（无文件→默认主屏右下距边 40px；越界兜底：不在任一显示器→回默认）→ set_position；窗口销毁/退出时保存（拖动中不写盘）；"文件不存在回默认"登记 AGENTS 容错白名单（三要素）
> - 单实例：tauri-plugin-single-instance = "2"（沿 Pulse PL004.5），builder 首位注册，二次启动唤起主窗并聚焦
> - 收口：全量门禁 + 一期验收清单逐项（置顶常驻/全屏让位/勾选折叠删除线/增删/重启持久/位置记忆/双开防）+ README/AGENTS/计划书状态回写
>
> 状态：🚧 执行中（2026-09-17）——PL003.1–.6 落地并用户目验通过（27 项测试全绿；行为探针：双开防/优雅关闭/关闭落位实证）；待 PL003.7 一期收口，PL003.8 审计已执行归档（见附录 A001，修复走 FIX001）

## 附录 A001：全量代码审计报告（第1轮，2026-09-17）

> 范围：core/ 全部 .rs（9 文件）+ Cargo.toml + build.rs + tauri.conf.json + capabilities/default.json + tests/storage_probe.rs；ui/ 全部 6 件（App.vue / main.ts / style.css / types.ts / components/AddBar.vue / TodoList.vue）；根配置 4 件（package.json / tsconfig.json / vite.config.ts / index.html）。约 1500 行。
> 方式：主会话全量通读（audit-project skill，代码规模小未启用三路并行），对照 AGENTS.md 规范基线与 13 类审计维度逐类核查；门禁现状七项全绿（fmt --check / clippy -D warnings / cargo test 27+探针 / doc 0 告警 / vue-tsc / npm build / prettier）。
> 编号确认（2026-09-17 用户定案）：A001 / FIX001；观察项全部保留观察——不提升 P 级、暂不定案豁免。
> 状态：✅ 已修复（2026-09-17，FIX001 三条闭环，live 目验经用户确认；提交随一期收口统一执行）

### 零、上轮修复复核清单

无上轮（首轮审计）。

### 一、P0-P3 修复清单（按严重度）

| 文件:行号                      | 类型                            | 描述                                                                                                                                                                                                                          | 建议                                                                                                                                  | 性质 | 影响面                |
| ------------------------------ | ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | ---- | --------------------- |
| ui/components/AddBar.vue:21    | 正确性缺陷（13 错误策略一致性） | 失败文案不可读：`String(err)` 作用在跨 IPC 反序列化的 CommandError **对象**（如 `{"Todo":"待办文本超长（上限 100 字符）"}`）上，错误行显示"添加失败：[object Object]"。确定性复现：粘贴 >100 字符文本添加 → 后端 TooLong 拒绝 | Rust 侧单点收敛：CommandError 手动实现 Serialize 按变体输出可读字符串（前端零改动）；验证：live 粘贴超长文本 → 错误行显示完整中文原因 | 新增 | Vue 前端 / Tauri 后端 |
| README.md:25 + AGENTS.md:36-38 | 规范违反（6 文档一致性）        | 计划态措辞滞后实态：README 存储行"（一期定案后回改）"已过时（定案已发生、data/todo.db 已落地）；AGENTS 目录规划树仍标"规划态"，未回填 settings/fullscreen/storage/paths/tests 实态                                            | FIX001.2 统一回改（README 措辞 + AGENTS 目录树实态化）                                                                                | 新增 | 文档                  |

无 P0/P1/P2（安全维度 SQL 全参数化、无路径拼接、无秘钥面；并发维度 Mutex 单写者、锁序单向；防御维度损坏 db/损坏 JSON 均严格报错——未发现确定性可复现的中高危缺陷）。

### 二、参考级观察项（记录不修；用户定案保留观察）

| 文件:行号                                                                              | 描述                                                                                                                       | 回落理由                                                                                             |
| -------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| TodoList.vue:21,31                                                                     | toggle/remove 失败仅 console.error，无用户可见反馈                                                                         | 本地 IPC 毫秒回程失败概率架构性极低且已记录（沿 Pulse 同款）——条件豁免（需求演进统一错误通道时重评） |
| App.vue:23-24                                                                          | todo_list 失败冻结旧清单、无可见反馈                                                                                       | 有意降级——冻结优于报错打断——永久豁免                                                                 |
| lib.rs:134                                                                             | setup 内 expect（业务代码禁 expect 的白名单外硬校验）                                                                      | 架构保证不可达（窗口在 conf 定义且从不销毁；沿 Pulse A002 豁免同款）——永久豁免候选                   |
| settings.rs:47-48                                                                      | 原子写无 fsync，断电窗口可能旧内容                                                                                         | 纯理论；数据仅窗口位置非关键数据——条件豁免（产品定位升级时重评）                                     |
| tauri.conf.json / lib.rs:113,133 / fullscreen.rs:76 / capabilities:5 / main.rs / icons | 打包期一组：未配 CSP；窗口 label 依赖默认 "main"；main.rs 无 windows_subsystem（release 弹控制台）；占位图标；诊断仅控制台 | 打包分发时统一处理——条件豁免（触发 = 打包）                                                          |
| fullscreen.rs:16                                                                       | 容差 8px：自动隐藏任务栏环境（条约 2px）下最大化窗可能误判全屏 → 误让位                                                    | 需特定环境（当前实机非此配置，目验已过）——条件豁免（触发 = 该环境实测；需验证）                      |
| lib.rs:152                                                                             | 位置仅存于 CloseRequested，OS 关机等非常规退出可能不落位置                                                                 | 常规路径（×/Alt+F4）全覆盖，丢失代价小——条件豁免（需求演进时评估双保险）                             |
| x.progress PL002.4 注记                                                                | "坏路径启动失败"未实测（破坏性测试禁触真实 db）                                                                            | 代码路径经单测层错误传播覆盖；live 需备份-损坏-恢复流程——条件豁免（下轮审计或打包前实测；需验证）    |
| storage.rs:91-97,121-127                                                               | TodoItem 行映射闭包两处重复                                                                                                | 3 行小闭包，收敛收益低于抽象成本（KISS）——永久豁免                                                   |
| vite.config.ts:9                                                                       | envPrefix 前瞻键                                                                                                           | 有意保留（沿 Pulse 豁免先例）——永久豁免                                                              |

### 三、亮点

三轮 TDD 红灯实证全程（E0425 级红灯 → 转绿，28 项测试）；SQL 全参数化零拼接；单一事实源 = db 无内存/库双源；容错白名单 4 条三要素齐全；落址偏差实证自纠（db 首落 core/data/ → runtime_root 取父目录 → 探针复核）；全屏判定判法定案有据（包含法 vs 面积法的最大化窗误判分析 + 关键用例）；测试零真实时间/零真实用户数据（内存库 + 临时目录注入）；门禁七项全绿 + cargo doc 0 告警。
