# 项目方案与审计归档（z.plan.md）

> 文件职责：方案文档与审计归档。与 `x.progress.md`（任务清单）、`CapsuleTODO_plan.md`（总体规划）分工：总体规划不动，方案演进与审计记录都落在本文件。
> 结构：一、已完成 ✅ → 二、待完成 → 三、主题规划（按需）→ 四、审计观察项豁免定案清单 → 附录 PL{NNN}（专题方案，立项时创建）→ 附录 A{NNN}（审计报告，由 audit-report 归档环节生成）。

## 一、已完成 ✅

- **A002 第 2 轮全量代码审计**（2026-09-17 归档；修复走 FIX002）→ 附录 A002
- **PL005 白板**（2026-09-17 代码落地勾结；live 移交统一目验；提交随二期收口统一执行）→ 附录 PL005
- **PL004 页签导航与气泡**（2026-09-17 代码落地勾结；live 移交统一目验；提交随二期收口统一执行）→ 附录 PL004
- **FIX001 第 1 轮审计修复**（2026-09-17 收口，live 经用户确认；提交随一期收口统一执行）→ 附录 A001
- **PL003 桌面固定与一期收口**（2026-09-17 收口，用户目验全过；提交随一期收口统一执行）→ 附录 PL003
- **A001 第 1 轮全量代码审计**（2026-09-17 归档，首轮；修复走 FIX001）→ 附录 A001
- **PL002 清单持久化与内容管理**（2026-09-17 收口，用户目验全过；提交随一期收口统一执行）→ 附录 PL002
- **PL001 玻璃壳与最小清单闭环**（2026-09-17 收口，用户目验全过；提交随一期收口统一执行）→ 附录 PL001

## 二、待完成

**APP 回归执行中（2026-09-26 立项）**：PL008–PL014 七组直干 main（不开分支——用户定案 2026-09-26）；每 PL 自验三道闸（Rust TDD / IAB 冒烟断言 / 截图比对 design 忠实）+ 一条 feat 提交推送（V0.1.1.5 起 R+1）；**中间不请用户目验，PL014 后一次总目视验收**，问题走 FIX003；方案见附录 PL008–PL014，任务组见 x.progress.md 未完成区。
历史收口备忘：二期 PL004–PL006 + FIX002 已收口（2026-09-17，V0.1.1.1）；设计实验场 UI 初版已并入（2026-09-26，V0.1.1.4，分支 ui-1.0-feature 保留归档、标签 ui1.0-final）。
一期定案要点：置顶、全屏应用盖住板子（让位）、位置记忆、任务栏/Alt+Tab 暂不隐藏；开机自启一期不做（y.problems#4）。

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

## 附录 PL004：页签导航与气泡（2026-09-17 立项）

