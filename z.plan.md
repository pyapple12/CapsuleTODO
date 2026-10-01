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
> 状态：✅ 已完成（2026-09-28，任务组见 x.progress.md PL014；设置板 UI 前置于 PL012.6/PL012.5，持久化随本组落位）

## 附录 A003：全量代码审计报告（第3轮，2026-09-28）

> 范围：core/ 全部 .rs + Cargo.toml + tauri.conf.json + capabilities + ui/ 全部 .ts/.vue/.css + package.json + vite.config.ts；不含 design/（用户指定排除）。方式：三路并行只读通读（Rust 业务组 / Tauri 集成组 / Vue 前端组）+ A002 修复项回归复核 + 主会话全局 grep。
> 状态：📌 待修复（FIX003 任务清单见 x.progress.md）

### 零、上轮（A002）修复复核清单

| A002 条目                       | 现状                      | 证据                                                                  |
| ------------------------------- | ------------------------- | --------------------------------------------------------------------- |
| 气泡无长度上限                  | ✅ 仍在位                 | core/src/bubble.rs:12（MAX_BUBBLE_TEXT_LEN=2000）+ 校验与边界测试     |
| storage.rs count_bubbles 死代码 | ✅ 已删                   | 全仓 grep 零命中                                                      |
| 气泡行拖动劫持                  | ✅ 方案替代               | 拖动收敛 topbar（data-tauri-drag-region），交互区无白名单依赖         |
| BubblesView 计时器卸载清理      | ✅ 已补                   | onUnmounted 清 4 个 timer                                             |
| AGENTS 目录树滞后               | ⚠️ 本轮审计收口时顺手修正 | settings.rs max_bubbles 语义 / commands/settings.rs / ui/src 结构已补 |

### 一、P0-P3 修复清单

#### P1（确定性缺陷，真机必现）

| 文件:行号                   | 类型     | 描述                                                                                                                                                                                                                          | 建议                                                                                                       | 性质                         | 影响面              |
| --------------------------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- | ---------------------------- | ------------------- |
| core/src/lib.rs:112-129     | 1 正确性 | whiteboard_load/whiteboard_save 从未注册 invoke_handler（generate_handler 16 条无 whiteboard）。前端常驻挂载即 invoke load（恒失败静默回空串）、800ms 防抖保存恒失败。IAB 全绿漏网根因 = mock-invoke 有这两条而真机注册面没有 | 注册面补两行；固化"新命令必须 generate_handler + 真机冒烟"纪律                                             | 新增（存量，PL005.2 起即缺） | 白板页签全功能      |
| core/src/storage.rs:174-180 | 1 正确性 | 存量气泡迁移回填方向反：V0.1.1.10 前展示序 = id 倒序（新在前），PL013 迁移按 id 升序回填且列表改升序输出 → 老库升级后存量气泡整体倒序；注释自称"历史语义由回填保持"与实现矛盾（todos 侧方向正确反衬笔误）                     | 回填 SQL 改 b2.id > bubbles.id + 修 sort_order_migration.rs 断言与两处注释；只影响未拖拽过的存量库首次迁移 | 新增（PL013 引入）           | 存储层 / 业务状态机 |

#### P2（高，交互正确性）

| 文件:行号                                                           | 类型                      | 描述                                                                                                                                                                   | 建议                                                            | 性质            | 影响面              |
| ------------------------------------------------------------------- | ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- | --------------- | ------------------- |
| ui/src/components/ArchiveOverlay.vue:101,118-134                    | 1 正确性（V0.1.2.1 回归） | 开板 listKey++ 重建 UL 后，滑杆/整板阅读仍绑 detached 旧 UL（!boardRead 守卫不重挂、无 rebuildObserver）→ 第二次开板起：三角/抬带/收尾带全死、滑杆消失、原生滚动条裸露 | 重建后 unmount+重挂（沿 TodoList 模式），或按 UL 元素身份判重挂 | V0.1.2.1 批引入 | 归档板滚动体验      |
| ui/src/components/DetailOverlay.vue:128-149                         | 1/8 正确性                | 防抖保存定时器回调触发时刻读 props.todo!.id：300ms 内关板 → null.id 未捕获 TypeError 草稿丢；300ms 内切条目 B → A 的未决保存被取消（静默丢失），B.note 恰同则交叉污染  | 调度时闭包捕获 id；切源/关板先 flush 未决保存                   | 新增            | 详情板编辑链路      |
| ui/src/components/TodoList.vue:100-115 + ArchiveOverlay.vue:232-239 | 1/8 正确性                | 共享单定时器：300ms 内连续勾选 A、B → clearTimeout 取消 A 的 emit(changed) → A 行带已勾视觉滞留清单，再点 A 会反向翻转 DB（UI/DB 分叉）。归档 restore 同款             | per-item 定时器 Map 或被取消时立即补偿 emit                     | 新增            | 清单勾选 / 归档退回 |
| ui/src/composables/useVeils.ts:33,39 + ArchiveOverlay.vue:284       | 1/5 正确性                | 归档豁免是死条件：全仓无 id="archive-list" 元素（TransitionGroup 未挂 id）→ 特判永不命中 → 归档板开板时自家滑杆/三角被 .veiled 隐藏，与"归档自家豁免"设计注释相反      | TransitionGroup 补 id="archive-list" 一行                       | 新增            | 归档板滚动形态      |
| ui/src/composables/useVeils.ts:13 + DetailOverlay.vue 全文          | 1/5 正确性                | bindOverlayState("detail") 全仓零调用、textarea 缺 id="detail-note" → 详情板开板不吃帘，.edge-hint(z4) 盖过 .detail-overlay(z3)——清单可滚时 ▲▼ 在详情板上方闪烁        | DetailOverlay 注册 bind + textarea 补 id                        | 新增            | 详情板视觉          |

