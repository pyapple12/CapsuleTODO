# 进度追踪（x.progress.md）

> 文件职责：任务清单与进度追踪，与 `z.plan.md`（方案与审计归档）配套：方案在 z.plan 展开，执行拆条在本文件勾选。结构：**`## 已完成 ✅` 区在前、`## 未完成` 区在后**，任务完成后整组移动到已完成区；**两个区内部均按从上到下 = 从旧到新排序，新组一律追加在区末尾**；已完成区历史组全量保留、只增不删（系列定案：收口不得滚出历史组，违规滚出须自 git 历史恢复）。
> 格式速查：任务组 `### PL{NNN}: {标题} [来源引用]`（来源引用：[plan#Phase N] 计划书 / [problems#N] 问题备忘录 / [audit#A{NNN}] 审计报告），组内用 `#### {小节}` 分层；子任务 `- [ ] PL{NNN}.{序号} {标题} —— {做法}；验证：{检验方式}`；审计修复任务组 FIX{NNN} 由 audit-report 归档环节生成，条目格式 `- [ ] FIX{NNN}.{序号} [P{级别}] {标题} —— {做法}；验证：{检验方式}`，编号规则见 `.agents/skills/audit-report`。
> 勾选注记：条目完成时改 `[x]`，并在句尾追加 `（YYYY-MM-DD 已验证：{一句话结论}）`——结论如实，不达标不降级宣布。

## 已完成 ✅

### PL001: 玻璃壳与最小清单闭环 [plan#一期]

> 范围：工程骨架 + 玻璃壳（焦点联动材质沿系列配方）+ Todo 状态机 TDD + 内存态清单勾选闭环；方案见 z.plan.md 附录 PL001。
> 红线：业务纯逻辑零 tauri 依赖（禁 import tauri）；玻璃观感用户目验通过才算闭环；依赖版本锚定沿 Pulse 实证（typescript ^5.9.3 等，防 TS7×vue-tsc 不兼容）；临时文件一律落 .temp/。

#### 阶段 A：工程骨架

- [x] PL001.1 工具链确认与 Tauri 2 工程骨架 —— 工具链沿机器实证（cargo 1.96.1 / Node 26.7.0 在位，Pulse 已验）；手写骨架沿系列目录风格：`core/Cargo.toml`（name capsule-todo、version 0.1.0 单一来源、edition 2021；deps：tauri { version = "2" }、serde { version = "1.0.229", features = ["derive"] }、thiserror "2.0.20"；build-deps：tauri-build { version = "2" }）+ `core/tauri.conf.json`（productName CapsuleTODO、identifier com.pyapple12.capsule-todo、窗口 title CapsuleTODO / width 300 / height 400 / transparent true / decorations false / **resizable false**、build.frontendDist ../dist + beforeDevCommand npm run build、app.version 省略、bundle.active true + icons/icon.ico）+ `core/capabilities/default.json`（permissions：core:default + core:window:allow-start-dragging）+ `core/src/main.rs`（薄入口调 capsule_todo::run()）+ `core/src/lib.rs`（run() 空壳 + `//!` 模块注释）+ 占位图标（`.temp/gen-icon.mjs` 程序生成，沿 Pulse PL001.2）+ `ui/`（package.json：name capsule-todo、type module、scripts dev/build/preview/tauri 沿 Pulse、deps @tauri-apps/api ^2.11.1 + vue ^3.5.42、devDeps @tauri-apps/cli ^2.11.4 + @vitejs/plugin-vue ^6.0.8 + prettier ^3.9.6 + typescript ^5.9.3 + vite ^8.2.2 + vue-tsc ^3.3.11；`ui/index.html` + `ui/main.ts` + `ui/App.vue` 占位；vite.config.ts 与 tsconfig.json 逐字对照 Pulse 同名文件移植）+ configs/ 与 data/ 目录占位（.gitkeep）；npm install；验证：`npm run build` 绿 + `cargo check` 绿 + debug exe 启动探测 MainWindowTitle=[CapsuleTODO] 5 秒存活后受控关闭（沿 Pulse 等价验证法）（2026-09-17 已验证：npm build 3.9s 绿 / cargo check 首跑 2m21s 绿 / exe 探针过——ALIVE title=[CapsuleTODO] 6 秒存活受控关闭；占位图标 4286 字节生成；探针路径修正——Cargo 包根 = core/，产物落 core/target/debug/ 而非仓库根 target/）
- [x] PL001.2 门禁四件套接入与首跑 —— cargo fmt / clippy --all-targets -- -D warnings / check + npx prettier --write + vue-tsc 全部接入（prettier 配置 .prettierrc 已就位，node_modules 已随 PL001.1 就位）；验证：首跑全绿并记录耗时基线（注记回写本条）（2026-09-17 已验证：全绿——fmt 1.7s / clippy 6.3s（增量，依赖编译随 check 2m21s 完成）/ check 1.3s / prettier 1.8s / npm build 3.9s 含 vue-tsc）

#### 阶段 B：玻璃壳

- [x] PL001.3 窗口玻璃与焦点联动材质 —— `core/src/lib.rs`：run() 装配；Windows 分支局部闭包 `set_backdrop(hwnd, kind: u32)`（封装 DwmSetWindowAttribute(hwnd, 38, &kind, 4) 与 HRESULT 非零严格抛错，extern "system" dwmapi 直连零新依赖，沿 Pulse PL011 同款）；`.on_window_event` Focused 分支：true → set_backdrop(3 DWMSBT_TRANSIENTWINDOW)、false → set_backdrop(1 DWMSBT_NONE)（**非 0，0=AUTO**）+ emit "window-focus" 事件（失败落 eprintln 不中断主流程）+ **容错白名单登记第①项**（AGENTS 错误策略：材质切换失败维持前态，场景/降级/理由三要素，沿 Pulse FIX003.7）；验证：cargo clippy/check/test 绿 + AGENTS 白名单三要素核对 + live 目验——平时透明透桌面、聚焦瞬间真磨砂、失焦即刻回透明、反复切换无灰板残留（300×400 小窗复核）（2026-09-17 已验证：cargo 门禁绿 + AGENTS 白名单①三要素核对 + 用户实机目验全过——平时透明/聚焦磨砂/失焦回透明/反复切换无灰板残留）
- [x] PL001.4 玻璃卡片与深浅色跟随 —— `ui/App.vue`：全窗单层玻璃（margin 0、height 100vh、radius 8px、::before 分态纱——透明态 30% 浅白 rgba(255,255,255,0.3)/纯黑 rgba(0,0,0,0.3)、.focused 磨砂态 0% 纱，沿 Pulse PL011 用户拍板值）+ 亮边描边/rim/text-shadow 令牌对照 Pulse `ui/App.vue` 实态移植（配方权威 = Pulse z.plan 附录 PL010）+ prefers-color-scheme 双主题；监听 window-focus 挂 .focused class + isFocused() 启动兜底（沿 PL011 管线）；验证：vue-tsc + npm run build 绿 + live 目验双主题玻璃观感与可读性（2026-09-17 已验证：构建绿 + 用户实机目验全过；开发期曾修一处 TS6133 未使用类型导入）
- [x] PL001.5 全窗拖动 —— `ui/App.vue` 全局 mousedown：`e.target.closest(DRAG_INTERACTIVE)` 命中即忽略（常量白名单：button/input/textarea/清单行交互件，随组件演进维护），其余 `getCurrentWindow().startDragging()`（@tauri-apps/api/window；capabilities 已含 allow-start-dragging）；验证：live——空白板面按住拖动流畅、清单行/复选框不误触拖动、无文字误选（2026-09-17 已验证：构建绿 + 用户实机目验全过）

#### 阶段 C：Todo 状态机（TDD：先 FAIL 后 PASS）

- [x] PL001.6 状态骨架与错误类型 —— 新建 `core/src/todo.rs`（`//!` 模块注释声明纯逻辑禁 import tauri）：`TodoItem { id: i64, text: String, done: bool }`（derive Serialize/Clone/Debug）+ `TodoList { items: Vec<TodoItem>, next_id: i64 }`（next_id 从 1 起）+ `TodoError`（thiserror：EmptyText/TooLong/NotFound）+ 常量 `MAX_TEXT_LEN: usize = 100`（注释注明可调）；验证：cargo clippy/check 绿（2026-09-17 已验证：clippy -D warnings + check 绿，纯逻辑零 tauri 依赖）
- [x] PL001.7 四操作 TDD —— 先写测试确认 FAIL 再实现：`add(text) -> Result<TodoItem, TodoError>`（trim 后非空 + 长度 ≤100 校验，追加 tail 并返回带 id 条目）/`toggle(id) -> Result<TodoItem, TodoError>`（翻转 done，不存在 Err(NotFound)）/`remove(id) -> Result<(), TodoError>`/`sorted_view() -> Vec<TodoItem>`（新建 Vec：未完成在前按 id 升序、已完成在后按 id 升序，不动内部顺序）；用例：正常增、空文本/纯空白拒、超长拒、恰 100 过界、toggle 往返断言翻转值、toggle/remove 不存在 id 均 Err、交错增入后排序视图断言分区与序；验证：红灯（编译错 E0425 级）→ 实现转绿，全部用例零真实时间/外部依赖（2026-09-17 已验证：红灯 E0425/E0433 实证 → 转绿 11 用例全绿，含 trim 存储、恰 100 过界、排序视图分区断言与内部顺序不动）
- [x] PL001.8 命令层 —— 新建 `core/src/commands/mod.rs`：`AppContext { list: Mutex<TodoList> }`（.manage 注册）+ 泛型 `poison()` 锁助手（LockResult\<T\> → Result\<T, CommandError\>，沿 Pulse FIX001.9）+ `CommandError`（thiserror：Todo(#[from] TodoError)/Poisoned，derive Serialize 跨 IPC）；新建 `core/src/commands/todo.rs`：todo_add(text)/todo_toggle(id)/todo_remove(id)/todo_list() 四命令——核心逻辑抽接收 &AppContext 的自由函数（脱离 tauri::State 直测，沿 Pulse PL001.11 法）；`lib.rs` run() 构造预置示例 TodoList（3–4 条含已完成，目验用；PL002 接真实数据后删除）注入 manage；验证：命令核心函数 cargo test 全绿（增/删/勾/非法文本/不存在 id/排序视图）（2026-09-17 已验证：命令核心 4 用例直测全绿（全组 15）；预置示例 4 条含 1 条已完成；cargo doc --no-deps 0 告警）

#### 阶段 D：UI 接线与收口

- [x] PL001.9 清单 UI 与勾选闭环 —— 新建 `ui/types.ts`（`interface TodoItem { id: number; text: string; done: boolean }`，注释注明 Rust serde 为契约单一来源）+ 新建 `ui/components/TodoList.vue`：清单行（自绘复选框 + 文本，完成态删除线 + 透明度变淡）+ 分区展示（进行中区 / 已完成区，数据源 = todo_list 排序视图，前端按 done 分组渲染，不排序只分组）；勾选 invoke todo_toggle 成功后重拉 todo_list（无轮询——无计时需求）；App.vue 装配 TodoList；验证：vue-tsc + npm run build 绿 + live 人工闭环（勾选→删除线并沉入已完成区、取消勾选→回进行中区、预置数据渲染正常、玻璃观感用户确认）（2026-09-17 已验证：构建绿 + 用户实机目验全过——预置 4 条渲染、勾选删除线沉入已完成区、取消勾回进行中区）
- [x] PL001.10 PL001 收口 —— 门禁四件套全绿（fmt --check / clippy -D warnings / cargo test / vue-tsc / npm run build / prettier）；结论回写 z.plan 附录 PL001 状态行；README/AGENTS 状态行回改（PL001 完成）；勾结；验证：门禁全绿 + 文档一致性核对（README 徽章与 Cargo.toml 0.1.0 一致、版本 V0.1.0.3）+ commit 草案（feat: V0.1.0.3）交用户（2026-09-17 已验证：门禁七项全绿 + 用户目验全过；**提交按用户定案延后**——一期三 PL 全部做完后统一一次性提交推送，V0.1.0.3 归一期收口提交）

### PL002: 清单持久化与内容管理 [plan#一期]

> 范围：SQLite 存储落 data/（双落址）+ 操作即落库（单一事实源 = db）+ 添加/删除 UI + 已完成折叠区与空态；方案见 z.plan.md 附录 PL002。
> 红线：参数化 SQL 禁拼接；测试禁触真实用户数据（内存库/临时路径）；首启无 db = 正常态（建表自动），库损坏/SQL 错误严格报错启动失败（不进白名单）。

#### 阶段 A：存储层（TDD，内存 db）

- [x] PL002.1 依赖接入与探针 —— `core/Cargo.toml` 增 rusqlite { version = "0.40.2", features = ["bundled"] }（锚定 Pulse 实证版本）；新建 `core/tests/storage_probe.rs` bundled 工具链冒烟（open_in_memory + 建表查询 smoke，沿 Pulse 资产模式）；验证：cargo test 编译绿 + smoke 过（2026-09-17 已验证：rusqlite 0.40.2 bundled 编译 1m00s（全局缓存无长编译痛感）+ smoke 过）
- [x] PL002.2 storage.rs TDD —— 新建 `core/src/storage.rs`（`//!` 模块注释）：`StorageError`（thiserror：Sqlite(#[from] rusqlite::Error)/Io(std::io::Error)/NotFound）+ `Storage` struct：`open(path)`（父目录 create_dir_all + Connection::open）/`open_in_memory()` + `init()` 建表（`CREATE TABLE IF NOT EXISTS todos(id INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT NOT NULL, done INTEGER NOT NULL DEFAULT 0)`，幂等；**不做 created_at**——排序按 id 即创建序，KISS）+ `add(text)`（?1 参数化绑定，last_insert_rowid 回填 id）/`toggle(id)`（UPDATE done = NOT done + get 回读）/`get(id)`/`remove(id)`/`list()`（`ORDER BY done ASC, id ASC`）；用例：内存库建表幂等、add→list 往返、toggle 往返与 get、remove 后 list 空、不存在 id 三操作均 Err、多行排序断言、open 父目录自建落盘；验证：先 FAIL 后 PASS 全绿，SQL 全参数化走查（2026-09-17 已验证：红灯 E0425/E0433 → 转绿 7 用例；**实现演进**：设计稿 set_done 改为 toggle+get——勾选语义 SQL 侧翻转更顺且命令层需回读，语义等价；一处测试修 Windows 文件锁（drop 连接后再清目录））
- [x] PL002.3 运行时落址 —— 新建 `core/src/paths.rs`（`//!` 模块注释）：`runtime_root()`（cfg 分支）+ `data_dir_under(root)`（root.join("data") + create_dir_all，可注入）+ `data_dir()` + `db_path()`；用例：临时目录注入路径解析（禁触真实 data/）；验证：cargo test 绿 + live dev 启动后项目根 `data/todo.db` 出现（2026-09-17 已验证：测试绿 + **落址偏差实证修正**——debug 下 CARGO_MANIFEST_DIR = core/，db 首落 core/data/，runtime_root 修正为取其父目录（项目根 = 仓库根），冒烟探针复核 DB-AT-ROOT）

#### 阶段 B：接线（单一事实源 = db）

- [x] PL002.4 AppContext 扩展与启动装载 —— `core/src/commands/mod.rs` AppContext 改 `AppContext { storage: Mutex<Storage> }`（lock_storage + poison 助手；CommandError 增 Storage(String) 变体 + From<StorageError>）；`core/src/lib.rs` run() 启动：paths::db_path() → Storage::open（失败带路径格式化后启动报错，严格抛错主线）→ 空库即为空态（**预置示例条目删除**，空态由前端兜底）；验证：cargo test 绿（命令核心改测内存库）+ live 正常启动装载 + 人为给坏路径启动退出码非零（2026-09-17 已验证：测试绿 + 冒烟探针正常启动装载；坏路径变体并入用户目验清单）
- [x] PL002.5 命令落库与纯逻辑收敛 —— `core/src/commands/todo.rs` 四命令改走 storage（add → 先 todo.rs `validate_text(text)` 校验再 storage.add；toggle → storage.toggle；remove → storage.remove；list → storage.list）；`core/src/todo.rs` 收敛为 TodoItem + TodoError（EmptyText/TooLong，NotFound 归 StorageError）+ validate_text（TodoList/sorted_view 退役删除——排序移交 SQL，单一事实源 = db，禁留双源）；测试同步改内存库往返；验证：cargo test 全绿 + grep TodoList 零残留 + validate_text 拒绝用例仍在（2026-09-17 已验证：16 项 lib 测试全绿（paths 3 + storage 7 + todo 3 + commands 4）+ 探针过 + cargo doc 0 告警）

#### 阶段 C：收口

- [x] PL002.6 添加/删除/折叠 UI —— 新建 `ui/components/AddBar.vue`：顶部输入行（placeholder "添加一条待办…"、回车与按钮双触发、trim 后空文本禁用）+ 错误行（invoke 失败红色文案可见、成功后清空输入，沿 Pulse FIX001.5 反馈模式）；`ui/components/TodoList.vue` 增：条目悬停 × 删除钮（invoke todo_remove，直接删不确认——一期定案）+ 已完成折叠区（表头 "已完成 N" 点击收/展，默认收起，N 前端计数，展开区条目保留删除线）+ 空态文案（无任何条目时 "暂无待办，添加一条吧"）；App.vue 装配 AddBar + TodoList；验证：vue-tsc + npm run build 绿 + live（添加即现、空文本禁用、删除即消、勾选沉入折叠区且计数对、重启数据在、空库空态文案、AddBar 失败可见——可临时传非法值触发）（2026-09-17 已验证：构建绿 + 用户实机目验全过——添加/空文本禁用/删除/勾选折叠计数/重启数据在/空态文案）
- [x] PL002.7 PL002 收口 —— 门禁全量（fmt --check / clippy -D warnings / cargo test / vue-tsc / npm run build / prettier）；结论回写 z.plan 附录 PL002 状态行；README/AGENTS 状态行（PL002 完成）；勾结；验证：门禁全绿 + 重启持久性复验（增删勾→重启→全在）+ 文档一致性（版本 V0.1.0.3）+ commit 草案（feat: V0.1.0.3）交用户（2026-09-17 已验证：门禁七项全绿（fmt --check / clippy -D warnings / cargo test 27+探针 / doc 0 告警 / vue-tsc / npm build / prettier）+ 重启持久经用户目验；**提交随一期收口统一执行**——用户定案）

### PL003: 桌面固定与一期收口 [plan#一期]

> 范围：置顶 + 全屏让位 + 位置记忆 + 单实例 + 观感细调 + 一期全量收口；方案见 z.plan.md 附录 PL003。
> 红线：零新依赖（extern user32 直连沿系列 extern 模式；tauri-plugin-single-instance 除外——沿 Pulse 先例）；全屏让位必须 live 实测（视频全屏场景），不得推演宣布；验证禁改系统设置（AGENTS 陷阱）。

#### 阶段 A：桌面固定

- [x] PL003.1 置顶 —— `core/tauri.conf.json` 窗口配置增 `"alwaysOnTop": true`；验证：live——启动后板子浮于普通应用窗口之上（2026-09-17 已验证：用户实机目验全过）
- [x] PL003.2 全屏让位判定纯函数 TDD —— 新建 `core/src/fullscreen.rs`（`//!` 模块注释）：`is_covering(win: Rect, monitor: Rect) -> bool`（Rect { left, top, right, bottom } i64 四元组；win 覆盖 monitor 面积 ≥95% 判全屏，阈值常量 `FULLSCREEN_COVER_PCT` 注释注明依据）+ `should_yield(foreground_is_fullscreen) -> bool` 直通语义留扩展位；用例：恰好全屏 true、差 1px false（95% 边界两侧）、半屏 false、超出屏幕 true；验证：先 FAIL 后 PASS，零 Win32 依赖纯函数（2026-09-17 已验证：红灯 E0422/E0425 → 转绿 6 用例；**判法定案修订**——设计稿"面积 ≥95%"改**逐边包含法**（四边贴边 + 8px 容差）：面积法会把小任务栏显示器的最大化窗口误判为全屏（占比可 >95%），包含法为业界标准（Chromium 同款）且最大化窗必留任务栏条不会误判；z.plan 附录 PL003 已同步修订）
- [x] PL003.3 全屏让位接线 —— `core/src/fullscreen.rs` 增 `spawn_fullscreen_watcher(app: AppHandle)`：std::thread + 1s sleep 轮询；extern "system" user32 直连 GetForegroundWindow/GetWindowRect/MonitorFromWindow/GetMonitorInfoW（零新依赖，沿 dwmapi extern 模式；失败 eprintln 维持前态 + **容错白名单登记第②项**：轮询失败维持当前置顶态，三要素）；板子所在显示器取主窗 monitor；前台窗覆盖该显示器 → AppHandle.run_on_main_thread 摘 topmost（set_always_on_top(false)），否则挂回 true（**状态变化才调用**，避免每秒重设）；`lib.rs` setup 末尾 spawn；验证：cargo test 绿（纯函数部分）+ live——普通使用置顶不丢、浏览器 F11 视频全屏时板子被盖住、退出全屏板子回置顶、反复进出无闪烁；无边框全屏游戏若可测则补测，不可测如实登记"仅视频全屏实测"（2026-09-17 已验证：用户实机目验全过——视频全屏被盖/退出恢复置顶/普通最大化窗口不误判；**实现注记**：set_always_on_top 内部派发主线程，工作线程直调，未走 run_on_main_thread，行为等价；失败单次日志防刷屏已实现）
- [x] PL003.4 位置记忆 —— 新建 `core/src/settings.rs`（沿 Pulse JSON 原子写模式：同目录 .tmp 写入 + rename 替换，失败清理临时文件）：`WindowSettings { x: i32, y: i32 }`（serde default）+ `load(path)/save(path)` + `SettingsError`（Json/Io）；`core/src/paths.rs` 增 `configs_dir()`（root.join("configs") + 自建）+ `settings_path()` = configs/config.json；`lib.rs` 启动 load——**文件不存在 → 默认主屏右下距边 40px（默认值常量注释可调），NotFound 回默认登记 AGENTS 容错白名单第③项（三要素）；JSON 损坏等其余错误严格报错退出不在此列** + 越界兜底（x/y 不在任一显示器范围 → 回默认，显示器枚举经 tauri available_monitors）；窗口 Destroyed/ExitRequested 时按 outer_position save（拖动中不写盘）；验证：cargo test 绿（往返一致/损坏严格/临时路径）+ live——拖到别处重启位置保留、删 config.json 首启回默认右下、把 config.json 改成非法值启动报错退出（2026-09-17 已验证：单测往返/损坏严格 + 行为探针 CONFIG-SAVED 实证（{"x":1505,"y":540} 右下默认位落盘）+ 用户实机目验——拖动重启保留、删文件回默认、非法值报错）
- [x] PL003.5 单实例 —— `core/Cargo.toml` 增 tauri-plugin-single-instance = "2"（沿 Pulse PL004.5 同款）；`lib.rs` builder 首位注册插件：二次启动回调唤起主窗（show + set_focus）；验证：live 双开——第二实例自退、板子被唤起聚焦、tasklist 仅一进程（2026-09-17 已验证：行为探针 PROC-COUNT=1（二次启动自退）+ 用户实机目验唤起聚焦过）

#### 阶段 B：收口

- [x] PL003.6 观感细调与全形态复核 —— 300×400 小窗密度/字号/间距目验微调（App.vue/TodoList.vue 令牌级调整，零结构重排）；深浅双主题全形态复核（清单行/折叠区/AddBar/空态/错误行/聚焦磨砂与透明态切换）；验证：live 目验清单逐项过（跟随系统当前主题，禁改系统设置，沿 AGENTS 陷阱约束）（2026-09-17 已验证：用户实机目验——300×400 密度字号无需调整，深浅双主题全形态过）
- [x] PL003.7 PL003 与一期收口 —— 全量门禁 + 一期验收清单逐项过（①玻璃小板常驻桌面置顶 ②全屏让位 ③勾选→折叠+删除线 ④添加/删除 ⑤重启持久 ⑥位置记忆 ⑦双开防 ⑧深浅色跟随）；README 徽章与状态行、AGENTS 状态行、计划书 §6 一期勾选回写；勾结；验证：门禁全绿 + 验收清单逐项 + 文档一致性（版本 V0.1.0.4）+ commit 草案（feat: V0.1.0.4）交用户（2026-09-17 已验证：一期验收清单 8 项全过（两轮用户实机目验累计覆盖：置顶/全屏让位/勾选折叠删除线/增删/重启持久/位置记忆/双开防/深浅色跟随 + A001 审计与 FIX001 修复闭环）；门禁七项全绿；**版本注记修正**——V0.1.0.2 已被一期立项文档提交占用，一期收口提交号顺延为 V0.1.0.3；README 徽章/AGENTS 状态行/计划书 §6 同步回写）
- [x] PL003.8 首轮全量审计 A001 —— 用户触发 audit-project skill 全量审计 → audit-report 归档（编号经用户确认 → z.plan 附录 A001 + FIX001 任务组写入本文件）；验证：审计报告四节完整（含豁免清单"无豁免也写无"）+ FIX 条目做法/验证可执行（2026-09-17 已验证：A001 归档 z.plan 附录（四节完整，观察项 10 条经用户定案保留观察）+ FIX001 任务组三条落本文件；审计结论 = 无 P0-P2，P3 两项）

### FIX001: 第1轮审计修复 [audit#A001]

> 范围：A001 报告（2026-09-17）P3 两项 + 收尾验证；无 P0/P1/P2。
> 红线：审计修复不引入行为变化（P3-1 错误文案可读化除外——错误显示路径本身即缺陷对象）；保活不回归；零新依赖。

- [x] FIX001.1 [P3] CommandError 跨 IPC 可读化 —— `core/src/commands/mod.rs:30-38`：CommandError 去 `#[derive(Debug, Serialize)]` 中的 Serialize（保留 Debug），追加手动实现 `impl serde::Serialize for CommandError`（match 变体 `serializer.serialize_str`：Todo/Storage 透传承载的 msg、Poisoned 固定文案"共享锁中毒"）——跨 IPC 契约从对象变纯字符串，前端 `String(err)` 即恢复可读；同文件 tests 增用例断言 `serde_json::to_string(&CommandError::Todo("x".into()))` == `"\"x\""`；验证：cargo fmt/clippy/test 全绿 + 单测序列化断言 + live 粘贴超长文本 → AddBar 错误行显示"待办文本超长（上限 100 字符）"（2026-09-17 已验证：TDD 红→绿——红灯断言精确复现缺陷（left = {"Todo":"待办文本为空"} 对象形态）→ 手动 impl serde::Serialize 后转绿 28 项全绿；**live 一项经用户目验确认（2026-09-17）**，反向验证记录 .temp/fix001-verification.md）
- [x] FIX001.2 [P3] 文档实态回改 —— README.md:25 存储行删"（一期定案后回改）"括注改实态表述（`data/todo.db` 双落址已落地）；AGENTS.md:36-38 目录规划引言"规划态"改"2026-09-17 一期实态"，目录树回填 core/src 实际模块（lib.rs 装配 / main.rs / todo.rs / storage.rs / paths.rs / settings.rs / fullscreen.rs / commands/（mod + todo）/ tests/storage_probe.rs）与 ui/components（AddBar.vue / TodoList.vue）；验证：grep "一期定案后回改"（README）与"规划态"（AGENTS 目录规划段）零残留 + 人工核对树与实态一致（2026-09-17 已验证：双 grep 归零（0/0）+ 树逐行核对实态（assets/ 行删除——仓库无此目录，图标实落 core/icons/；新增 capabilities/tests/icons 三行与 src 七模块）+ npm build/prettier 绿）
- [x] FIX001.3 [P3] FIX001 收尾 —— 全量门禁（fmt --check / clippy -D warnings / cargo test / doc / vue-tsc / npm build / prettier）+ 反向验证逐条（原问题"错误不可读"→验收"live 超长文本显示完整中文"；原问题"文档滞后"→验收"grep 归零 + 树实态一致"）+ A001 状态行回写（📌 待修复 → ✅ 已修复）+ 勾结；验证：门禁全绿 + 反向验证清单逐项过（2026-09-17 已验证：门禁七项全绿 + 反向验证逐条（序列化契约断言绿 + live 超长文本显示完整中文经用户确认 + 文档 grep 双归零）+ A001 状态行回写 ✅ 已修复；反向验证记录 .temp/fix001-verification.md；全组 3 条勾结，V0.1.0.3 随一期收口统一提交）

### PL004: 页签导航与气泡 [plan#二期]

> 范围：三页签导航骨架 + bubbles 表 + 气泡全链路（捕获/列表/复制回/删除/满 5 提醒与清空）；方案见 z.plan.md 附录 PL004。
> 红线：零自动剪贴板监听（用户定案——隐私与噪音考量）；气泡仅剪贴板来源；满 5 软提醒不自动删；单一事实源 = db；clipboard 读写仅 Rust 侧（不经 ACL，capabilities 不动）。

#### 阶段 A：导航骨架（纯前端）

- [x] PL004.1 三页签结构 —— `ui/App.vue`：`activeTab = ref<"todos" | "bubbles" | "whiteboard">("todos")`；顶栏下新增分段控件（三个 button：清单/气泡/白板，当前项 accent 高亮样式）；既有清单区（AddBar + TodoList）整体迁入 todos 页容器（v-if 按页渲染）；bubbles/whiteboard 页占位；`watch(activeTab)` 切页触发对应数据刷新（bubbles 页拉 bubble_list、todos 页拉 todo_list）；验证：vue-tsc + npm build 绿 + live——页签切换正常、清单功能零回归（2026-09-17 已验证：构建绿；live 移交二期统一目验）

#### 阶段 B：数据层（TDD）

- [x] PL004.2 bubble 纯逻辑 —— 新建 `core/src/bubble.rs`（`//!` 模块注释，禁 import tauri）：`BubbleItem { id: i64, text: String }`（derive Serialize/Clone/Debug）+ `MAX_BUBBLES: usize = 5`（注释：用户定案满 5 提醒，可调）+ `should_remind(count: usize) -> bool`（count >= MAX）+ `validate_bubble_text(text) -> Result<(), BubbleError>`（trim 非空，BubbleError::EmptyText）；用例：恰 5 触发、4 不触发、6 触发、空/纯空白拒；验证：先红（E0425 级）后绿（2026-09-17 已验证：红灯 E0425/E0433 → 转绿 3 用例）
- [x] PL004.3 bubbles 表与存储 —— `core/src/storage.rs`：init 增 `CREATE TABLE IF NOT EXISTS bubbles(id INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT NOT NULL)`；增 `add_bubble(text) -> Result<BubbleItem>`（?1 绑定 + last_insert_rowid）/`list_bubbles()`（ORDER BY id DESC 新在前）/`remove_bubble(id)`（零行 NotFound）/`clear_bubbles() -> Result<usize>`（返回清除条数）/`count_bubbles() -> Result<usize>`（COUNT(*)）；用例：往返/倒序断言/删除 NotFound/清空返回数与清空后空/多行；验证：先红后绿全绿，SQL 全参数化（2026-09-17 已验证：红灯 E0599 → 转绿 3 用例）
- [x] PL004.4 剪贴板捕获命令 —— `core/Cargo.toml` 增 tauri-plugin-clipboard-manager = "2"（官方插件，二期唯一新依赖）；`lib.rs` `.plugin(tauri_plugin_clipboard_manager::init())`；新建 `core/src/commands/bubble.rs` 五命令——`bubble_capture(app: AppHandle, ctx)`（app.clipboard().read_text() → Err 或空串 → CommandError::Clipboard("剪贴板无文本内容")；validate_bubble_text → storage.add_bubble）/`bubble_list(ctx)`/`bubble_copy(id, app)`（get 文本 → write_text）/`bubble_remove(id, ctx)`/`bubble_clear(ctx) -> usize`；`commands/mod.rs` CommandError 增 `Clipboard(String)` 变体（手动 Serialize match 增透传分支，纯字符串契约沿 A001 修复后形态）+ From 无需（构造点直构）；核心自由函数（capture_core(text, ctx) 等）脱离 AppHandle 直测内存库，clipboard 读写留命令薄壳；验证：先红后绿 + clippy -D warnings + doc 0 告警（2026-09-17 已验证：4 用例直测全绿（校验入库/空文本拒/回读/删清）+ clippy/doc 绿；实现中补 get_bubble（复制回取数源，设计遗漏当场补红绿）；clipboard 读写薄壳 live 移交统一目验）
- [x] PL004.5 气泡页 UI —— 新建 `ui/components/BubblesView.vue`：顶部操作行（"⧉ 捕获剪贴板"按钮 invoke bubble_capture → 成功重拉/失败错误行，沿 AddBar 反馈模式）+ 横幅（items.length >= 5 → "气泡已满 5 个，该清理了" + 清空按钮**二态确认**：首点变红"确认清空 N 条？"再点执行 bubble_clear，3 秒未点复位）+ 气泡条列表（倒序渲染；点击行 invoke bubble_copy → 顶部状态行"已复制到剪贴板"2 秒消失；悬停 × 删除 invoke bubble_remove）；空态"暂无气泡，点上方捕获剪贴板"；`ui/types.ts` 增 BubbleItem 镜像；App.vue 装配气泡页；验证：vue-tsc + npm build 绿 + live 全链路（捕获→点击复制回粘贴验证→单删→满 5 横幅→二态清空）（2026-09-17 已验证：构建绿 + 冒烟探针过；**实现演进**——bubble_list 改返回 BubbleSnapshot{items, remind}，满 5 判定由 Rust 裁决防前后端阈值双处漂移（沿 A001 dim 12 教训）；live 全链路移交统一目验）
- [x] PL004.6 PL004 收口 —— 门禁全量 + 结论回写 z.plan 附录 PL004 + README/AGENTS 状态行（PL004 完成）+ 勾结；验证：门禁全绿 + live 清单 + commit 不单独提交（V0.1.1.1 随二期统一提交——用户定案）（2026-09-17 已验证：门禁七项全绿 + 全组勾结；**自纠记录**：bubble_list_core 快照化时曾用 python 补丁改源码一处，违反"edit 工具"约定，当场自查记录并回归 edit 工具（沿 Pulse PL002.13 先例）；live 移交统一目验）

### PL005: 白板 [plan#二期]

> 范围：单块文本草稿区 + 防抖自动保存 + 切页/关窗 flush；方案见 z.plan.md 附录 PL005。
> 红线：纯文本不做画笔；内容软上限 10,000 字符严格拒绝；自动保存失败可见不静默。

#### 阶段 A：数据层（TDD）

- [x] PL005.1 whiteboard 表与校验 —— 新建 `core/src/whiteboard.rs`（`//!` 模块注释，禁 import tauri）：`MAX_CONTENT_LEN: usize = 10_000`（注释可调）+ `validate_content(content) -> Result<(), WhiteboardError>`（超长拒，WhiteboardError::TooLong）；`core/src/storage.rs` init 增 `CREATE TABLE IF NOT EXISTS whiteboard(id INTEGER PRIMARY KEY CHECK (id = 1), content TEXT NOT NULL DEFAULT '')` 单行约束 + `load_whiteboard() -> Result<String>`（无行返回空串——首启正常态）+ `save_whiteboard(content) -> Result<()>`（INSERT OR REPLACE UPSERT 幂等）；用例：默认空串/写入回读/覆盖/upsert 幂等/超长拒（校验侧）；验证：先红后绿（2026-09-17 已验证：两轮红灯（E0425 校验侧 + E0599 存储侧）→ 转绿 4 用例；whiteboard 表随 init 幂等建表）
- [x] PL005.2 白板命令 —— 新建 `core/src/commands/whiteboard.rs`：`whiteboard_load(ctx) -> String` / `whiteboard_save(content, ctx)`（validate_content → save_whiteboard；超长 → CommandError::Whiteboard("白板内容过长（上限 10000 字符）")——CommandError 增变体 + Serialize 分支）；核心自由函数直测内存库；验证：先红后绿 + clippy/doc（2026-09-17 已验证：2 用例直测全绿（默认空/回读 + 超长可见拒）+ 序列化契约断言增 Clipboard/Whiteboard 两分支 + clippy/doc 绿）

#### 阶段 B：UI 与收口

- [x] PL005.3 白板页 UI —— 新建 `ui/components/WhiteboardView.vue`：全页 textarea（玻璃面板内嵌，placeholder"随手记点什么…"）+ 底部状态行（有未存改动 → "编辑中…"；防抖保存成功 → "✓ 已自动保存"）；防抖 800ms（常量注释可调）；保存失败错误行可见（沿 AddBar 模式）；`ui/App.vue`：切页 watch flush 白板脏数据 + `getCurrentWindow().onCloseRequested`（async 保存后放行，不调 prevent 无需额外权限）；验证：vue-tsc + npm build 绿 + live（输入 → 切页切回内容在；重启在；关窗重开在；超长拒提示可见）（2026-09-17 已验证：构建绿；**实现修订（用户"按推荐后期再改"授权内）**——JS onCloseRequested 关窗 flush 实测挂起关闭（探针 AFTER-CLOSE=1，Tauri 2 该 API 把关闭权移交 webview destroy 路径；含 no-op handler 对照实验排除 flush 本体），定案回退为纯 800ms 防抖自动保存 + 组件常驻挂载（v-show，切页不卸载、防抖计时不中断），丢字窗口仅"输入后 0.8s 内即退出"，关窗强 flush 登记已知局限待后期 Rust 侧方案；修复后探针 AFTER-CLOSE=0 恢复正常关闭；live 移交统一目验）
- [x] PL005.4 PL005 收口 —— 门禁全量 + 结论回写 + 勾结；验证：门禁全绿 + live 白板链路（2026-09-17 已验证：门禁七项全绿（44 测试）+ 全组勾结；live 移交统一目验）

### PL006: 二期收口 [plan#二期]

> 范围：全量门禁 + 二期验收 + 版本推进 + A002 审计修复 + 状态回写；方案见 z.plan.md 附录 PL006。
> 红线：沿一期"审计先行后收口"节奏；版本推进 0.1.0 → 0.1.1（R 回 1）。

- [x] PL006.1 二期验收与门禁 —— 全量门禁七项 + 二期验收清单逐项（①捕获剪贴板成气泡 ②非文本剪贴板提示 ③点击气泡复制回 ④单条删除 ⑤满 5 横幅 ⑥清空二态确认 ⑦白板自动保存 ⑧关窗/切页不丢 ⑨页签切换下一期功能零回归）；验证：门禁全绿 + 验收清单 live 逐项（用户目验）（2026-09-17 已验证：门禁七项全绿 + 验收清单 9 项经用户实机目验全过（含 FIX002.1 气泡点击不拖窗与 FIX002.2 超长提示两项修复抽验））
- [x] PL006.2 版本推进 —— `core/Cargo.toml` version "0.1.0" → "0.1.1"（R 回 1，二期 milestone）；README 徽章同步 0.1.1.1；验证：cargo test/build 绿 + 文档一致性（tauri.conf.json 仍省略 version 回落 Cargo.toml）（2026-09-17 已验证：Cargo.toml 0.1.1 落地 + README 徽章 0.1.1.1 同步 + 45 项测试/build 绿）
- [x] PL006.3 首轮二期审计 A002 —— 用户触发 audit-project 全量审计 → audit-report 归档（A002/FIX002）→ FIX002 修复闭环；验证：报告四节完整 + FIX 条目可执行并修复（2026-09-17 已验证：A002 归档 z.plan 附录（四节完整，观察项 2 新增 + A001 10 条延续保留观察）+ FIX002 五条落本文件并修复闭环）
- [x] PL006.4 二期收口回写 —— README/AGENTS/计划书 §6 状态回写（二期完成）+ 勾结 + commit 草案（feat: V0.1.1.1，二期全量一次性提交——用户定案节奏）交用户；验证：门禁全绿 + 文档一致性核对（2026-09-17 已验证：README 徽章/状态行、AGENTS 状态行、计划书 §6 二期勾选全部回写二期完成态 + 全组勾结 + PL004–FIX002 四组移入已完成区；commit 草案（feat: V0.1.1.1）已交用户）

### FIX002: 第2轮审计修复 [audit#A002]

> 范围：A002 报告（2026-09-17）P2 两项 + P3 两项 + 收尾验证；无 P0/P1。
> 红线：审计修复不引入行为变化（P2 交互修复与长度上限除外——缺陷对象本身）；保活不回归；零新依赖。

- [x] FIX002.1 [P2] 气泡行入拖动白名单 —— `ui/App.vue:67` DRAG_INTERACTIVE 常量追加 `.bubble-row`（沿 `.todo-row` 先例；现按住气泡行会 startDragging 吞 click，复制回不可靠）；验证：live 点击气泡稳定复制回剪贴板不拖窗 + 空白处拖动与清单行勾选不回归（2026-09-17 已验证：白名单在位（grep 实证）+ npm build 绿 + exe 重建冒烟过（ALIVE/AFTER-CLOSE=0）；**live 两项经用户目验确认**）
- [x] FIX002.2 [P2] 气泡长度上限 —— `core/src/bubble.rs` 增 `MAX_BUBBLE_TEXT_LEN: usize = 2_000`（注释：气泡 = 短片段语义，可调）+ `BubbleError::TooLong` 变体 + validate_bubble_text 增超长拒（chars().count() 比较）；bubble.rs tests 增超长拒用例；验证：cargo test 先红后绿 + live 粘贴超长剪贴板捕获 → 错误行显示上限提示（2026-09-17 已验证：红灯 E0425/E0599 → 转绿（含恰 2000 过界断言）；45 项全绿；错误路径经 CommandError::Clipboard 纯字符串契约可读（A001-P3-1 契约延续）；live 移交统一目验）
- [x] FIX002.3 [P3] count_bubbles 死代码清除 —— `core/src/storage.rs:202` 删除 count_bubbles 方法（满 5 判定走 list_bubbles().len()，生产零调用）；bubble_clear_returns_count_and_empties 测试的 count 断言改用 list_bubbles().len()；验证：cargo test 全绿 + grep count_bubbles 零残留（2026-09-17 已验证：方法删除 + grep 归零 + 45 项全绿无回归）
- [x] FIX002.4 [P3] AGENTS 二期实态回改（与 PL006.4 收口同批执行）—— 目录树"2026-09-17 一期实态"改"二期实态"并补 bubble.rs/whiteboard.rs/commands/bubble.rs+whiteboard.rs/BubblesView.vue/WhiteboardView.vue；技术栈"通知常驻"行回填实态（一期定案：无系统通知常驻）；验证：人工核对树与实态一致 + grep "待一期方案定案后补充" 零残留（2026-09-17 已验证：树改"二期实态"逐行核对（src 补 bubble/whiteboard、storage 三表、commands 三分文件、ui 补两视图）+ 技术栈行回填 + grep 归零 + prettier 绿；PL006.4 收口时随状态行一并复核）
- [x] FIX002.5 收尾验证 —— 全量门禁（fmt --check / clippy -D warnings / cargo test / doc / vue-tsc / npm build / prettier）+ 反向验证逐条（原问题"气泡点击被拖拽劫持"→验收"live 点击稳定复制"；原问题"无长度上限"→验收"超长捕获可见拒"；原问题"死代码"→验收"grep 归零"；原问题"文档滞后"→验收"树实态一致"）+ A002 状态行回写（📌 → ✅）+ 勾结；验证：门禁全绿 + 反向验证清单逐项过（2026-09-17 已验证：门禁七项全绿 + 反向验证逐条（白名单 live 确认 / 超长拒绝 TDD 闭环 + live 确认 / count_bubbles grep 归零 / 文档树核对）+ A002 状态行回写 ✅；记录 .temp/fix002-verification.md；全组 5 条勾结，V0.1.1.1 随二期收口统一提交）

## 未完成

### PL007: 界面实验场 design/ [plan#工具]

> 范围：design/ 独立可交互原型（三页签复刻 + glass.css 令牌化 + README 映射表 + RESEARCH.md 同步 + 背景素材）；方案见 z.plan.md 附录 PL007。
> 红线：不动 ui/ 一行（落地回 ui/ 属后续单独 PL）；design/ 不进 vite 构建与打包；完成后不审计（用户定案）。

- [x] PL007.1 骨架与素材 —— 新建 `design/`（index.html / glass.css / README.md / RESEARCH.md / assets/）；用户两张照片（F:\Download Pool\project_avatar\avatar002/005.jpeg，2304×1728）经 PowerShell System.Drawing 转宽 1600 q80 小 jpg 入 assets/（转换脚本落 .temp/）；RESEARCH.md 自 CapsulePulse 同名文件同步（R001 uiverse galaxy 全量内容 + 出身注记）；验证：文件齐备 + jpg 体积对比源图显著缩小（2026-09-17 已验证：bg-cafe.jpg 348KB / bg-sunset.jpg 230KB（源 5.7MB/4.2MB，缩放 1600×1200 q80）+ 五文件齐备 + RESEARCH 出身注记与关联行按本项目改写）
- [x] PL007.2 基准配方与三页复刻 —— glass.css 令牌 1:1 移植现行 App.vue（30% 分态纱/亮边/rim/落影/accent，深浅双主题）+ 背景图接管 + 聚焦态预览（backdrop-filter 近似磨砂）；index.html 三页签完整交互复刻（清单添加/勾选/折叠/删除、气泡演示捕获/复制回/满 5 横幅/二态清空、白板防抖状态行——纯客户端模拟状态，演示捕获 = 示例片段池）；验证：浏览器打开可交互 + 深浅双主题（2026-09-17 已验证：glass.css 令牌 1:1 移植 + index.html 三页复刻完成（演示捕获 = 示例片段池、聚焦态 backdrop 近似含实验场控件）；内联 JS 语法自检过（node Function 编译）+ prettier 全过；**浏览器目验移交用户**——交互观感以用户浏览器为准）
- [x] PL007.3 映射表与收口 —— README 职责表/基准配方读数/查看方式/映射表（原型七区块 → Vue 组件 → 落点）/真实窗口可行性标注（✅⚠️❌ 三档）；勾结；不做审计（用户定案）；uiverse 组件五处引入（R001 规范：类名隔离/颜色令牌化/reduced-motion 守卫，定值经用户逐处目验收敛）：页签 heavy-dragonfly-92（glider 滑块 + 同色辉光 + 气泡徽章固定正圆）、输入框 plastic-parrot-88（聚焦环随主题 accent）、勾选框 hot-dragonfly-56（常态透明底，行 overflow:clip 裁装饰防幻影滚动）、删除按钮 smart-emu-83（25% 透明黑底、中心缩放向两侧展开覆盖文字 + 红辉光、悬停文字保留组件原文 "Delete"）、添加按钮 REC average-swan-99（Teenage Engineering EP-133 源码 100% 直引：黑方/红方/居中 REC/落影 rgba(0,0,0,0.566) 1.7px 1.7px 4px 外框标定，均用户定值）；REC 玻璃配方推广全玻璃板（--panel-bg 深度 ×2 + 新令牌 --panel-shadow，最终 rgba(0,0,0,0.35) 1.7px 1.7px 8px——模糊须盖过偏移防贴边暗环）；清单分组容器 8px 内边距 + 虚拟外框外扩（自动宽 + 左右负外边距 8px：框距卡边恒 10px、框宽 = 卡宽 − 20 随卡片动态，行宽 = 卡宽 − 36 与其他面板等宽；顶边上移 8px、底随内容）；配色基准 design/color_spark.md（暮色截图实测取色指南：色板/令牌映射建议/渐变配方，accent 分装饰与交互两层）；验证：映射表覆盖全部原型区块 + 用户浏览器目验持续进行（2026-09-17 起——实验场本身就是目验载体，目验反馈十余处均已修复收敛）

### PL008: APP 回归·玻璃基座与通用组件族 [plan#APP回归]

> 范围：实验场形态落回 ui/ 的基座层——DEV 冒烟基座、玻璃令牌、App.vue 骨架、四个通用组件；方案见 z.plan.md 附录 PL008。
> 红线：uiverse 组件忠实移植（R001 规范）；design 定案参数不改；mock 通道禁进生产；每 PL 自验三道闸后提交推送 V0.1.1.5（用户定案 2026-09-26：主分支直干、中间零人工验收、PL014 总验收）。

#### 阶段 A：冒烟与令牌基座

- [x] PL008.1 DEV 冒烟基座（mock invoke 通道）—— 新建 `ui/src/dev/mock-invoke.ts`：导出 `installMockInvoke()`，内存态维护三源（todos/bubbles/whiteboard，种子沿 design/assets/js/state.js 的 8 清单 + 5 气泡 + 长文白板同款），`mockInvoke(cmd, args)` 覆盖既有十命令（todo_list/todo_add/todo_toggle/todo_remove/bubble_snapshot/bubble_add/bubble_remove/bubble_clear/whiteboard_load/whiteboard_save），PL010–PL013 将增命令留扩展位；`ui/main.ts` 顶部 DEV 分支（`import.meta.env.DEV` 且 `window.__TAURI_INTERNALS__` 缺失时）注入假 internals 路由 invoke 到 mock；验证：`npm run build` 绿（vue-tsc 含，DEV 死分支被 vite 消除——dist 产物 grep 无 mockInvoke 字符串）+ vite dev 起后 IAB evaluate 断言：todo_list 返回 8 条种子 / todo_add 后 9 条 / todo_remove 回 8 条（2026-09-26 已验证：build 295ms 绿 + dist 零 mock 痕迹（生产消除实证）+ vite dev（localhost:5173——**vite 默认只绑 localhost 不通 127.0.0.1**）IAB 全断言过：种子 sorted 视图 10 条（8 活跃+2 归档含）/ add→11 / remove→10 / toggle 往返 / 空白拒 / 气泡快照 5 条 remind 翻转 / 白板读回；注意清单页可见 8 条 = 种子 4 未经 sorted；校验阈值对齐 Rust 实测：气泡 2000（MAX_BUBBLE_TEXT_LEN）/ 待办 100）
- [x] PL008.2 玻璃令牌层落位 —— 新建 `ui/src/styles/glass.css`：design/glass.css 全量令牌 1:1 移植（分态纱/亮边/rim/落影/accent 双主题/panel-bg/panel-shadow/radius 族，值零改动）；main.ts import 链首位挂载；App.vue 内联玻璃值改令牌引用（等值替换不改观感）；style.css 清理重复；验证：vue-tsc/build 绿 + IAB 双主题截图与 design 同页并排取色断言（卡片底/描边 rgba 逐项一致，prefers-color-scheme 翻转复测）（2026-09-26 已验证：build 257ms 绿 + IAB 令牌读数断言过——暗色组 accent #c0b0fd / panel-bg 0.14 / panel-shadow 0.566·1.7px·4px / glider-bg 0.3 全对，卡片圆角 8px 生效 = scoped→令牌链路通；文件头注明单一来源 = design/glass.css 及两处环境差异（无 body[data-bg] 演示背景段/APP 跟随系统单 dark 块——design 的手动 data-theme 双块属实验场控件，APP 无主题切换按定案不迁））

#### 阶段 B：骨架与组件

- [x] PL008.3 App.vue 骨架重构与窗口尺寸 —— App.vue template 重写为 design/index.html 骨架 Vue 版：topbar（标题 + data-tauri-drag-region 拖动区）、TabsBar 挂点、`#board` 卡片容器（position:relative 锚点保留——浮板/浮钮/三角宿主）、三 page 容器 v-show；全局 mousedown startDragging 收敛为 topbar 拖动区（交互件天然不误触）；`core/tauri.conf.json` 窗口宽改实验场卡宽实测定值（实施时 IAB 量 design `#board` offsetWidth + 外距）、高 580，resizable 沿 false；置顶/让位/位置记忆零改动；验证：build 绿 + cargo check 绿 + IAB 三 page hidden 切换断言 + topbar 拖动属性在位 + tauri dev exe 探针（标题/5s 存活/GetWindowRect 尺寸，脚本落 .temp/）（2026-09-26 已验证：build 255ms 绿 + cargo check 4.9s 绿 + IAB 断言过（#board 玻璃卡锚点/topbar 拖动属性/三页 v-if·v-show 切换互斥/清单 10 行与气泡 5 行 mock 渲染）+ exe 探针 PASS（标题 CapsuleTODO、客户区 300×580 精确一致、二次采样存活稳定）；**两项实测教训入档**：① mock 基座补全 internals 面——listen 缺 transformCallback 抛错致 onMounted 中断（await listen 后的 refresh 永不执行 = 页面恒空态）、plugin:* 命令（event|listen / window|is_focused）需假响应路由，均已在 mock-invoke.ts 修复；② 探针判定改 bash 侧——GetWindowRect 含透明无边框窗口 14×8 hit-test 外扩（客户区 GetClientRect 才等于配置值），且文件内 if 分支在 -File 上下文不可靠（同数据内联 PASS 文件 FAIL，原因未明不再追），探针只输出数据、判定交调用方）
- [x] PL008.4 TabsBar 页签组件 —— 新建 `ui/src/components/TabsBar.vue` + `ui/src/styles/tabs.css`（design/assets/css/tabs.css 1:1）：uiverse heavy-dragonfly-92 glider——radio 组改 modelValue props + update:modelValue emit（滑块 left 过渡 CSS 保留）；气泡徽章（正圆、count prop、0 隐藏）挂气泡页签；App.vue 接线（v-model + 徽章接气泡计数）；验证：build 绿 + IAB：切页断言 page hidden 翻转 + 滑块位移读数 + 徽章数与 mock 一致 + 0 时隐藏（2026-09-26 已验证：build 绿 + IAB 全断言——徽章 "5" 显示且正圆/滑块三档 matrix 0/80px/160px（=0%/100%/200%）/切页往返/回清单 10 行；**Vue 教训入档**：tabDefs 写 `badgeCount: bubbleCount.value` 是一次性快照（ref 更新不回写静态数组）致徽章恒隐——改 computed 坐标系响应式；vite 根 = 仓库根，组件 style 内 @import 相对路径自组件文件起算 `../styles/tabs.css`，写 `./styles/` 会落错目录 ENOENT）
- [x] PL008.5 NeonCheckbox 勾选框组件 —— 新建 `ui/src/components/NeonCheckbox.vue`（hot-dragonfly-56 全装饰层 1:1：frame/box/check-container/glow/borders 四条/particles 十二颗/rings 三环/sparks 四点）+ 样式段并入 `ui/src/styles/todos.css`：checked prop + toggle emit；勾选流光 borderFlow1-4 随 checked 启停（V0.016 ④ 定案：四条 nth-child 域 + keyframes 透明度节点 0/15/55/80%）；行 overflow:clip 裁装饰防幻影滚动；验证：build 绿 + IAB：checked 翻转四条 border 域 animation-name 断言（getComputedStyle）+ 未勾选空跑不亮 + reduced-motion 下 animation none（2026-09-26 已验证：build 274ms 绿 + 样式文件经 vite 直读核对完整（borderFlow1-4/粒子 12 颗 nth-child(12)/sparkFlash/ring 三档延迟 0/0.1/0.2s 全在，8830 字节）+ 页面零编译错误；**行为断言顺延 PL010.4**——组件尚未接清单行，tree-shake 使 scoped 样式不入 bundle，裸 DOM 注入命不中 hash 选择器，届时随行接线一并验证流光启停与 reduced-motion；**教训**：sed 行区间抽取截断 ring 延迟段，python 按行切片 + 尾部人工核对才完整）
- [x] PL008.6 DelButton 二态确认删除钮 —— 新建 `ui/src/components/DelButton.vue`：template = FA6 trash-can 双 path（桶体在前、盖 `.del-lid` 在后=绘制在上，d 值逐字抄 design 模板）；样式抽 `ui/src/styles/del-button.css`（.del 族 1:1：hover 展开 35px 红底辉光/中心缩放 translateX(5px) 右缘补偿/Delete 字 ::before/del-open 锁红底/盖翻 0.5s cubic-bezier(0.34,1.4,0.64,1) origin 24px 64px/开盖态 svg overflow visible/del-press 脉冲关键帧整链携带 translateY(-50%) translateX(5px)/合盖无动画）；交互：confirming prop（父级管单实例）+ press/confirm emit（**confirm 在 200ms 执行窗后触发恰一次**——组件内 pendingConfirm 锁 + timer，onBeforeUnmount 清理）+ `rollBack()` expose + 脉冲 remove+回流重播 + reduced-motion 跳过；验证：build 绿（vue-tsc 含）+ 组件编译与样式落位断言过；**行为断言（开盖/盖翻/脉冲/窗口锁/离开回退）顺延 PL010.4**——组件未接清单行前无宿主（同 PL008.5 顺延逻辑），接线后随行一并全链路验证（2026-09-26）
- [x] PL008.7 AddBar 重构（REC + 输入框）—— `ui/components/AddBar.vue` 重写：REC 添加钮（average-swan-99 直引配方：黑方/红方/居中 REC/落影 rgba(0,0,0,0.566) 1.7px 1.7px 4px）+ 输入框（plastic-parrot-88：聚焦环随主题 accent、label 上浮）+ 空输入禁用；新建 `ui/src/styles/addbar.css` 1:1；验证：build 绿 + IAB：空输入 disabled / Enter 与 REC 点击均走 todo_add（mock +1）/ 聚焦环类断言 / 双主题取色（2026-09-26 已验证：build 281ms 绿 + IAB 断言——REC 结构（btn-add/btn-add-face/btn-add-text）与空输入 disabled ✓/输入后启用 ✓/点击入列 11 行 ✓/成功清空 ✓/输入框几何（22px 高/9999px 胶囊/panel 阴影）✓；**聚焦环与双主题留 PL014**：遮挡 IAB 渲染器无系统焦点，:focus 伪类不触发（matches(":focus")=false 实测），样式规则直读核对在位（.input:focus → var(--input-focus-ring)）；**事故教训**：仓库同时存在 ui/components/（旧）与 ui/src/components/（新）双目录，App.vue import 指向旧 AddBar 致新组件"不生效"假象——PL010 换装 TodoList 时必须同步删旧组件防再次误引）

#### 阶段 C：收口

- [x] PL008.8 PL008 收口提交 —— 全门禁（cargo fmt --check + clippy -D warnings + cargo test + npm run build）+ 三道闸复核 + AGENTS.md 状态头推进 + z.plan 附录 PL008 状态 ✅ + 本组勾结回写；提交 `feat: V0.1.1.5，玻璃基座与通用组件族回归` 推送；验证：门禁全绿 + push 成功 + git status 干净（2026-09-26 已验证：fmt/clippy/test/build 四件套全绿（test 三套件 1 用例过——既有 storage_probe；前端 vue-tsc 含 build 240ms）+ prettier 全部文件过 + push 成功（2ebcc40..c932267）+ 工作区干净；Mimosa 钩子 medium 警告照旧为 core/target/doc rustdoc 生成物误报）

### PL009: APP 回归·横切工厂组合式化 [plan#APP回归]

> 范围：design 四大命令式工厂转 Vue composables + board-read 样式；方案见 z.plan.md 附录 PL009。
> 红线：算法与参数 1:1 移植实测定案值（数值不改——六轮拖拽失败教训）；不依赖 transitionend（遮挡 webview 不派发），一律显式 duration。

- [x] PL009.1 useGlassBar 玻璃滑杆 —— 新建 `ui/src/composables/useGlassBar.ts`：`useGlassBar(scrollerRef, opts)`（anchor/inset/right 同 design makeGlassBar）；浮钮 DOM 组合式创建挂 anchor（默认 #board）；sync 闭包（height 0 隐藏/轨道内缩/scrollable 判定/比例位移）+ scroll/input 监听 + MutationObserver(childList) + pointerdown 拖拽（公式 1:1 + textarea 合成 scroll 补发）；导出注册表 `glassBars`（useVeils 用）；onUnmounted 清监听清 DOM；验证：build 绿 + IAB（mock 灌 >5 条）：浮钮出现 + thumb top 随 scrollTop 三点线性断言 + pointermove 拖拽序列 scrollTop 断言 + 卸载后 observer disconnected 无泄漏（2026-09-27 已验证：build 绿 + 探针页（.temp/probe-pl009.html + ui/probe-pl009.ts 临时挂载面，验后已删）断言——浮钮显示/三点线性 0/191/385（中点居中差 ≤2px）/拖拽上滚 14px/veil 挂摘全过；**罩死钩子解耦**：design 的全局函数松耦合改为 registerMaskDeadHook 注册（防 composables 循环 import），scroll/MutationObserver 双挂点经钩子同频触发）
- [x] PL009.2 useBoardRead 整板阅读 —— 新建 `ui/src/composables/useBoardRead.ts`（design makeBoardRead 1:1）：opts（padB/rowSel/gate/layout/hintHost/skipDuringDrag/maskShift）同名；▲▼ 三角挂 hintHost（默认 #board）；sync 双分支（layout||hintHost 走 offsetParent 链累加至宿主 / gBCR 基准 = hintHost rect）；settle（到底 at-bottom + --fade-btm 100%/半截行 fadeStart/空容器早退补 sync）；scroll 监听（实时抬带 + maskShift + sync）+ scrollend + MutationObserver；返回 { sync, settle, hints } + 注册表 boardReads；新建 `ui/src/styles/board-read.css`（.board-read/.edge-hint/edge-blink/@property --fade-btm 与 --mask-shift 族 1:1 含 [hidden] display:none 显式）；验证：build 绿 + IAB 直呼断言：半截行内联 --fade-btm = 行顶 − 容器顶 − 2 / 到底 at-bottom + 100% / 滚回摘类 / 三角 hidden 随 scrollTop 翻转 / hintHost 实例坐标手算 offset 链对表（2026-09-27 已验证：build 绿 + 探针页直呼断言——到底 settle at-bottom 类 + --fade-btm=100% + ▼熄▲亮 / 回顶类清默认带 / 滚 60px ▲亮▼亮 / maskShift 分支在位；**拖拽互斥解耦**：dragCtx 全局直读改 registerDragProbe 注入（PL013 useDragReorder 注册）；**断言语义教训**：顶部 ▲ 本就该熄（scrollTop=0 上方无内容）、▼亮（下方有内容）——首验写反断言误报失败，语义以 design showUp=scrollTop>2 为准；遮挡环境 scroll 不派发，判定走直呼 sync/settle）
- [x] PL009.3 useMaskDead 罩死判定 —— 新建 `ui/src/composables/useMaskDead.ts`：常量 RATIO 0.38/HYST 0.36/BAND_TOP 6/BAND_BTM 14 + rowInvadeRatio（**at-bottom 例外不计底带——V0.027 修复版逐行移植**）+ rowMaskDead + syncMaskDead（双列表核算、[hidden] 祖先跳过）；挂 boardRead scroll 链（注册表对接）；灰化样式（.mask-dead 三件套 + path fill 直改灰 + 恢复渐变落常态——双向 0.5s）；验证：build 绿 + IAB（灌列表 + 直呼 syncMaskDead + 手动 settle）：首行侵入 44% 挂类 / 滚回 36% 下迟滞摘 / at-bottom 末行不罩死（回归）/ .del pointer-events none + cursor default 断言（2026-09-27 已验证：探针页断言——首行侵入 50% 挂 mask-dead / 回顶迟滞摘除 / at-bottom 末行存活（V0.027 修复回归）全过；灰化样式随 PL010.4 行接线落 todos.css 对应段——组件未挂行前无宿主，同 PL008.5 顺延逻辑）
- [x] PL009.4 useVeils 浮板帘联动 —— 新建 `ui/src/composables/useVeils.ts`：`syncVeils()`（glassBars：archive-list 实例 veil = !boardOpen 其余 anyOpen；boardReads：archive 实例显式 continue——V0.028 结构性豁免 / detail-note 实例 !detailOpen / 其余 anyOpen）；浮板组件挂载时 `bindOverlayState(name, read)` 注入开合读取器（组合式不持浮板 DOM 引用——Vue 组件化后浮板开合态属组件内部 ref，design 的全局 classList 直读不可用）；验证：build 绿 + IAB：开详情板清单滑杆三角挂 veiled、归档三角不挂 / 关板摘 / 三板互斥组合断言（2026-09-27 已验证：探针页 bindOverlayState("detail") 假浮板断言——开 detail 滑杆 veiled + 三角 veiled / 关板双摘全过；归档豁免分支代码在位、归档板组件 PL011 挂载后随板验证）
- [x] PL009.5 PL009 收口提交 —— 同 PL008.8 体例：全门禁 + 回写 + `feat: V0.1.1.6，横切工厂组合式化` 提交推送；验证：门禁全绿 + push 成功（2026-09-27 已验证：cargo fmt/clippy/test 全绿（未触 Rust 零回归）+ npm build 309ms 绿 + prettier 全过 + 探针临时文件全清（probe-pl009.html/ts 删除）+ 工作区仅含本组四 composable + board-read.css + main.ts 挂载行）

### PL010: APP 回归·清单页与数据迁移 [plan#APP回归]

> 范围：todos 表三列迁移 + age/note 纯逻辑 + 命令扩容 + TodoList/详情板回归；方案见 z.plan.md 附录 PL010。
> 红线：迁移保存量行幂等；业务零进 Vue；TDD 红灯先行。

#### 阶段 A：Rust 数据层（TDD）

- [ ] PL010.1 todos 表迁移 —— `core/src/storage.rs`：init() 后加 `fn migrate(&self)`——`PRAGMA table_info(todos)` 读现列集合，缺列 ALTER TABLE ADD：created_at INTEGER（存量回填 NULL 不模拟时间）、done_at INTEGER、note TEXT NOT NULL DEFAULT ''（回填空串）；`core/src/todo.rs` TodoItem 扩 `created_at: Option<i64>`/`done_at: Option<i64>`/`note: String`（serde 同步）；Storage 构造注入时间源（`now: Box<dyn Fn() -> i64 + Send>` 默认 SystemTime——add 落 created_at）；list/get 行映射补三列；先写测试：内存库手工建旧 schema → open → 新列就位 / 存量行 note='' 与 created_at NULL / 二次 open 幂等 / 新 add 行 created_at 非 NULL；验证：红灯（旧库缺列断言失败）→ 转绿全过
- [ ] PL010.2 age 阈值与 note 校验 —— `core/src/todo.rs`：`enum AgeLevel { None, Yellow, Red }`（Serialize+Copy）+ `pub fn age_level(created_at: Option<i64>, now: i64) -> AgeLevel`（None→None；> 48h 红；> 24h 黄；常量注释注明与实验场 24/48h 定案同源）+ `MAX_NOTE_LEN: usize = 10_000` + `pub fn validate_note(s: &str) -> Result<(), TodoError>`（超限 Err，允许空白——笔记语义）；测试：None / 恰 24h 不黄 / 24h+1 黄 / 48h+1 红 / note 恰限过 / 超限 Err；验证：红灯→绿（now 全注入零真实等待）
- [ ] PL010.3 命令层扩容 —— `core/src/commands/todo.rs`：todo_rename(id, text)（复用 add 文本校验）/ todo_set_note(id, note)（validate_note）/ todo_toggle 改造（置 done 同时 done_at：true→now、false→NULL，storage.toggle 扩参）/ todo_list 条目平铺 `age_level: AgeLevel`（命令层裁决注入——DTO 单一来源不破坏）；storage 补 rename/set_note（参数化 SQL）；`ui/types.ts` 镜像同步（TodoItem +三字段 +age_level）；测试：rename 校验复用 / set_note 往返 / toggle 往返 done_at 有无 / list age_level 组包（注入老 created_at 断言黄红）；验证：cargo test 全绿 + cargo doc --no-deps 零 broken link + mock-invoke 同步四命令

#### 阶段 B：Vue 回归

- [x] PL010.4 TodoList.vue 回归 —— `ui/src/components/TodoList.vue` 重写：行模板（NeonCheckbox + t-text + DelButton + age-alert 黄红文案 1:1——age_level 驱动）+ `<TransitionGroup name="todo" :duration="320">`（key=id；enter/leave 类沿 design：scale 0.6 回弹入场 / 钉高收 0 + flex-shrink:0 塌缩，CSS 1:1）+ 罩死守卫 + DelButton 单实例管理（stale 收口 + 换页/浮板开合 rollBack）+ 勾选 300ms 主拍后离场（显式 timer）+ 清单分组虚拟外框（框宽 = 卡宽 − 20 动态公式）+ 空态壳；样式扩 `ui/src/styles/todos.css`；验证：build 绿 + IAB mock：增删勾全链路 + entering/leaving 320ms 自摘 + 罩死行点击无响应 + 黄红文案按 mock created_at 渲染 + 外框宽量测（2026-09-27 已验证：build 绿 + IAB——行模板三件（checkbox/t-text/del 双 path）✓、age-alert 黄+红各一（按 30h/50h 种子）✓、虚拟外框 margin -8/gap 6 ✓、两拍删除 mock 9→8 ✓、单实例（点第二行首行自动回退）✓、DOM 与 mock 终态收敛 domMatchesMock=true ✓；**排障教训三连**：① 行数断言错位根因 = TodoList 只渲染 active（done 过滤），mock 排序视图含归档行——断言必须同语义过滤；② 锚定行用 id+1 猜测在 id 不连续种子下必错——必须按 data-row-id 实锚；③ 遮挡渲染器 rAF 冻结使 TransitionGroup leave 摘除延迟（DOM 滞后 mock 数秒，最终收敛）——行为终判以 mock 数据为准、DOM 摘除真实窗口目验；**Vue 教训**：defineEmits 只能调用一次（重复调用 compiler 报错），多事件并入同一类型字面量）
- [x] PL010.5 DetailOverlay 详情板（todo 模式）—— 新建 `ui/src/components/DetailOverlay.vue`（双模式骨架，本条落 todo 模式、bubble 分支 PL012 填）：板揭示（origin 变量 + 0.4s 回弹）、标签头 + 标题 input（maxlength 12 实时 todo_rename + 清单行同步）、note textarea（防抖 300ms todo_set_note）、三板互斥（开前收归档/设置）、单击行 180ms 延迟开板（mousedown/双击可取消——消歧沿定案）、双击行内改标题（span→input 原位）、飞出原点、板内整板阅读（gate + layout）+ 滑杆；syncVeils 挂点；验证：build 绿 + IAB：单击 180ms 开板 / 双击行内编辑且板不开 / 标题改清单同步 / note 防抖读回 / 板外收板 / veil 挂清单滑杆三角（2026-09-27 已验证：build 444ms 绿 + IAB 断言——单击 180ms 后 open ✓ / 标题装载行文本 ✓ / 笔记种子装载 ✓ / 改标题防抖后清单行同步 ✓ / 改笔记防抖后 mock 落库 ✓ / 板外 mousedown 收板 ✓（capture 守卫，板内点击不收）；**接线方式**：App.vue 持 detailTodo ref，行 openDetail 事件上抛置值；板内 useBoardRead/useGlassBar 首帧挂载（textarea 出现后才有滚动几何）+ 450ms settle 重算；三板互斥待 PL011/PL014 归档/设置板挂载后接 bindOverlayState 全链）

#### 阶段 C：收口

- [x] PL010.6 PL010 收口提交 —— 全门禁 + 回写 + `feat: V0.1.1.7，清单页回归与数据迁移` 提交推送；**旧库迁移专项**：.temp/ 造旧 schema db 文件真实 open→断言→关（文件级冒烟非仅内存）；验证：门禁全绿 + push 成功 + 迁移专项过（2026-09-27 已验证：cargo fmt/clippy/test（61 用例）/doc 四件套全绿 + prettier 全过 + **文件级迁移专项 PASS**（.temp/run-migrate-probe.mjs 动态生成 core/tests/migrate_probe.rs 真实 SQLite 文件——旧 schema+存量数据+白板 → open 就地迁移 → 存量行保留/created_at·done_at NULL/note 空串/新增行走新字段/白板不受伤/重开幂等，六断言全过，跑完探针即删）+ push 成功）

### PL011: APP 回归·归档板 [plan#APP回归]

> 范围：归档视图命令 + ArchiveOverlay 全形态；方案见 z.plan.md 附录 PL011。
> 红线：归档行点正文 no-op、归档态 note 不可见（V0.015 用户定案）。

- [ ] PL011.1 归档视图命令 —— `core/src/storage.rs`：`pub fn list_done(&self)`（`WHERE done=1 ORDER BY done_at DESC, id DESC`——同秒按 id 稳定序）；`commands/todo.rs`：todo_archive_list 命令；测试：不同 doneAt 倒序 / 同 doneAt 按 id 倒序 / 空集空 vec；验证：红灯→绿 + mock 同步
- [ ] PL011.2 ArchiveOverlay 组件 —— 新建 `ui/src/components/ArchiveOverlay.vue` + `ui/src/styles/archive.css`（boards.css + archive-fx 形态 1:1）：归档按钮（archive 图标出入场：吸气 1.15→塌缩 0 + 粒子迸裂）、板揭示（--origin-x/y 注入 + 0.4s 回弹）、title-plate 标题玻璃板（「已完成 N」计数）、行（NeonCheckbox 退回 + t-text 删除线 + DelButton 二态）、空态、增量摘除退场（TransitionGroup duration）、板开合 450ms 后逐实例 settle + 滑杆 sync（揭示中几何中间态防御）、滑杆锚板内玻璃 + 三角 hintHost 板内（V0.028 方案 B 局部坐标）、行距 6px/落影缓冲 -8px/底缘软边 12px（V0.016 定值全保留）；点板空白与再点按钮双出口收板；验证：build 绿 + IAB：开板 + 450ms settle spy / 行数一致 / 退回一条板内离场 + 清单 +1（done_at NULL）/ 删除二态 / 计数 / 空态翻转 / 三角局部坐标手算对表 / 点正文零响应
- [ ] PL011.3 勾选入档两拍编排 —— TodoList.vue：勾选 → 流光 300ms 主拍（显式 timer）→ 塌缩离场 → 归档侧重拉或计数就地 +1（板开时清单被遮不可见、板内浮现不做——V0.015 定案注释落码）；退回 = 板内塌缩 + 清单 popIn（entering 重放）；验证：IAB：勾选 320ms 内行仍在 / 之后离场 + done 集 +1 / 退回逆流程 / 快速连勾两行在飞动画互不拔除（V0.014 增量摘除回归）
- [ ] PL011.4 PL011 收口提交 —— 同体例：全门禁 + 回写 + `feat: V0.1.1.8，归档板回归` 提交推送；验证：门禁全绿 + push 成功

### PL012: APP 回归·气泡页与剪贴板真链路 [plan#APP回归]

> 范围：clipboard 插件 + BubblesView 回归 + 全文板 bubble 模式；方案见 z.plan.md 附录 PL012。
> 红线：满 5 阈值 Rust 裁决不漂移；剪贴板真链路 IAB 测不了——UI 走 mock、真链路 PL014 用户验收。

- [ ] PL012.1 剪贴板插件与捕获命令 —— `core/Cargo.toml` 加 tauri-plugin-clipboard-manager（官方 2.x 锚定）；lib.rs run() 注册 plugin；`commands/bubble.rs`：bubble_capture()（读剪贴板文本 → validate_bubble_text → add_bubble → 快照返回）/ bubble_copy(id)（get_bubble → 写剪贴板）；命令层抽 ClipboardReader trait（生产 plugin 实现 / 测试固定实现——可测性）；`capabilities/default.json` permissions 增 clipboard-manager:allow-read-text / allow-write-text；构建后核 `core/gen/schemas/acl-manifests.json` 权限事实源在位（ACL 静默拒教训）；测试：空文本拒 / 入列快照 remind 翻转 / copy 读回一致；验证：cargo test 绿 + acl-manifests grep 权限名 + mock 同步两命令
- [ ] PL012.2 BubblesView 回归 —— `ui/components/BubblesView.vue` 重写 + `ui/src/styles/bubbles.css`（1:1）：捕获钮（双图标 clipboard/clipboard-check + 占字态 1s 禁点 + 紫染 50%）+ 一键清空二态（宽度动画量宽法 Vue 版：量旧宽→锁宽→回流→设新宽 + 330ms 定时器解锁；confirming 50% 透明红；悬停感知 2s 超时 mouseenter 暂停；旁路取消挂捕获入口与行点击）+ 行（b-text line-clamp 2 + DelButton + **单击开板/双击复制 180ms 消歧——V0.028 对调语义**）+ 满仓警告红字（> 上限出现、文案 1:1、has-warning 两档偏移 -9.5/-4 + maskShift threshold 8.5 depth 6）+ 徽章计数接线 + 整板阅读（rowSel .bubble-row + 罩死）+ 0 气泡清空灰染 disabled；验证：build 绿 + IAB mock：捕获入列 remind / 灌 6 条警告 + 偏移量测 / 清空确认→集体塌缩→0 条灰钮 / 单击 180ms 开板 + 双击占字不开板 / 旁路三入口 / 罩死翻转
- [ ] PL012.3 DetailOverlay bubble 模式 —— DetailOverlay.vue 补 applyDetailMode("bubble")：bubble-mode 类（标签头 display:none + textarea 底透明 padding 0——单层玻璃文字直落板面）/ readonly + 去输入质感 / 落位原点双模式共用 / 板内滑杆 + 恒定软边带（textarea 退化顶 6 底 14）；验证：build 绿 + IAB：单击开板 readonly + head hidden / 长文（mock ~600 字）滑杆出现 + 拖拽比例 / 短文无滑杆
- [ ] PL012.4 PL012 收口提交 —— 同体例：全门禁 + 回写 + `feat: V0.1.1.9，气泡页回归与剪贴板链路` 提交推送；验证：门禁全绿 + push 成功 + ACL 事实源复核注记

### PL013: APP 回归·拖拽排序与持久化 [plan#APP回归]

> 范围：sort_order 列 + reorder 事务 + useDragReorder；方案见 z.plan.md 附录 PL013。
> 红线：算法参数一字不改（六轮失败教训）；拖拽期冻结列表响应式重渲染。

- [ ] PL013.1 排序持久化 —— `core/src/storage.rs`：migrate 扩段（todos/bubbles 各加 sort_order INTEGER，存量回填 = 现序号——读出重写幂等）；`pub fn reorder_todos(&self, ids: &[i64])`（事务：ids 与未完成集合一致性校验（长度 + 差集空——不符 Err 防丢行/幽灵 id）→ 逐条 UPDATE sort_order）；reorder_bubbles 同款（与全量气泡集）；list/list_done/list_bubbles 改 `ORDER BY sort_order`（done 项沿创建序不随归档变）；测试：旧库回填序 / reorder 后序断言 / 部分 id 集 Err / 重复 id Err / 事务失败回滚；验证：红灯→绿 + .temp/ 真实文件库迁移冒烟
- [ ] PL013.2 reorder 命令与 mock —— `commands/todo.rs` todo_reorder(ids) / `commands/bubble.rs` bubble_reorder(ids)（薄壳）；mock-invoke 同步（内存数组重排）；测试：命令直测 happy path；验证：cargo test 绿 + IAB mock reorder 前后序断言
- [ ] PL013.3 useDragReorder 组合式 —— 新建 `ui/src/composables/useDragReorder.ts`：DRAG_TARGETS 注册表 1:1 移植（清单+气泡：scopeId/rowSel/itemSel/ghostClass/onDown（清单掐 detailOpenTimer + 排除 del/t-edit/勾选区；气泡掐 bubbleCopyTimer + 排除 del）/commit（invoke reorder）/rerender（重拉））+ 机制本体（HOLD_MS 250 / DRAG_THRESHOLD 6 / 重挂 #board + ghost + shifting / applyDragShifts 让位公式 / 自动滚动 ZONE 32 SPEED 10 moved 3px 门槛 / 卡内钳制 12px / suppressDetailUntil 350ms / blur 收尾 dispatch mouseup）**数值公式逐行对照不改**；drag-live 锁滚动 + skipDuringDrag 对接；**Vue 冻结防线**：engaged 期间列表 watch 早退（外部变更到达不重渲染，落点收场统一重拉——守卫 defer）；罩死行不起拖；验证：build 绿 + IAB 合成序列（沿本窗口验证法）：长按 450ms engaged + ghost 在位 / 移动一槽 +15px 余量让位 transform 咬合 / mouseup 后 order 变换 + mock 落库一致 / 清理无残留 / 拖拽中注入变更不重渲染（守卫生效）/ blur 收场
- [ ] PL013.4 PL013 收口提交 —— 同体例：全门禁 + 回写 + `feat: V0.1.1.10，拖拽排序回归与持久化` 提交推送；验证：门禁全绿 + push 成功

### PL014: APP 回归·白板设置收口与总验收 [plan#APP回归]

> 范围：白板/设置板回归 + 全量回归 + 用户总验收移交；方案见 z.plan.md 附录 PL014。
> 红线：验收不达标不宣布完成；观察项沿 y.problems 登记。

- [ ] PL014.1 WhiteboardView 回归 —— `ui/components/WhiteboardView.vue` 重写 + `ui/src/styles/whiteboard.css`：board-shell 壳（panel 底/描边/落影/圆角静止）+ textarea 透明填壳（墨迹溶解与卡底分离——V0.021 定案）+ 整板阅读（textarea 退化恒定软边带 + 到底抬带）+ 板内滑杆 + 防抖 300ms 保存（既有保留）+ 状态行；验证：build 绿 + IAB：写入 300ms 后 mock 读回一致 / 长文溶解带内联值 / 到底抬带
- [ ] PL014.2 SettingsOverlay 与气泡上限配置 —— 新建 `ui/src/components/SettingsOverlay.vue` + `ui/src/styles/settings.css`（design 形态 1:1，实验场 lab 控件段剔除）：齿轮按钮出入场 + 板揭示 + 气泡上限步进（1~20 钳制）+ 主题跟随说明行；`core/src/settings.rs`：设置结构扩 `max_bubbles: u32`（serde default 5——旧 config.json 兼容）；`core/src/bubble.rs` should_remind(count, max) 参数化（AppContext 注入配置）；命令 settings_get/settings_set（沿 settings.rs 持久化管线扩）；测试：默认值 / 步进边界钳制 / 旧 config 缺字段不崩 / 阈值随配置翻转；验证：cargo test 绿 + IAB：步进断言 + mock remind 随上限变化 + veil
- [ ] PL014.3 全量回归与真窗口探针 —— `.temp/` 全交互走查脚本（IAB 顺序：清单增删勾/归档开合退删/气泡捕获清空双击/白板读写/拖拽两列表/罩死翻转/详情双模式/换页回退——逐项断言汇总）；tauri dev 真窗口 exe 探针自动（标题/存活/尺寸/无 panic 日志）；全门禁四件套；AGENTS 状态头收口（"PL008–PL014 完成，待用户总验收"）+ z.plan 七附录状态 + 本文件全勾结；提交 `feat: V0.1.1.11，白板设置收口与全量回归` 推送；验证：走查全绿输出留档 .temp/ + 探针过 + 门禁全绿 + push 成功
- [ ] PL014.4 用户总目视验收移交 —— 向用户提交验收清单（真窗口 DWM 玻璃可读性双主题 / 常驻功耗体感 / 剪贴板真链路（捕获+复制回）/ 全交互走查 / 迁移后旧数据完好）；验收问题走 FIX003；通过后 AGENTS 状态头终稿 + 记忆同步；验证：用户明确验收结论回执（不达标不勾结本条）