> 背景：二期核心 = "临时剪贴板"——把随手复制的内容以气泡钉在板上，用完即走，满 5 提醒清理；承载它的三页签导航同时为白板（PL005）备好容器。
> 关键决策（2026-09-17 用户定案，按推荐）：气泡仅手动捕获剪贴板文本（零自动监听——隐私与噪音考量）；点击气泡 = 复制回剪贴板（捕获→粘贴走→清理闭环）；满 5 软提醒不自动删，5 固定常量；单条删除不确认（沿一期），一键清空用按钮二态确认（首点变红"确认清空 N 条？"再点执行，3 秒复位——沿一期无弹窗基调，不引入 ConfirmModal 组件）；剪贴板非文本统一提示"剪贴板无文本内容"；无全局热键（远期可加）。
> 目标：收口时交付"页签切换 + 气泡全链路"——清单/气泡/白板三页签、捕获剪贴板成气泡、点击复制回、单删与清空、满 5 横幅提醒。
> 方案要点：
>
> - 导航：App.vue 增 `activeTab`（todos/bubbles/whiteboard，默认 todos）+ 顶栏下分段控件三按钮（当前项 accent 高亮）；既有清单区整体迁入 todos 页；切页 watch 触发对应数据刷新；纯前端，Rust 零改动
> - 数据：新表 `bubbles(id INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT NOT NULL)`（不做 created_at——新在前按 id 倒序，沿一期 KISS）；storage 增 add_bubble/list_bubbles（ORDER BY id DESC）/remove_bubble（零行 NotFound）/clear_bubbles（返回清除数）/count_bubbles
> - 纯逻辑：core/src/bubble.rs——`BubbleItem { id, text }` + `MAX_BUBBLES: usize = 5`（注释可调）+ `should_remind(count) -> bool` + 捕获文本校验（trim 非空）；提醒判定薄但独立成模块，随 TDD 立基线
> - 剪贴板：tauri-plugin-clipboard-manager（官方插件，二期唯一新依赖）——Rust 侧 `app.clipboard().read_text()/write_text()`，不经 ACL，capabilities 不动；命令 bubble_capture（读→Err/空报"剪贴板无文本内容"→校验→入库）/bubble_list/bubble_copy(id)（回读文本→写剪贴板）/bubble_remove(id)/bubble_clear()；CommandError 增 Clipboard(String) 变体（手动 Serialize match 加分支——跨 IPC 纯字符串契约沿 A001 修复后形态）；核心自由函数直测内存库，clipboard 读写留命令薄壳
> - UI：ui/components/BubblesView.vue——顶部捕获按钮（失败错误行沿 AddBar 模式）+ 满 5 横幅"气泡已满 5 个，该清理了"与清空二态确认按钮 + 气泡条列表（点击复制回→状态行"已复制到剪贴板"2 秒消失；悬停 × 删除）+ 空态；types.ts 增 BubbleItem 镜像
>
> 状态：✅ 已完成（2026-09-17 代码落地，38 项测试全绿；live 全链路移交统一目验；提交随二期收口统一执行）

## 附录 PL005：白板（2026-09-17 立项）

> 背景：一期清单之外的"临时写内容"容器——一块持续存在的文本草稿区，随手记随手没，自动保存零操作成本。
> 关键决策（2026-09-17 用户定案，按推荐）：纯文本草稿区（不做画笔 canvas——"写内容"字面即文字）；防抖自动保存（~800ms，常量注释可调）+ 切页 flush + 关窗 flush（前端 onCloseRequested 异步保存后放行，不调 prevent 无需额外权限）；内容软上限 10,000 字符严格拒绝（防呆，常量可调）。
> 目标：收口时交付"白板页即写即存"——输入自动保存、切页与关窗不丢、重启仍在。
> 方案要点：
>
> - 数据：新表 `whiteboard(id INTEGER PRIMARY KEY CHECK (id = 1), content TEXT NOT NULL DEFAULT '')` 单行约束；storage 增 load_whiteboard()（无行返回空串）/save_whiteboard(content)（INSERT OR REPLACE UPSERT）
> - 纯逻辑：core/src/whiteboard.rs——`MAX_CONTENT_LEN: usize = 10_000`（注释可调）+ `validate_content`（≤ 上限，超长拒）；与 todo.rs 校验模块对称
> - 命令：commands/whiteboard.rs——whiteboard_load() -> String / whiteboard_save(content)（校验→落库）；核心直测内存库
> - UI：ui/components/WhiteboardView.vue——全页 textarea（玻璃面板内嵌）+ 底部状态行（脏"编辑中…" → 防抖后"✓ 已自动保存"）
> - **实现修订（2026-09-17）**：App.vue 关窗 flush（JS onCloseRequested）实测挂起关闭——Tauri 2 该 API 把关闭权移交 webview destroy 路径，本机探针关闭挂起（含 no-op handler 对照实验排除 flush 本体），定案回退为纯 800ms 防抖自动保存 + 组件常驻挂载（v-show），丢字窗口仅"输入后 0.8s 内即退出"；关窗强 flush 登记已知局限，后期如需再上 Rust 侧方案
>
> 状态：✅ 已完成（2026-09-17 代码落地，44 项测试全绿；live 移交统一目验；提交随二期收口统一执行）