#### P3（清理/规范/防御收尾）

| 文件:行号                                                                                                                           | 类型             | 描述                                                                                                                        | 建议                                                      |
| ----------------------------------------------------------------------------------------------------------------------------------- | ---------------- | --------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| ui/src/components/WhiteboardView.vue:67 + App.vue                                                                                   | 1/5              | flush 已 expose 但 App 零调用——关窗路径 800ms 防抖窗口内输入必丢                                                            | beforeunload 兜底 flush 或与 Rust CloseRequested 联合方案 |
| ui/src/components/DetailOverlay.vue:151-156                                                                                         | 1                | flushPending 只 clearTimeout 不保存（名实不符），close 不调用——防抖窗口内编辑关板即丢                                       | flushPending 改真保存 + close 首行调用                    |
| core/src/settings.rs:17-29 + core/src/lib.rs:92                                                                                     | 2 防御           | load 路径 max_bubbles 无钳制（手改 config 0/9999 直进运行时）；结构体注释"越界兜底"对 max 不成立                            | load 后 clamp(1,20)                                       |
| core/src/commands/settings.rs:40                                                                                                    | 13 错误策略      | settings_set 越界静默钳制未登记容错白名单                                                                                   | AGENTS 白名单补第五条或改严格报错                         |
| core/src/lib.rs:160                                                                                                                 | 13 错误策略      | 关闭时 settings 锁中毒 → 以默认 5 覆盖落盘（吞用户自定义值）                                                                | 中毒分支跳过保存落日志                                    |
| core/src/commands/mod.rs:106-134                                                                                                    | 10 可测试性      | 契约测试缺 CommandError::Settings 变体断言                                                                                  | 补断言                                                    |
| core/src/bubble.rs:9 + core/src/settings.rs:11 + core/src/commands/settings.rs:47-50                                                | 5/12 死代码/双源 | MAX_BUBBLES 死常量 + 默认值双源（5 两处各写）+ default_max_bubbles() 死代码注释失实                                         | 删常量或 DEFAULT_MAX_BUBBLES 单一来源                     |
| core/capabilities/default.json:8-9                                                                                                  | 12 配置死键      | clipboard-manager 两 ACL 权限无消费方（剪贴板全走 Rust 侧）                                                                 | 删除或注释预留                                            |
| core/src/storage.rs:112-153                                                                                                         | 4 重复           | PRAGMA 列探测逻辑同文件三份                                                                                                 | 抽 column_set 助手                                        |
| core/src/storage.rs:372,413                                                                                                         | 13 错误策略      | reorder 回滚 let _ = 吞错（白名单未登记；settings.rs:71 先例是落日志）                                                      | eprintln 落日志                                           |
| core/src/fullscreen.rs:74                                                                                                           | 4 规范           | fn 内 use（孤例）                                                                                                           | 上提文件头                                                |
| AGENTS.md:15,32,48 + ui/src/components/App.vue:30                                                                                   | 6 文档           | DWM 移除后四处文档/注释仍描述 Acrylic 现行机制（状态头已改，其余未改）                                                      | 按 2026-09-28 定案改写                                    |
| ui/src/components/BubblesView.vue:185-192                                                                                           | 8 并发           | 清空集体退场 300ms 嵌套 setTimeout 未入卸载清理清单                                                                         | 句柄入 onUnmounted                                        |
| ui/src/components/TodoList.vue:293,300 + ui/src/composables/useDragReorder.ts:160 + ui/src/components/NeonCheckbox.vue:13-16        | 5 死代码         | 恒假 class 绑定 / @toggle 死绑（勾选已走坐标分流）/ 自引用空模块声明 / onClick 保留位                                       | 删除                                                      |
| ui/src/components/BubblesView.vue:41 + ui/src/components/SettingsOverlay.vue:142 + core/src/bubble.rs:36                            | 6 文档           | 注释漂移三处（"会话内有效再议"×2 已持久化 / "倒序列表"已改升序）                                                            | 改注释                                                    |
| ui/App.vue:44 + ui/src/components/TodoList.vue:191 + ui/src/components/DelButton.vue:61,71 + ui/src/components/DetailOverlay.vue:87 | 6/14/8 规范      | 徽章计数内联类型未复用 types.ts / stopMask computed 副作用（宜 watchEffect）/ !== undefined 判空两处 / close 缺 isOpen 早退 | 小改                                                      |
| core/src/commands/todo.rs:19 vs :48                                                                                                 | 4 重复           | todo_add 存原文不 trim（rename/bubble 均 trim 落库）——同字段两套落库语义，API 直调可绕前端 trim                             | add 统一 trim 落库                                        |
| core/src/commands/todo.rs:218-232                                                                                                   | 10 可测试性      | age_level 命令层测试断言力≈0（自陈无法构造跨阈值场景）                                                                      | open_with_now 双时钟补真断言                              |

### 二、参考级观察项（豁免，含回落理由）