## 附录 PL006：二期收口（2026-09-17 立项）

> 背景：二期质量闸门与收尾——全量审计（A002）、版本推进、状态回写，沿一期"审计先行后收口"定案节奏。
> 关键决策（2026-09-17 用户定案）：版本推进 Cargo.toml 0.1.0 → **0.1.1**（一期→二期 milestone，R 回 1）；二期沿用"做完统一提交"节奏——单提交 **V0.1.1.1**。
> 方案要点：
>
> - 收口顺序：全量门禁 + 二期验收清单（捕获/非文本提示/复制回/单删/满 5 横幅/清空二态确认/白板自动保存/关窗 flush/页签切换下的一期功能回归）→ 版本推进 → A002 全量审计 → FIX002 修复闭环 → README/AGENTS/计划书状态回写 → commit 草案交用户
>
> 状态：🚧 执行中（2026-09-17）——版本推进 0.1.1 已落地（44 项测试全绿、门禁七项绿）；PL004/PL005 已归档勾结，live 统一目验与 FIX002 修复、收口回写待做（任务清单见 x.progress.md PL006/FIX002）

## 附录 PL007：界面实验场 design/（2026-09-17 立项）

> 背景：沿参考项目 CapsulePulse design/ 方法论——独立可交互原型（纯 HTML/CSS/JS，零构建）复刻 APP 完整界面与交互，玻璃配方令牌化集中一处；界面/材质/动效调整在实验场零风险试错，定稿后按映射表落回 ui/（后续单独 PL）。用户定案：**本 PL 做完不审计**，验收 = 浏览器目验。
> 关键决策（2026-09-17 用户定案）：①配方基准 = 1:1 复刻现行 APP 观感（先"像"再"改"）；②演示背景由用户提供两张照片，转小 jpg 入 assets/（验证用，标注不随落地进 APP）；③RESEARCH.md 直接同步 CapsulePulse 同名文件（本项目同样引用 uiverse galaxy 组件库做动效/技法挖掘）。
> 方案要点：
>
> - design/index.html：三页签完整交互复刻（清单添加/勾选/折叠区/删除、气泡捕获/点击复制回/满 5 横幅/二态清空、白板即写即存状态行）——纯客户端模拟状态，无 Tauri 无构建；演示捕获 = 示例片段池（剪贴板真链路属 APP，实验场只管观感）；聚焦态预览 = backdrop-filter 近似磨砂（页面内近似，真窗口仍为一期 DWM 背板）
> - design/glass.css：玻璃引擎——现行 App.vue 令牌 1:1 移植（30% 分态纱/亮边/rim/落影/accent/accent 亮紫）+ 深浅双主题（prefers-color-scheme）+ 背景图接管；调参 = 改令牌读数
> - design/README.md：职责表 + 基准配方读数 + 查看方式 + 映射表（原型区块 → App.vue/TodoList/AddBar/BubblesView/WhiteboardView → 落点）+ 真实窗口可行性标注（✅ 页面内效果 / ⚠️ 近似 / ❌ OS 边界）
> - design/RESEARCH.md：自 CapsulePulse design/RESEARCH.md 同步（R001 uiverse galaxy 调研全量内容，含动效候选清单与引入改造规范）
> - design/assets/：用户照片两张转小 jpg（宽 1600 q80，PowerShell System.Drawing 转换）
> - 红线：不动 ui/ 一行（落地回 ui/ 属后续单独 PL）；design/ 不进 vite 构建与打包
>
> 状态：✅ 已完成（2026-09-17，五文件齐备 + 素材转换 + JS 语法自检过；浏览器观感目验随使用进行；不做审计——用户定案）

## 附录 A002：全量代码审计报告（第2轮，2026-09-17）

> 范围：同 A001 全量范围；重点 = 二期新增（bubble.rs / whiteboard.rs / commands/bubble.rs / commands/whiteboard.rs / storage 扩展 / lib 插件接线 / App.vue 页签重构 / BubblesView.vue / WhiteboardView.vue / types.ts）。约 1900 行。
> 方式：主会话全量通读（audit-project skill，第 2 轮），对照 AGENTS.md 规范基线与 13 类审计维度逐类核查；门禁现状七项全绿（fmt --check / clippy -D warnings / cargo test 44 / doc 0 告警 / vue-tsc / npm build / prettier）。
> 编号确认（2026-09-17 用户定案）：A002 / FIX002；观察项延续保留观察——不提升 P 级、暂不定案豁免（A001 观察项 10 条延续，不重复罗列）。
> 状态：✅ 已修复（2026-09-17，FIX002 五条闭环，live 目验经用户确认；提交随二期收口统一执行）

### 零、上轮修复复核清单

| 上轮条目                            | 现状   | 证据（文件:行号）                                                                                                                             |
| ----------------------------------- | ------ | --------------------------------------------------------------------------------------------------------------------------------------------- |
| FIX001.1 CommandError 跨 IPC 可读化 | ✅仍在 | core/src/commands/mod.rs:48 手动 impl serialize_str 契约在位，且随二期 Clipboard/Whiteboard 变体正确扩展；契约测试覆盖五变体（mod.rs:78-107） |
| FIX001.2 文档实态回改               | ✅仍在 | README"一期定案后回改"与 AGENTS"规划态"双 grep 归零（0/0）；注：二期新增模块使 AGENTS 目录树再次滞后——属新增问题（见 P3-2），非回退           |

### 一、P0-P3 修复清单（按严重度）

| 文件:行号                                                    | 类型                                           | 描述                                                                                                                                                                                  | 建议                                                                                                                                                             | 性质 | 影响面                  |
| ------------------------------------------------------------ | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---- | ----------------------- |
| ui/App.vue:67 + ui/components/BubblesView.vue:101            | 正确性缺陷（交互劫持，沿 Pulse FIX003.1 同类） | `.bubble-row` 未入拖动白名单：mousedown 处理器对白名单外元素一律 startDragging，按住气泡行会启动窗口拖拽并吞掉 click——"点击气泡复制回剪贴板"不可靠触发                                | App.vue:67 DRAG_INTERACTIVE 追加 `.bubble-row`（沿 `.todo-row` 先例）；验证：live 点击气泡稳定复制不拖窗、空白处拖动不回归                                       | 新增 | Vue 前端                |
| core/src/bubble.rs:42-47 + core/src/commands/bubble.rs:24-28 | 防御性缺口（2/9：参数校验漏 + 无界增长）       | 气泡无长度上限：validate_bubble_text 仅空校验（对比 todo 100 字符上限）；粘贴超长剪贴板（如整篇文章/超大 JSON）全量入库，db 无界膨胀                                                  | 新增 MAX_BUBBLE_TEXT_LEN 常量（气泡 = 短片段语义，建议 2,000–5,000，注释注明可调）入 validate_bubble_text + 超长用例；验证：单测超长拒 + live 粘贴长文本提示可见 | 新增 | 业务状态机 / Tauri 后端 |
| core/src/storage.rs:202                                      | 死代码（5）                                    | count_bubbles 生产零调用（满 5 判定走 list_bubbles().len()，本方法仅测试消费）                                                                                                        | 改 `#[cfg(test)]` 或删除                                                                                                                                         | 新增 | 存储层                  |
| AGENTS.md:38-60 + AGENTS.md:16                               | 规范违反（6 文档一致性）                       | 目录树标注"一期实态"但滞后二期（缺 bubble.rs/whiteboard.rs/commands 扩展/BubblesView.vue/WhiteboardView.vue）；技术栈"通知常驻"行仍为"待一期方案定案后补充"（一期已定案：无通知常驻） | PL006.4 收口统一回改                                                                                                                                             | 新增 | 文档                    |

无 P0/P1。说明：SQL 全参数化、clipboard 仅 Rust 侧不经 ACL、单行表 CHECK 约束、锁序单向——未发现确定性可复现的高危缺陷。