1. lib.rs default_position 300×400 与 tauri.conf.json 双处——有注释依据且"尺寸固定不入配置"定案在位（改尺寸须两处同步）。
2. lib.rs:90 {db:?} 路径进启动错误消息——本地桌面应用助排障，豁免。
3. fullscreen.rs 轮询 1000ms——有注释依据。
4. fullscreen 轮询线程不随窗口关闭退出——进程生命周期线程。
5. lib.rs:133 setup 内 expect——装配期配置不变量断言，非业务散落。
6. 最小化 -32000 坐标入 config.json——启动越界兜底自愈。
7. bubble.rs 双锁 SQLite 读——低频 UI 读锁内耗时微小。
8. BubbleError → CommandError::Clipboard 复用——气泡文本唯一来源即剪贴板，语义可容。
9. examples/seed_data.rs 写真实 data/todo.db——注释完备的开发期注入工具。
10. 单实例 window.show() 对最小化窗行为——需验证，路径实际不可达。
11. 常驻三板 nextTick 挂载未持 composable destroy——无卸载语义（改 v-if 须补）。
12. DelButton pulse 强制回流连点重播——需验证，理论缺口。
13. 主题三态不持久化——产品决策观察（与 max_bubbles 已持久化不对称）。
14. remind >= 与前端警告 > 语义分歧——remind 前端不消费，未来接横幅须统一。
15. 300×400 数据量小列表无虚拟化、类名 camelCase/kebab 混用（uiverse 保形）——豁免。

### 三、亮点

SQL 全参数化零拼接；迁移逐列幂等可断点续迁；时间源全注入零 sleep；锁序纪律全路径合规无反向；types.ts 与 serde 契约零漂移；mock-invoke 22 条命令语义与 Rust 注册面一致（唯覆盖缺口见 P1）；delConfirmBus 注册注销三处对称。

## 附录 PL015：全局气泡热键（2026-09-30 立项）

> 背景：用户需求——任何应用里复制文本后按全局热键，自动捕获剪贴板入气泡；热键组合可在设置板更改。路线 B 拍板（2026-09-30）：裸 Win32 RegisterHotKey（fullscreen.rs user32 直连先例，零新依赖），不走官方 global-shortcut 插件。
> 方案要点：
>
> - **热键解析器（纯逻辑 TDD）**：`core/src/hotkey.rs`——"Ctrl+Alt+C" ↔ HotkeyCombo{mods, vk}（MOD_* 常量与 Win32 对齐），parse/to_display 往返恒等，非法组合 thiserror 报错
> - **注册运行时**：RegisterHotKey + GetMessageW 循环线程（fullscreen.rs 先例），WM_HOTKEY → AppHandle.run_on_main_thread 投递捕获流程；reregister = Unregister + 重组装线程（PostThreadMessage WM_QUIT）；注册失败（热键被占）Err 上抛不裸奔（**实现注记（FIX004.9 补）**：实际未走 run_on_main_thread——捕获流程仅读剪贴板 + 入库，AppContext 有锁保护、剪贴板插件跨线程安全，WM_HOTKEY 在热键线程直调行为等价，沿 PL003.3 先例）
> - **捕获流程（与气泡页同一条数据通路）**：Rust 侧读剪贴板 → bubble 校验 → add_bubble 排头插入 → 满 5 走现有满额数据流（气泡页自会呈现）；全程失败落日志静默——贴"无系统通知常驻"一期定案，桌面常驻面板即反馈
> - **重复内容去重（2026-09-30 增补）**：气泡页手动捕获与热键捕获**同一裁决**——内容与现存气泡重复时拒绝入库（数据层查重，两入口天然同规）；返回 Duplicate 语义供前端提示，提示形态定案 = 捕获钮占字态复用（成功"✓已捕获" clipboard-check 图标 / 重复"✕重复捕获，无效！"否定图标同款风格，50% 紫禁点 1s 同款），热键路径天然静默
> - **设置板快捷键行**：当前组合显示 + 录制态（keydown 捕获修饰+主键，Esc 取消）→ 落 config.json 扩展字段 bubble_hotkey（默认 Ctrl+Alt+C，旧文件回填默认，非法值静默回默认）→ 触发重注册；冲突红字 + 回退旧热键
>
> 红线：解析器纯逻辑零 tauri 依赖（TDD 先红后绿）；注册失败必须可见回退禁止裸奔失效；测试数据自清理禁污染用户库；AGENTS 容错白名单登记两条（非法热键回默认 / 热键缺失不阻断启动）。
> 状态：🚧 已立项未开工（任务组见 x.progress.md PL015，7 条三阶段；完成时 commit `feat: V0.1.4.0，全局气泡热键` minor 推进）

## 附录 A004：全量代码审计报告（第4轮，2026-09-30）

> 范围：core/ 全部 .rs + Cargo.toml + tauri.conf.json + capabilities + ui/ 全部 .ts/.vue/.css + package.json + vite.config.ts。方式：三路并行只读通读（Rust 业务组 / Tauri 集成组 / 前端组）+ A003/FIX003 修复项回归复核（grep 批量 + 子代理 git diff 补盲）。基线 commit 4c0ae72（V0.1.4.0）。
> 状态：📌 待修复（FIX004 任务清单见 x.progress.md）

### 零、上轮（A003/FIX003）修复复核清单

| A003 条目                                                                                                                                                                                                                   | 现状                       | 说明                                                                       |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------- | -------------------------------------------------------------------------- |
| FIX003 全部 18 项修复（whiteboard 注册 / 迁移方向 / column_set / 归档滑杆重建 / 详情闭包捕获 / per-item 定时器 / archive-list id / load 钳制 / 锁中毒跳过 / Capabilities 卫生 / todo_add trim / 契约测试 Settings 断言 等） | ✅ 16 项完好在位           | 批量 grep + 三路子代理 git diff 双重复核                                   |
| BubblesView 清空嵌套 setTimeout 入卸载清理                                                                                                                                                                                  | ❌ 遗留未修（FIX003 漏派） | BubblesView.vue:220 句柄丢弃；危害面补充 = 页签徽章不归零                  |
| WhiteboardView flush 接线                                                                                                                                                                                                   | ❌ 遗留未修（FIX003 漏派） | App 零调用，关窗防抖窗口输入必丢                                           |
| 本轮新增代码复现已修模式                                                                                                                                                                                                    | ⚠️ 2 处                    | settings.rs:75 回滚 `let _` 吞错、lib.rs:175 fn 内 use（均在 FIX004 清单） |

### 一、P0-P3 修复清单

#### P2（高）

| 文件:行号                                     | 类型       | 描述                                                                                                                                                                              | 建议                                                  | 性质 | 影响面          |
| --------------------------------------------- | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- | ---- | --------------- |
| ui/src/components/DetailOverlay.vue:141-174   | 1 正确性   | 防抖"改回原文"残留：watch 相等分支直接 return 不清 pending/Timer——300ms 内改回原文，到点仍把中间草稿写库（UI 原文 / DB 中间值分叉）                                               | 相等分支同时清 renamePending/renameTimer（note 同构） | 新增 | 详情板编辑链路  |
| ui/src/components/SettingsOverlay.vue:155-158 | 1 正确性   | 录制态遗留吞键盘：录制中收板/切页签不复位 recording——document keydown capture 持续吞一切按键                                                                                      | close/toggle 分支补 hotkeyRecording=false             | 新增 | 设置板/全局键盘 |
| ui/src/components/BubblesView.vue:130-136,312 | 1/8 正确性 | duplicate 占字永久卡死：复制占字回调只复位 copied 不复位 duplicate——捕获钮永久禁用                                                                                                | 复制回调同复位 duplicate（或收敛单占字状态机）        | 新增 | 气泡捕获钮      |
| ui/src/composables/useDragReorder.ts:279-286  | 1 正确性   | 单行拖拽还原缺 forceRemount（需验证）：单行长按松手后行从 DOM 消失（收场路径已补、还原路径漏挂），数据无损切页自愈                                                                | 还原路径补 forceRemount                               | 新增 | 拖拽排序        |
| ui/src/dev/mock-invoke.ts:222-289             | 10/11      | mock 注册面落后真机：热键两命令缺（IAB 恒读取失败）、bubble_capture 未返回去重后 {status,item} 形状——PL015 前端分支冒烟零覆盖（A003 whiteboard 教训反向同构）                     | mock 补两命令 + capture 改 BubbleCaptureOutcome 形状  | 新增 | IAB 冒烟通道    |
| ui/src/components/TodoList.vue:290-292        | 1 正确性   | 删末条待办无动画：清单页仍 v-if/v-else 空态互斥——与归档/气泡同根修复的漏网处（ TransitionGroup 卸载 leave 跳过）                                                                  | 同款共存渲染改造（空态延后 after-leave）              | 新增 | 清单页          |
| core/src/hotkey.rs:204-243                    | 8 并发     | 热键线程握手竞态（微秒级窗口非确定性）：tid 落位晚于结果回传 / PostThreadMessageW 返回值未检 / 500ms 超时带病推进 / 退出清零无条件覆盖——最坏旧热键残留生效 + 僵尸线程（重启自愈） | tid 提前落位 / 检查返回值 / CAS 清零 / 超时报错不推进 | 新增 | 设置板换热键    |

#### P3（清理/规范/防御，合并后 14 条）

| #   | 文件:行号                                                                  | 类型 | 描述与建议                                                                                                                                       |
| --- | -------------------------------------------------------------------------- | ---- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | core/src/commands/settings.rs:75                                           | 13   | 热键回滚落库 `let _ =` 吞错且注释自称"仅落日志"实无日志（FIX003 同款重现）——改 eprintln                                                          |
| 2   | core/src/hotkey.rs:4 等 + x.progress/z.plan 表述                           | 6    | 线程模型三处文档失实：注释称"主线程执行/run_on_main_thread"实为热键线程直调（含 `let _ = app` 死参）——按 PL003.3 先例补实现注记或改实现          |
| 3   | core/src/commands/mod.rs:128-162                                           | 10   | 契约测试缺 Hotkey 变体断言（A003 同款缺口在新变体复现）——补断言                                                                                  |
| 4   | core/src/hotkey.rs:16-31,177-183                                           | 5/12 | HotkeyError/HotkeyCombo 的 Serialize 派生死代码（实际走 From→String）+ extern 块缺 `#[link(name="user32")]`（需验证）——删派生 + 补 link 对齐先例 |
| 5   | core/src/lib.rs:175                                                        | 4    | cfg 块内冗余 `use tauri::Manager as _`（文件头已全局导入）——删                                                                                   |
| 6   | AGENTS.md:5,47-58 + core/src/settings.rs:77                                | 6    | AGENTS 三处失实：状态头停 V0.1.2.3、目录树缺 glass_backdrop.rs、settings.rs 注释"白名单⑥"编号漂移——状态头推进/目录树补行/注释改⑦                 |
| 7   | core/examples/seed_data.rs:148-165                                         | 4/13 | 去重漏接：种子池 3 条轮转注入 6 条必产 3 次 Duplicate 静默丢弃、摘要行失真（默认参数 100% 复现，仅开发工具）——match Duplicate 计数或扩池         |
| 8   | core/src/storage.rs:13,40 + :1                                             | 6    | use 夹置（TodoItem 导入被 BubbleAddOutcome 块隔开）+ 模块头仍只写 todos 表——上提相邻 + 头部补全三表职责                                          |
| 9   | core/src/storage.rs:27-37 + commands/bubble.rs:41-46                       | 5    | 两枚举 `added_item()`（Duplicate panic）常驻 pub 零生产调用——收敛 `#[cfg(test)]`（两组件统一定夺）                                               |
| 10  | core/src/settings.rs:76-86 vs lib.rs:97-98                                 | 4    | 载入规范化不对称：热键回默认在 load 内、max_bubbles 钳制在装配点（无现行错值）——钳制挪入 load 收敛单点                                           |
| 11  | core/src/settings.rs:22-28                                                 | 6    | 两个 serde default 私有函数缺 `///` 文档注释——各补一行                                                                                           |
| 12  | ui/src/components/WhiteboardView.vue:64 + App.vue                          | 1/5  | （零节遗留）flush 接线缺失：关窗防抖窗口输入必丢——App 持 ref 关窗兜底或 Rust CloseRequested 联合方案                                             |
| 13  | ui/src/components/BubblesView.vue:220,45 + ArchiveOverlay.vue:213          | 8    | 清空嵌套 setTimeout 未入卸载清理（遗留，徽章不归零危害面）+ showEmpty 420ms 兜底定时器同纪律缺口（新增）——句柄入 onUnmounted                     |
| 14  | ui/src/components/AddBar.vue:21 + ui/App.vue:132-143                       | 4/9  | added/changed 成对 emit 使新增双跑刷新（6 invoke 幂等但翻倍）——onTodoAdded 不调 onListChanged 或 AddBar 只发 added                               |
| 15  | ui/src/styles/settings.css:410 + ui/src/components/BubblesView.vue:125-128 | 3/11 | 单源复用两处：hotkey-error 硬编码 #e5484d 改 var(--danger)；capture 内联 invoke 泛型改 BubbleCaptureOutcome（types.ts 镜像已在）                 |