### 二、参考级观察项（记录不修；用户定案延续保留观察）

| 文件:行号                                       | 描述                                                                              | 回落理由                                                                   |
| ----------------------------------------------- | --------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| core/src/commands/bubble.rs:19                  | 非文本/异常剪贴板的错误文案为"剪贴板读取失败：{插件原始错误}"，可能含英文技术细节 | 功能正确、前缀可读；需 live 确认实际文案后定是否打磨——条件豁免（目验后定） |
| ui/components/BubblesView.vue:13-14,43-44,66-67 | copiedTimer/confirmTimer 不随组件卸载清理，切页后单次 ref 写入                    | Vue 3 下无 DOM 写、零危害——永久豁免（无可达危害路径）                      |
| ——                                              | A001 观察项 10 条继续保留观察（本轮代码未触及其触发条件），不重复罗列             | 见附录 A001 第二节                                                         |

### 三、亮点

- 满 5 阈值由 Rust 侧裁决（BubbleSnapshot.remind）防前后端双处漂移——A001 dim 12 教训直接落地
- clipboard 读写仅 Rust 侧薄壳，ACL 面零扩大；气泡/白板核心逻辑全部脱离 tauri 直测内存库
- 关窗挂起快速定位（no-op handler 对照实验排除 flush 本体）与"防抖兜底"定案回退，实现修订全程留痕
- 四轮 TDD 红绿（bubble 纯逻辑/存储/命令/whiteboard 链路），44 项测试全绿；白板单行表 CHECK 约束 + UPSERT 幂等

## 附录 PL008：APP 回归·玻璃基座与通用组件族（2026-09-26 立项）

> 背景：ui-1.0-feature 分支 UI 初版（ui1.0 V0.001–V0.028）已 squash 并入 main（V0.1.1.4）；本组起实验场产物正式落回 ui/。**工作模式（用户定案 2026-09-26）**：不开分支、七组 PL 全部直干 main；**每 PL 不请用户目验**——自验三道闸（Rust TDD 全绿 / IAB 冒烟行为断言 / 截图比对 design 形态忠实）；PL014 后用户做一次总目视验收，问题走 FIX003。每 PL 一条 feat 提交（R+1：V0.1.1.5 起）提交即推送。
> 方案要点：
>
> - **DEV 冒烟基座**：ui/src/dev/mock-invoke.ts——浏览器（vite dev + IAB）环境拦截 invoke，内存态模拟全量命令（种子沿 design/assets/js/state.js）；import.meta.env.DEV 死分支，生产构建消除。这是后续所有 PL 的 IAB 自动验证通道（真 invoke 在纯浏览器不可用）
> - **令牌层落位**：design/glass.css 全量令牌 1:1 → ui/src/styles/glass.css；App.vue 内联玻璃样式改令牌引用
> - **App.vue 骨架重构**：design/index.html 骨架 Vue 化（topbar + 页签挂点 + #board 卡片容器锚点 + 三 page 容器 v-show）；窗口尺寸按实验场卡宽实测调整（tauri.conf.json，置顶/让位/位置记忆不动）；拖动区从全局 mousedown 收敛到 topbar
> - **通用组件族**：TabsBar（uiverse heavy-dragonfly-92 glider + 气泡徽章）、NeonCheckbox（hot-dragonfly-56 全装饰层 + 勾选流光启停）、DelButton（smart-emu-83 + trash-can 双 path 二态盖翻确认全套 V0.025–V0.028 定案形态）、AddBar 重构（REC average-swan-99 + plastic-parrot-88 输入框）
>
> 红线：uiverse 组件忠实移植（类名隔离/颜色令牌化/reduced-motion 守卫）；design 定案参数不自作主张改；mock 通道禁进生产（DEV 死分支）。
> 状态：⏳ 待执行（任务组见 x.progress.md PL008）

## 附录 PL009：APP 回归·横切工厂组合式化（2026-09-26 立项）