#### 性质说明

P2 七条与 P3 多条为 **PL015 新增代码引入**（热键线程/去重前端状态机/录制态边界/mock 漂移——新功能引入面典型形态）；两条为 **A003/FIX003 漏派遗留**；其余为规范/文档卫生。

### 二、参考级观察项（豁免，含回落理由；完整版见三路子代理原始输出）

1. 假行 leave 模拟证归档/气泡动画 CSS 链路健康——两个已修 bug 的机制层无恙。
2. useThresholdDrag 的 mousedown preventDefault 对 titleParticles mousemove 无影响——理论缺口经实测排除。
3. types.ts ↔ serde tagged 契约（BubbleCaptureOutcome）零漂移——真实链路健康，漂移仅在 mock 与内联用法。
4. 录制态按旧热键组合会同时触发一次捕获（系统 RegisterHotKey 先于 webview）——系统级固有，行为怪但无害。
5. 热键线程/500ms 等待常量/单次使用模块常量——有注释依据或单次使用，豁免。
6. glass_backdrop Win10 pre-22H2 聚焦翻转日志刷屏——平台基线 Win11 定案，失败维持前态即白名单①行为。
7. ON_HOTKEY 锁中毒静默恢复 / 启动读设置锁中毒回默认热键——装配期不可达理论缺口（需验证）。
8. TodoList 删末条若定案"清单保留原行为"请落注释豁免——否则按 P2 修复（见清单）。
9. BubblesView/ArchiveOverlay 快速双击归档钮 450ms settle 提前取中间态几何（≤中，需验证）。
10. 热键 reregister 命令同步阻塞 ≤1s——设置动作低频可接受。

### 三、亮点

三路交叉确认：SQL 全参数化、时间源注入、锁序纪律无反向、types.ts↔serde 主契约零漂移；FIX003 十八项修复十六项完好在位（两项漏派如实列出）；PL015 解析器往返恒等 10 用例、去重三裁决用例、热键回滚回退设计（失败回滚旧热键落库+重注册）结构正确；归档/气泡删末条动画修复的 CSS 链路经假行模拟证实健康。

## 附录 PL016：热键捕获两改进（2026-09-30 立项）

> 背景：PL015 全局热键落地的两个余量改进（用户 2026-09-30 拍板立项）：①**失焦进气泡实时刷新**——热键入库后 app 失焦、气泡页可见时不切页即现新泡；②**圈选直达**——任意应用选中文字按热键直接捕获（无需先 Ctrl+C），实现 = 模拟 Ctrl+C 两段式（快照 → 合成 → 等变化 → 读新 → 恢复）。
> 用户定案两边界：无选区/复制失败（300ms 剪贴板无变化）= **静默**（不回退捕获旧剪贴板——换了剪贴板内容的场景语义模糊）；**不设设置板开关**（默认常开，行为确定性优先）。
> 方案要点：
>
> - **失焦刷新（小）**：`commands/bubble.rs` quiet 路径入库 Added 后 `app.emit("bubble-changed", ())`（emit 失败落日志，热键反馈静默定案不变；Duplicate/Err 不发）；`ui/App.vue` onMounted listen → refreshBadge（徽章实时）+ `BubblesView.vue` onMounted 同款 listen → refresh（组件 v-if 挂载 = 仅气泡页激活时刷新），两处 onUnmounted 清；ACL 零改动（listen 权限经 window-focus 实证 core:default 覆盖）
> - **圈选直达（大）**：新 `core/src/capture.rs`（cfg windows）——`snapshot_text() -> Option<String>` / `restore_text()`（复用 clipboard 插件读写，不裸 Win32；恢复失败落日志不阻断）+ `synthesize_ctrl_c()`（user32 keybd_event 直连：VK_CONTROL down → 'C' down/up → CONTROL up，键间 10ms；extern 补 #[link(name="user32")] 对齐 fullscreen/hotkey 先例）+ `wait_clipboard_change(before, timeout, now, read)` 纯逻辑状态机（10ms 步进轮询、注入时间源与读函数 TDD：立即命中/超时静默/变化后命中/空文本忽略）；`commands/bubble.rs` 热键路径编排：快照 → 合成 → 等 300ms → 命中 = bubble_capture_core 入库（校验/去重同规）+ 恢复原剪贴板 + emit bubble-changed（与①同出口）；未命中 = 静默（原剪贴板未变无需恢复）
> - **终端边界（条件条目）**：cmd/conhost 无选区时 Ctrl+C = 中断信号风险，实测若实证成立则 capture.rs 加前台控制台守卫（GetForegroundWindow + GetClassName 分治 ConsoleWindowClass 与 Windows Terminal CASCADIA_HOSTING_WINDOW_CLASS）——命中控制台 = 跳过合成直接静默；实测无风险则记豁免
>
> 红线：合成按键向系统发真实 Ctrl+C，live 验证须用户明示授权或亲自配合（实测矩阵：Windows Terminal 有选区 / cmd conhost / 管理员提权前台窗 UIPI 拒绝→静默自愈 / 无选区静默 / 剪贴板占用重试 3×10ms）；剪贴板恢复失败落日志不阻断（容错白名单登记候选）；测试零污染用户库；热键线程编排延续 PL015.4 直调实现注记。
> 状态：✅ 已完工（2026-09-30，V0.1.5.0 minor 推进；实施记录与 live 实测定案见 x.progress.md PL016 组——圈选直达两段式 + 修饰键残留合成 keyup 清理 + 终端守卫 + 失焦实时刷新；live 矩阵 B/C 场景过、A 场景根因（修饰键残留）定位修复后用户回执通过）

## 附录 PL017：置顶开关与贴边吸附（2026-09-30 立项）

> 背景：用户需求两开关——①**窗口置顶**：当前置顶为硬编码（tauri.conf.json alwaysOnTop + fullscreen.rs 让位线程每秒挂回），需可开关（设置板日夜切换下方）；②**贴边吸附**：拖动到屏幕边缘 ≤50px 内松手自动贴边停驻（落位距边 5px），开关在置顶下方。
> 用户定案三点（2026-09-30）：`snap_to_edge` **默认开**；吸附时机 = **A 松手吸附**（Moved 事件 300ms 防抖，连发停止即松手后吸附一次，不碰系统拖动循环零抖动风险）；置顶关闭 → **让位线程直接休眠**（板子已是普通窗口，全屏自然盖住，不再每秒挂回置顶）。
> 方案要点：
>
> - **settings.rs**：WindowSettings 加 `always_on_top: bool`（serde default = default_always_on_top() → true，现状即置顶老配置零迁移）+ `snap_to_edge: bool`（default → true）+ Default impl 同步 + 缺字段回填测试；常量 SNAP_THRESHOLD_PX = 50 / SNAP_GAP_PX = 5（吸附参数单一来源）
> - **snap.rs（新）**：纯函数 `snap_position(win_x, win_y, win_w, win_h, mon_x, mon_y, mon_w, mon_h) -> (i32, i32)`——四边距离 ≤ 阈值判定（角落组合吸附、未命中原样返回、已在吸附位幂等返回同值）；TDD 七用例（左/右/上/下/角落组合/51px 不动/幂等）
> - **commands/settings.rs**：`settings_get/set_always_on_top`（set = 锁 ctx 更新 → 落盘 → 主窗 set_always_on_top 即时生效；核心抽 *_core 直测临时路径）+ `settings_get/set_snap_to_edge`（纯落盘，生效在事件层每轮读 ctx 天然即时）；mock-invoke.ts 四 handler 双注册
> - **lib.rs 接线**：setup 启动按 config 应用置顶；`WindowEvent::Moved` 吸附状态机——LAST_MOVED 时间戳 + DRAG_ACTIVE/DRAG_SNAP_PENDING 原子标志：Moved 更新时间戳（间隔 ≤300ms 置拖动中）+ pending 防重 spawn 单次检查线程（350ms 后确认连发停了才吸附）；do_snap = current_monitor 取屏矩形 → snap_position → 与当前位不同才 set_position（**幂等防环**：吸附位 5px 仍在阈值内，不判等会死循环）；启动恢复的单次 Moved 不构成连发 → 默认落位 40px 天然不被吸（无 boot 标志）
> - **fullscreen.rs**：让位线程每轮开头读 ctx.always_on_top——false 即 continue 休眠本轮（锁失败按白名单②维持前态落日志）；置顶开 = 行为不变
> - **SettingsOverlay.vue + App.vue + settings.css**：日夜切换行下依次插"窗口置顶"（desc：关闭后板子允许被其他窗口遮挡）与"贴边吸附"（desc：拖到屏幕边缘 50px 内自动贴边停驻）两行——通用 .toggle-switch（轨道+圆钮 accent 令牌，与 theme-switch 日月组件区分）；沿 maxBubbles 同构（App.vue 持 ref 启动拉取传 prop，组件内 toggle invoke 落库失败可见）
>
> 红线：吸附事件层读设置锁失败跳过吸附落日志（容错白名单候选，登记待实施时定）；吸附必须幂等防 Moved→snap→Moved 死循环；启动默认落位 40px 不被吸附侵蚀；watcher 休眠不得影响全屏让位恢复（置顶开时行为零变化）；测试零污染用户库。
> 状态：✅ 已完工（2026-09-30，V0.1.6.0 minor 推进；live 四轮迭代定案：±阈值过冲窗口、视觉矩形校准（像素实测隐形边 8px）、左键松手判定（悬停/慢速拖不误吸）、150/100ms 防抖；实施记录见 x.progress.md PL017 组）