> 背景：design/ 四大命令式工厂（glass-bar / board-read / mask-dead / veils）是五滚动容器的统一玻璃语言，转 Vue composables。
> 方案要点：
>
> - **useGlassBar**：玻璃滑杆浮钮组合式（anchor/inset/right opts、scroll+MutationObserver 双挂点、pointerdown 拖拽、textarea 合成 scroll 补发）
> - **useBoardRead**：整板阅读组合式（渐隐带 @property --fade-btm 1s 过渡、▲▼ 三角 hintHost opt（归档板三角住板内随板缩放——V0.028 方案 B）、到底抬带 at-bottom 100ms、半截行 settle、scrollend；skipDuringDrag 互斥）
> - **useMaskDead**：罩死判定（侵入比 38%/迟滞 36%/at-bottom 例外不计底带——V0.027 修复版）+ 灰化 0.5s 双向
> - **useVeils**：浮板开合帘联动（滑杆 veiled 0.22s / 三角 0.15s；归档实例结构性豁免）
> - 样式：ui/src/styles/board-read.css（glass-bar.css 的 board-read/edge-hint 族 1:1）
>
> 红线：算法与参数 1:1 移植 design 实测定案值（六轮拖拽失败教训：机制不经纸上设计不改参数）；遮挡 webview 下 transitionend 不派发——一律显式 duration 定时器。
> 状态：⏳ 待执行（任务组见 x.progress.md PL009）

## 附录 PL010：APP 回归·清单页与数据迁移（2026-09-26 立项）

> 背景：清单页换装实验场形态 + todos 表扩展（实验场新增的 created_at/done_at/note 语义落库）。
> 方案要点：
>
> - **迁移（保用户数据）**：storage.init() 探测列缺失 → ALTER TABLE ADD COLUMN（created_at INTEGER NULL——存量行回填 NULL 不模拟时间；done_at INTEGER NULL；note TEXT NOT NULL DEFAULT ''）；幂等
> - **纯逻辑 TDD**：todo.rs age_level(created_at, now)（None→无提醒 / >48h 红 / >24h 黄，时间源注入）+ validate_note（MAX_NOTE_LEN）
> - **命令扩容**：todo_rename / todo_set_note / todo_list 返回含新字段与 age_level（业务裁决在 Rust）；toggle 置 done_at（true→now / false→NULL）
> - **TodoList.vue 回归**：行模板（NeonCheckbox + t-text + DelButton 二态 + age-alert）、罩死、删除单实例收口与换页回退、TransitionGroup 出入场（显式 duration）
> - **DetailOverlay（todo 模式）**：三板互斥、标题行内编辑（≤12 字）、note 防抖 300ms 保存、单击 180ms 开板/双击行内改标题消歧、飞出原点动画
>
> 红线：迁移必须保存量行（旧库 open 后全字段可读写）；业务零写进 Vue。
> 状态：⏳ 待执行（任务组见 x.progress.md PL010）

## 附录 PL011：APP 回归·归档板（2026-09-26 立项）

> 背景：归档板成品化（V0.015–V0.016 定案形态）+ 三角随板生长（V0.028 方案 B）。
> 方案要点：
>
> - **Rust**：todo_archive_list（done=1 ORDER BY done_at DESC）+ storage.list_done()
> - **ArchiveOverlay.vue**：归档按钮出入场（吸气放大→塌缩 / 粒子）、板揭示（origin 变量注入 + 0.4s 回弹）、title-plate 标题玻璃板、计数与空态、行（勾选退回 / 删除二态）、增量摘除退场、滑杆锚板内 + 三角 hintHost（板内局部坐标 layout 数学）、板开合 450ms settle 重算
> - **勾选入档两拍**：清单勾选 → 300ms 流光主拍 → 塌缩 → 归档计数 +1（板开着清单被遮不可见，板内浮现动画不做——V0.015 定案）；退回 = 塌缩 + 清单 popIn
>
> 红线：归档板点正文 no-op、归档态 note 不可见（V0.015 用户定案）。
> 状态：⏳ 待执行（任务组见 x.progress.md PL011）