## 附录 PL018：托盘图标与悬浮预览（2026-09-30 立项）

> 背景：用户需求——主窗不占任务栏，启动后仅托盘小图标；**左键**收起/展示主窗；**右键**菜单列设置项（贴边吸附/窗口置顶，带勾选态）；**hover 托盘**出现悬浮列表显示前五条 todo（带打勾，可直接勾选落库）。
> 用户定案（2026-09-30）：主窗无边框无 X、**无退出方式且不需要**（托盘菜单不加退出项；未来若有关闭入口也是隐藏语义）；预览窗 = **透明玻璃小窗**（与主窗同风格）；**hover 技术验证不过则继续讨论，不设兜底**（不做左键弹出退化路线）。
> 方案要点：
>
> - **主窗托盘化（低风险）**：tauri Cargo features 加 `tray-icon`；主窗 `skipTaskbar: true`；setup 建 `TrayIconBuilder`（图标 = 现有 app icon 占位，后续会换）——左键事件 toggle 主窗（show + set_focus / hide）；右键菜单 = Menu：显示主窗 + CheckMenuItem"贴边吸附"/"窗口置顶"（**无退出项**，用户定案）；菜单勾选态初始从 AppContext 读
> - **开关三入口同步**：settings 两 set 命令（PL017 已有）成功后 `emit("prefs-changed", { always_on_top, snap_to_edge })`；托盘菜单 CheckMenuItem 监听事件刷新勾选态（set_checked），菜单勾选变化 → 复用 settings 命令核心落库 + 即时生效（与设置板三入口同源，单一事实源 = ctx + config.json）
> - **hover 预览窗（中风险，先 spike）**：`WebviewWindowBuilder` 建 label `tray-preview` 第二窗（透明 / 无边框 / skipTaskbar / 不抢焦点 show）；前端按窗口 label 分流（App.vue 挂载时 `getCurrentWindow().label` 判断，preview 窗渲染 TrayPreview 组件：`todo_list` 前 5 条 + NeonCheckbox 打勾 → `todo_toggle` 落库 → `emit("todo-changed")` 驱动主窗刷新链）；托盘 `TrayIconEvent::Enter` 拿托盘 rect → 预览窗定位托盘上方（越界翻转到下方）→ show；**显隐宽限逻辑** = 托盘 Leave 后延迟隐藏 + 预览窗 mouseenter 取消 / mouseleave 延迟（经典悬浮窗竞态处理）
> - **spike 条目（先行）**：Windows 托盘 Enter/Move/Leave 事件可靠性验证（muda 事件在真机是否稳定触发、rect/position 是否可用）——**结论不过则停止实施回讨论**（用户定案不设兜底）
>
> 红线：主窗 hide 后热键捕获/气泡入库等后台链路不受影响（show 时数据自刷新）；预览窗打勾与主窗数据单向同步（emit 驱动，禁止双写）；托盘菜单勾选态以 ctx 为唯一事实源；预览窗不抢焦点（show 不 steal focus，打勾点击才聚焦）；测试零污染用户库。
> 状态：✅ 已完工（2026-10-01，V0.1.7.0 minor 推进；live 多轮迭代定案：DWM 系统圆角、视觉矩形、左键松手判定、代际计数+光标守卫防误隐、挂载 80ms 重申聚焦；spike 报告 .temp/tray-spike-report.md；实施记录见 x.progress.md PL018 组）

## 附录 PL019：托盘交互四修与退出入口（2026-10-01 补记立项，用户四 bug 报告驱动）

> 背景：PL018 收口后用户 live 目验报四 bug——①慢速移开预览窗不消失（Leave 事件慢速移出丢失，一次性判定无重试）②右键菜单与预览窗并存（右键意图是菜单，预览应让位）③菜单无退出项（托盘模式需要退出入口，PL018"无退出项"定案作废）④白板全选删除后 ▼ 三角残留（textarea 增删不产生 DOM 子节点变化，MutationObserver 三触发源全部观察不到）。
> 方案要点：①隐藏改守候循环（200ms 步进四条件评估，连续 400ms 离开两区域才隐藏）②右键 Down 分支收预览窗（MENU_ACTIVE 互斥方案实施后语义不符被用户撤销，现状保留待后续讨论）③菜单退出项走主窗 close() 复用完整关窗保存链（禁 app.exit 绕过保存）④WhiteboardView 加 watch(content) → boardRead.sync() 数据层补触发源。
> 红线：退出必须走关窗保存链；守候循环隐藏不引入误隐（悬停/移动中不消失）；三角重算不得干扰现有滚动触发。
> 状态：✅ 已完工（2026-10-01，V0.1.7.1 修订推进，已提交推送 6e427f0；实施记录见 x.progress.md PL019 组）

## 附录 PL020：预览延迟出现与显隐架构重构（2026-10-01 补记立项，用户需求 + bug 报告驱动）

> 背景：用户需求——hover 托盘瞬间出窗欠缺，应持续停留 0.5s 才出现（瞬间划过不打扰）；0.5s 内右键打断出现并锁死，菜单收起恢复出现条件，收菜单时光标恰停图标上不出窗（用户定案极端情况）。实施后用户报两 bug：菜单开着移回图标仍出窗、慢速移开预览窗不消失（孤儿窗）。
> 机制实锤（诊断日志）：①**菜单开着时托盘 Enter 仍会触发**（menu_open=true 连发五行实录；"鼠标回托盘 = 菜单已收"启发式证伪，即方案 A 撤销根因）②孤儿窗 = 多线程换代结构固有缺陷（新 Enter 换代出的守候线程是出现循环，退出路径不隐藏）。另收口两轮死锁：守候线程持锁窗口动作/持锁窗口查询均互挂（**铁律：Tauri 非主线程窗口调用含查询同步等待主线程，禁入任何锁内**）。
> 方案要点：单一常驻守候线程 + 四态状态机（Idle/Armed{since_ms}/Shown{miss}/Suppressed）+ step_watch 纯函数（单测直测）；菜单存亡 = EnumWindows 探本进程 #32768 菜单窗口（Suppressed 解除唯一事实源，解除后需新 Enter 重新武装）；窗口查询/动作全移锁外，"Armed ∧ 可见 → Hide"自愈；删前端悬停上报链（守候光标轮询全覆盖）。
> 红线：显隐只由守候线程执行（show/hide 唯一调用者）；窗口调用一律锁外；Enter 时菜单开 = 整事件忽略；测试时间源注入禁真实 sleep 依赖（步进循环除外，沿先例）。
> 状态：✅ 已完工（2026-10-01，V0.1.7.2 修订推进，已提交推送 d648f7a；两轮死锁用户回执"未响应"实锤修复；实施记录见 x.progress.md PL020 组）

## 附录 PL021：托盘菜单自绘化（2026-10-01 立项，用户目验驱动）

> 背景：用户目验发现原生 Win32 右键菜单深色模式下上下空条——机制 = Win11 为圆角/阴影预留的菜单窗口边距被 muda 深色自绘（WM_NCPAINT/UAH 接管，移植自 ppsspp UAHMenuBar）涂成实色，浅色模式系统原生绘制不可见；原生菜单无尺寸口无 padding 配置。用户拍板 B 方案（弃原生菜单改自绘 HTML 菜单窗，"做完出问题再回退版本"）。
> 方案要点：tray-menu 自绘窗（预览窗同款配方 + focusable(false) 点击不抢系统焦点）；TrayMenu.vue 四项（显示主窗/贴边吸附/窗口置顶/退出）+ tray_menu_action 命令统一分发（点项先收窗；snap/top 复用 on_pref_menu 核心；exit 走主窗 close() 关窗保存链）；spawn_menu_watch 点外收起守候（起步等弹出右键松开防"弹出即收"）；菜单开着再右键 = 收起（原生同款）；菜单存亡事实源 = 菜单窗可见性（#32768 探测删除）；定位 = 左缘对齐图标中轴向右展开（用户定案）+ 屏右缘钳制；宽度 100 逻辑像素（用户定案）。
> 实测新知：**WebView2 创建期 inner_size 宽度受最小 bounds 钳制**（60/100 均被抬至 136 逻辑，高度精确不受影响；预览窗 200 从未低于线未暴露）——创建后 set_size 重设绕过（实测精确）；真实物理尺寸须用 DwmGetWindowAttribute(EXTENDED_FRAME_BOUNDS) 测（GetWindowRect 受调用进程 DPI 虚拟化影响）。
> 红线：菜单窗不抢焦点（正打字不被打断）；点项必收菜单；退出仍走关窗保存链；菜单开 = 预览窗锁死（PL020 链路沿用菜单窗可见性）。
> 遗连修复（收口时用户报随本版修复）：**托盘"退出"未真正结束进程**——第一轮误诊"幽灵图标"（未验证进程死活即断言 NIM_DELETE 未执行），修后用户回执无效，tasklist 实证进程根本没退出（主窗消失被误读为"app 已退出"）；真根因 = exit 动作只销毁主窗，tray-preview/tray-menu 两隐藏窗撑住事件循环（自 PL019 落地即存在，当时仅验证位置落盘）；修复 = exit 销毁全部窗口（主窗 close() 走保存链 + 两托盘窗 destroy）→ 事件循环自然退出 → App drop 删托盘；配套 RunEvent::ExitRequested 显式 remove_tray_by_id 双保险 + 守候线程 5s 缺席自灭退出口。
> 状态：✅ 已完工（2026-10-01，V0.1.8.0 minor 推进；live 全清单用户回执通过；实施记录见 x.progress.md PL021 组）

## 附录 PL022：设置板偏好同步与菜单文案（2026-10-01 立项，用户目验驱动）

> 背景：PL021 收口验证用户报两点——①托盘菜单切贴边/置顶后设置板对应开关不同步（根因 = 前端从未监听 prefs-changed，PL018.3"三入口双向"只实现了托盘勾选态方向，设置板回显缺失，原生菜单时代即存在）②菜单首项"显示主窗"词不达意（现实现 show+focus 本就是聚焦语义：隐藏唤回/被盖拉前/透明态夺焦，用户定案改称"聚焦主窗"）。
> 方案要点：App.vue 监听 prefs-changed → alwaysOnTop/snapToEdge ref 同步刷新（v-model 链自动回显，补齐三入口双向另一半）；TrayMenu.vue 首项文案改"聚焦主窗"。最小化不处理（app 无最小化入口，无法触发，用户确认）。
> 红线：设置板开合状态均同步；监听随组件卸载清理。
> 状态：✅ 已完工（2026-10-01，V0.1.8.1 修订推进；实施记录见 x.progress.md PL022 组）