## 附录 PL012：APP 回归·气泡页与剪贴板真链路（2026-09-26 立项）

> 背景：气泡页换装（V0.017–V0.020 定案形态）+ 捕获从"示例片段池"升真剪贴板。
> 方案要点：
>
> - **Rust + ACL**：tauri-plugin-clipboard-manager 接入；commands/bubble.rs bubble_capture()（读剪贴板 → validate → 入库 → 快照）/ bubble_copy(id)（写剪贴板）；capabilities 增 clipboard-read/write，构建后核 gen/schemas/acl-manifests.json 权限事实源（ACL 静默拒教训）
> - **BubblesView.vue 回归**：捕获钮双图标 + 占字 1s 反馈、清空二态（宽度动画 / confirming 50% 红 / 悬停感知 2s 超时 / 旁路取消）、行（两行截断 / DelButton / 单击开板双击复制 180ms 消歧——V0.028 对调后语义）、满仓警告红字（>5 出现 + has-warning 容器两档偏移 + 隐区位移 maskShift 6px）、页签徽章计数
> - **全文板（bubble 模式）**：DetailOverlay bubble-mode（单层玻璃 / 标签头摘除 / 只读 textarea / 板内滑杆 + 恒定软边带）
>
> 红线：满 5 阈值 Rust 侧裁决不漂移；剪贴板真链路 IAB 测不了（mock 无真板），UI 行为走 mock、真链路留 PL014 用户总验收。
> 状态：⏳ 待执行（任务组见 x.progress.md PL012）

## 附录 PL013：APP 回归·拖拽排序与持久化（2026-09-26 立项）

> 背景：长按重挂拖拽模型（清单 + 气泡，DRAG_TARGETS 工厂）1:1 移植 + 落库。
> 方案要点：
>
> - **迁移（Rust TDD）**：todos/bubbles 各加 sort_order INTEGER 列（存量回填 = 现序号）；storage reorder_todos/reorder_bubbles(ids) 事务（逐行 UPDATE，id 集合与全量一致性校验防丢行）；list 排序改 ORDER BY sort_order
> - **useDragReorder composable**：DRAG_TARGETS 注册表 1:1 移植 drag-reorder.js（长按 250ms / 抖动 6px / 重挂 #board / ghost / shifting / 边缘自动滚动 32px 带 10px 每帧 / moved 3px 门槛 / 卡内钳制 12px / 落点点击抑制 350ms）；落点 commit 调 invoke reorder；**拖拽期冻结列表响应式重渲染**（engaged 守卫 watch——重挂 DOM 与虚拟 DOM 打架防线）
>
> 红线：算法参数六轮失败教训——不改一个数；拖拽中不触发全量重绘（增量摘除同族语义）。
> 状态：⏳ 待执行（任务组见 x.progress.md PL013）

## 附录 PL014：APP 回归·白板设置收口与总验收（2026-09-26 立项）

> 背景：尾组收口 + 全量回归 + 交用户总目视验收。
> 方案要点：
>
> - **WhiteboardView 回归**：board-shell 壳 + textarea 透明填壳（墨迹溶解与卡底分离——V0.021 定案）+ 整板阅读第 5 实例 + 防抖保存沿用
> - **SettingsOverlay.vue**：实验场设置板形态；气泡上限步进（1~20）落 configs/config.json 扩展字段 max_bubbles（settings.rs 读写），bubble 阈值改读配置（默认 5）
> - **全量回归**：门禁四件套 + IAB 全交互走查脚本（清单/归档/气泡/白板/拖拽/罩死/详情全链路断言）+ tauri dev 真窗口 exe 探针（标题/存活/尺寸自动）+ AGENTS 状态头与本文档状态收口回写
> - **用户总目视验收**：真窗口 DWM 玻璃可读性 / 常驻功耗 / 剪贴板真链路 / 全交互走查——问题走 FIX003
>
> 红线：验收不达标不宣布完成；观察项沿 y.problems 登记。
> 状态：⏳ 待执行（任务组见 x.progress.md PL014）
