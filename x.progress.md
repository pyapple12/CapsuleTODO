# 进度追踪（x.progress.md）

> 文件职责：任务清单与进度追踪，与 `z.plan.md`（方案与审计归档）配套：方案在 z.plan 展开，执行拆条在本文件勾选。结构：**`## 已完成 ✅` 区在前、`## 未完成` 区在后**，任务完成后整组移动到已完成区；**两个区内部均按从上到下 = 从旧到新排序，新组一律追加在区末尾**；已完成区历史组全量保留、只增不删（系列定案：收口不得滚出历史组，违规滚出须自 git 历史恢复）。
> 格式速查：任务组 `### PL{NNN}: {标题} [来源引用]`（来源引用：[plan#Phase N] 计划书 / [problems#N] 问题备忘录 / [audit#A{NNN}] 审计报告），组内用 `#### {小节}` 分层；子任务 `- [ ] PL{NNN}.{序号} {标题} —— {做法}；验证：{检验方式}`；审计修复任务组 FIX{NNN} 由 audit-report 归档环节生成，条目格式 `- [ ] FIX{NNN}.{序号} [P{级别}] {标题} —— {做法}；验证：{检验方式}`，编号规则见 `.agents/skills/audit-report`。
> 勾选注记：条目完成时改 `[x]`，并在句尾追加 `（YYYY-MM-DD 已验证：{一句话结论}）`——结论如实，不达标不降级宣布。

## 已完成 ✅

（暂无——立项初期无收口任务组）

## 未完成

### PL001: 玻璃壳与最小清单闭环 [plan#一期]

> 范围：工程骨架 + 玻璃壳（焦点联动材质沿系列配方）+ Todo 状态机 TDD + 内存态清单勾选闭环；方案见 z.plan.md 附录 PL001。
> 红线：业务纯逻辑零 tauri 依赖（禁 import tauri）；玻璃观感用户目验通过才算闭环；依赖版本锚定沿 Pulse 实证（typescript ^5.9.3 等，防 TS7×vue-tsc 不兼容）；临时文件一律落 .temp/。

#### 阶段 A：工程骨架

- [ ] PL001.1 工具链确认与 Tauri 2 工程骨架 —— 工具链沿机器实证（cargo 1.96.1 / Node 26.7.0 在位，Pulse 已验）；手写骨架沿系列目录风格：`core/Cargo.toml`（name capsule-todo、version 0.1.0 单一来源、edition 2021；deps：tauri { version = "2" }、serde { version = "1.0.229", features = ["derive"] }、thiserror "2.0.20"；build-deps：tauri-build { version = "2" }）+ `core/tauri.conf.json`（productName CapsuleTODO、identifier com.pyapple12.capsule-todo、窗口 title CapsuleTODO / width 300 / height 400 / transparent true / decorations false / **resizable false**、build.frontendDist ../dist + beforeDevCommand npm run build、app.version 省略、bundle.active true + icons/icon.ico）+ `core/capabilities/default.json`（permissions：core:default + core:window:allow-start-dragging）+ `core/src/main.rs`（薄入口调 capsule_todo::run()）+ `core/src/lib.rs`（run() 空壳 + `//!` 模块注释）+ 占位图标（`.temp/gen-icon.mjs` 程序生成，沿 Pulse PL001.2）+ `ui/`（package.json：name capsule-todo、type module、scripts dev/build/preview/tauri 沿 Pulse、deps @tauri-apps/api ^2.11.1 + vue ^3.5.42、devDeps @tauri-apps/cli ^2.11.4 + @vitejs/plugin-vue ^6.0.8 + prettier ^3.9.6 + typescript ^5.9.3 + vite ^8.2.2 + vue-tsc ^3.3.11；`ui/index.html` + `ui/main.ts` + `ui/App.vue` 占位；vite.config.ts 与 tsconfig.json 逐字对照 Pulse 同名文件移植）+ configs/ 与 data/ 目录占位（.gitkeep）；npm install；验证：`npm run build` 绿 + `cargo check` 绿 + debug exe 启动探测 MainWindowTitle=[CapsuleTODO] 5 秒存活后受控关闭（沿 Pulse 等价验证法）
- [ ] PL001.2 门禁四件套接入与首跑 —— cargo fmt / clippy --all-targets -- -D warnings / check + npx prettier --write + vue-tsc 全部接入（prettier 配置 .prettierrc 已就位，node_modules 已随 PL001.1 就位）；验证：首跑全绿并记录耗时基线（注记回写本条）

#### 阶段 B：玻璃壳

- [ ] PL001.3 窗口玻璃与焦点联动材质 —— `core/src/lib.rs`：run() 装配；Windows 分支局部闭包 `set_backdrop(hwnd, kind: u32)`（封装 DwmSetWindowAttribute(hwnd, 38, &kind, 4) 与 HRESULT 非零严格抛错，extern "system" dwmapi 直连零新依赖，沿 Pulse PL011 同款）；`.on_window_event` Focused 分支：true → set_backdrop(3 DWMSBT_TRANSIENTWINDOW)、false → set_backdrop(1 DWMSBT_NONE)（**非 0，0=AUTO**）+ emit "window-focus" 事件（失败落 eprintln 不中断主流程）+ **容错白名单登记第①项**（AGENTS 错误策略：材质切换失败维持前态，场景/降级/理由三要素，沿 Pulse FIX003.7）；验证：cargo clippy/check/test 绿 + AGENTS 白名单三要素核对 + live 目验——平时透明透桌面、聚焦瞬间真磨砂、失焦即刻回透明、反复切换无灰板残留（300×400 小窗复核）
- [ ] PL001.4 玻璃卡片与深浅色跟随 —— `ui/App.vue`：全窗单层玻璃（margin 0、height 100vh、radius 8px、::before 分态纱——透明态 30% 浅白 rgba(255,255,255,0.3)/纯黑 rgba(0,0,0,0.3)、.focused 磨砂态 0% 纱，沿 Pulse PL011 用户拍板值）+ 亮边描边/rim/text-shadow 令牌对照 Pulse `ui/App.vue` 实态移植（配方权威 = Pulse z.plan 附录 PL010）+ prefers-color-scheme 双主题；监听 window-focus 挂 .focused class + isFocused() 启动兜底（沿 PL011 管线）；验证：vue-tsc + npm run build 绿 + live 目验双主题玻璃观感与可读性
- [ ] PL001.5 全窗拖动 —— `ui/App.vue` 全局 mousedown：`e.target.closest(DRAG_INTERACTIVE)` 命中即忽略（常量白名单：button/input/textarea/清单行交互件，随组件演进维护），其余 `getCurrentWindow().startDragging()`（@tauri-apps/api/window；capabilities 已含 allow-start-dragging）；验证：live——空白板面按住拖动流畅、清单行/复选框不误触拖动、无文字误选

#### 阶段 C：Todo 状态机（TDD：先 FAIL 后 PASS）

- [ ] PL001.6 状态骨架与错误类型 —— 新建 `core/src/todo.rs`（`//!` 模块注释声明纯逻辑禁 import tauri）：`TodoItem { id: i64, text: String, done: bool }`（derive Serialize/Clone/Debug）+ `TodoList { items: Vec<TodoItem>, next_id: i64 }`（next_id 从 1 起）+ `TodoError`（thiserror：EmptyText/TooLong/NotFound）+ 常量 `MAX_TEXT_LEN: usize = 100`（注释注明可调）；验证：cargo clippy/check 绿
- [ ] PL001.7 四操作 TDD —— 先写测试确认 FAIL 再实现：`add(text) -> Result<TodoItem, TodoError>`（trim 后非空 + 长度 ≤100 校验，追加 tail 并返回带 id 条目）/`toggle(id) -> Result<TodoItem, TodoError>`（翻转 done，不存在 Err(NotFound)）/`remove(id) -> Result<(), TodoError>`/`sorted_view() -> Vec<TodoItem>`（新建 Vec：未完成在前按 id 升序、已完成在后按 id 升序，不动内部顺序）；用例：正常增、空文本/纯空白拒、超长拒、恰 100 过界、toggle 往返断言翻转值、toggle/remove 不存在 id 均 Err、交错增入后排序视图断言分区与序；验证：红灯（编译错 E0425 级）→ 实现转绿，全部用例零真实时间/外部依赖
- [ ] PL001.8 命令层 —— 新建 `core/src/commands/mod.rs`：`AppContext { list: Mutex<TodoList> }`（.manage 注册）+ 泛型 `poison()` 锁助手（LockResult\<T\> → Result\<T, CommandError\>，沿 Pulse FIX001.9）+ `CommandError`（thiserror：Todo(#[from] TodoError)/Poisoned，derive Serialize 跨 IPC）；新建 `core/src/commands/todo.rs`：todo_add(text)/todo_toggle(id)/todo_remove(id)/todo_list() 四命令——核心逻辑抽接收 &AppContext 的自由函数（脱离 tauri::State 直测，沿 Pulse PL001.11 法）；`lib.rs` run() 构造预置示例 TodoList（3–4 条含已完成，目验用；PL002 接真实数据后删除）注入 manage；验证：命令核心函数 cargo test 全绿（增/删/勾/非法文本/不存在 id/排序视图）

#### 阶段 D：UI 接线与收口

- [ ] PL001.9 清单 UI 与勾选闭环 —— 新建 `ui/types.ts`（`interface TodoItem { id: number; text: string; done: boolean }`，注释注明 Rust serde 为契约单一来源）+ 新建 `ui/components/TodoList.vue`：清单行（自绘复选框 + 文本，完成态删除线 + 透明度变淡）+ 分区展示（进行中区 / 已完成区，数据源 = todo_list 排序视图，前端按 done 分组渲染，不排序只分组）；勾选 invoke todo_toggle 成功后重拉 todo_list（无轮询——无计时需求）；App.vue 装配 TodoList；验证：vue-tsc + npm run build 绿 + live 人工闭环（勾选→删除线并沉入已完成区、取消勾选→回进行中区、预置数据渲染正常、玻璃观感用户确认）
- [ ] PL001.10 PL001 收口 —— 门禁四件套全绿（fmt --check / clippy -D warnings / cargo test / vue-tsc / npm run build / prettier）；结论回写 z.plan 附录 PL001 状态行；README/AGENTS 状态行回改（PL001 完成）；勾结；验证：门禁全绿 + 文档一致性核对（README 徽章与 Cargo.toml 0.1.0 一致、版本 V0.1.0.2）+ commit 草案（feat: V0.1.0.2）交用户

### PL002: 清单持久化与内容管理 [plan#一期]

> 范围：SQLite 存储落 data/（双落址）+ 操作即落库（单一事实源 = db）+ 添加/删除 UI + 已完成折叠区与空态；方案见 z.plan.md 附录 PL002。
> 红线：参数化 SQL 禁拼接；测试禁触真实用户数据（内存库/临时路径）；首启无 db = 正常态（建表自动），库损坏/SQL 错误严格报错启动失败（不进白名单）。

#### 阶段 A：存储层（TDD，内存 db）

- [ ] PL002.1 依赖接入与探针 —— `core/Cargo.toml` 增 rusqlite { version = "0.40.2", features = ["bundled"] }（锚定 Pulse 实证版本）；新建 `core/tests/storage_probe.rs` bundled 工具链冒烟（open_in_memory + 建表查询 smoke，沿 Pulse 资产模式）；验证：cargo test 编译绿 + smoke 过
- [ ] PL002.2 storage.rs TDD —— 新建 `core/src/storage.rs`（`//!` 模块注释）：`StorageError`（thiserror：Sqlite(#[from] rusqlite::Error)/Io(std::io::Error)/NotFound）+ `Storage` struct：`open(path: &Path)`（父目录 create_dir_all + Connection::open）/`open_in_memory()` + `init()` 建表（`CREATE TABLE IF NOT EXISTS todos(id INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT NOT NULL, done INTEGER NOT NULL DEFAULT 0)`，幂等；**不做 created_at**——排序按 id 即创建序，纯逻辑不依赖真实时间，KISS）+ `add(text) -> Result<TodoItem>`（?1/?2 参数化绑定，last_insert_rowid 回填 id）/`set_done(id, done) -> Result<()>`（零行更新 → Err(NotFound)，零静默）/`remove(id) -> Result<()>`（零行 → Err(NotFound)）/`list() -> Result<Vec<TodoItem>>`（`ORDER BY done ASC, id ASC`）；用例：内存库建表幂等（重复 init）、add→list 往返、set_done 往返断言值、remove 后 list 不含、不存在 id 三操作均 Err、交错增入多行断言分区与序；验证：先 FAIL 后 PASS 全绿，SQL 全参数化走查
- [ ] PL002.3 运行时落址 —— 新建 `core/src/paths.rs`（`//!` 模块注释）：`runtime_root()`（cfg dev = CARGO_MANIFEST_DIR 项目根 / release = current_exe 父目录）+ `data_dir()`（root.join("data") + create_dir_all，失败上抛）+ `db_path()`（data_dir().join("todo.db")）；用例：临时目录注入路径解析（禁触真实 data/）；验证：cargo test 绿 + live dev 启动后项目根 `data/todo.db` 出现

#### 阶段 B：接线（单一事实源 = db）

- [ ] PL002.4 AppContext 扩展与启动装载 —— `core/src/commands/mod.rs` AppContext 改 `AppContext { storage: Mutex<Storage> }`（锁序 todo → storage 单向禁反向）；`core/src/lib.rs` run() 启动：paths::db_path() → Storage::open + init（失败 eprintln + exit(1)，沿 Pulse PL002.11 法）→ 空库即为空态（**预置示例条目删除**，空态由前端兜底）；验证：cargo test 绿（命令核心改测内存库）+ live 正常启动装载 + 人为给坏路径启动退出码非零
- [ ] PL002.5 命令落库与纯逻辑收敛 —— `core/src/commands/todo.rs` 四命令改走 storage（add → 先 todo.rs `validate_text(text) -> Result<(), TodoError>` 校验再 storage.add；toggle → storage.set_done；remove → storage.remove；list → storage.list）；`core/src/todo.rs` 收敛为 TodoItem + TodoError + validate_text（TodoList/sorted_view 退役删除——排序移交 SQL，单一事实源 = db，禁留双源）；测试同步改内存库往返；验证：cargo test 全绿 + grep TodoList 零残留 + validate_text 拒绝用例仍在
- [ ] PL002.6 添加/删除/折叠 UI —— 新建 `ui/components/AddBar.vue`：顶部输入行（placeholder "添加一条待办…"、回车与按钮双触发、trim 后空文本禁用）+ 错误行（invoke 失败红色文案可见、成功后清空输入，沿 Pulse FIX001.5 反馈模式）；`ui/components/TodoList.vue` 增：条目悬停 × 删除钮（invoke todo_remove，直接删不确认——一期定案）+ 已完成折叠区（表头 "已完成 N" 点击收/展，默认收起，N 前端计数，展开区条目保留删除线）+ 空态文案（无任何条目时 "暂无待办，添加一条吧"）；App.vue 装配 AddBar + TodoList；验证：vue-tsc + npm run build 绿 + live（添加即现、空文本禁用、删除即消、勾选沉入折叠区且计数对、重启数据在、空库空态文案、AddBar 失败可见——可临时传非法值触发）

#### 阶段 C：收口

- [ ] PL002.7 PL002 收口 —— 门禁全量（fmt --check / clippy -D warnings / cargo test / vue-tsc / npm run build / prettier）；结论回写 z.plan 附录 PL002 状态行；README/AGENTS 状态行（PL002 完成）；勾结；验证：门禁全绿 + 重启持久性复验（增删勾→重启→全在）+ 文档一致性（版本 V0.1.0.3）+ commit 草案（feat: V0.1.0.3）交用户

### PL003: 桌面固定与一期收口 [plan#一期]

> 范围：置顶 + 全屏让位 + 位置记忆 + 单实例 + 观感细调 + 一期全量收口；方案见 z.plan.md 附录 PL003。
> 红线：零新依赖（extern user32 直连沿系列 extern 模式；tauri-plugin-single-instance 除外——沿 Pulse 先例）；全屏让位必须 live 实测（视频全屏场景），不得推演宣布；验证禁改系统设置（AGENTS 陷阱）。

#### 阶段 A：桌面固定

- [ ] PL003.1 置顶 —— `core/tauri.conf.json` 窗口配置增 `"alwaysOnTop": true`；验证：live——启动后板子浮于普通应用窗口之上
- [ ] PL003.2 全屏让位判定纯函数 TDD —— 新建 `core/src/fullscreen.rs`（`//!` 模块注释）：`is_covering(win: Rect, monitor: Rect) -> bool`（Rect { left, top, right, bottom } i64 四元组；win 覆盖 monitor 面积 ≥95% 判全屏，阈值常量 `FULLSCREEN_COVER_PCT` 注释注明依据）+ `should_yield(foreground_is_fullscreen) -> bool` 直通语义留扩展位；用例：恰好全屏 true、差 1px false（95% 边界两侧）、半屏 false、超出屏幕 true；验证：先 FAIL 后 PASS，零 Win32 依赖纯函数
- [ ] PL003.3 全屏让位接线 —— `core/src/fullscreen.rs` 增 `spawn_fullscreen_watcher(app: AppHandle)`：std::thread + 1s sleep 轮询；extern "system" user32 直连 GetForegroundWindow/GetWindowRect/MonitorFromWindow/GetMonitorInfoW（零新依赖，沿 dwmapi extern 模式；失败 eprintln 维持前态 + **容错白名单登记第②项**：轮询失败维持当前置顶态，三要素）；板子所在显示器取主窗 monitor；前台窗覆盖该显示器 → AppHandle.run_on_main_thread 摘 topmost（set_always_on_top(false)），否则挂回 true（**状态变化才调用**，避免每秒重设）；`lib.rs` setup 末尾 spawn；验证：cargo test 绿（纯函数部分）+ live——普通使用置顶不丢、浏览器 F11 视频全屏时板子被盖住、退出全屏板子回置顶、反复进出无闪烁；无边框全屏游戏若可测则补测，不可测如实登记"仅视频全屏实测"
- [ ] PL003.4 位置记忆 —— 新建 `core/src/settings.rs`（沿 Pulse JSON 原子写模式：同目录 .tmp 写入 + rename 替换，失败清理临时文件）：`WindowSettings { x: i32, y: i32 }`（serde default）+ `load(path)/save(path)` + `SettingsError`（Json/Io）；`core/src/paths.rs` 增 `configs_dir()`（root.join("configs") + 自建）+ `settings_path()` = configs/config.json；`lib.rs` 启动 load——**文件不存在 → 默认主屏右下距边 40px（默认值常量注释可调），NotFound 回默认登记 AGENTS 容错白名单第③项（三要素）；JSON 损坏等其余错误严格报错退出不在此列** + 越界兜底（x/y 不在任一显示器范围 → 回默认，显示器枚举经 tauri available_monitors）；窗口 Destroyed/ExitRequested 时按 outer_position save（拖动中不写盘）；验证：cargo test 绿（往返一致/损坏严格/临时路径）+ live——拖到别处重启位置保留、删 config.json 首启回默认右下、把 config.json 改成非法值启动报错退出
- [ ] PL003.5 单实例 —— `core/Cargo.toml` 增 tauri-plugin-single-instance = "2"（沿 Pulse PL004.5 同款）；`lib.rs` builder 首位注册插件：二次启动回调唤起主窗（show + set_focus）；验证：live 双开——第二实例自退、板子被唤起聚焦、tasklist 仅一进程

#### 阶段 B：收口

- [ ] PL003.6 观感细调与全形态复核 —— 300×400 小窗密度/字号/间距目验微调（App.vue/TodoList.vue 令牌级调整，零结构重排）；深浅双主题全形态复核（清单行/折叠区/AddBar/空态/错误行/聚焦磨砂与透明态切换）；验证：live 目验清单逐项过（跟随系统当前主题，禁改系统设置，沿 AGENTS 陷阱约束）
- [ ] PL003.7 PL003 与一期收口 —— 全量门禁 + 一期验收清单逐项过（①玻璃小板常驻桌面置顶 ②全屏让位 ③勾选→折叠+删除线 ④添加/删除 ⑤重启持久 ⑥位置记忆 ⑦双开防 ⑧深浅色跟随）；README 徽章与状态行、AGENTS 状态行、计划书 §6 一期勾选回写；勾结；验证：门禁全绿 + 验收清单逐项 + 文档一致性（版本 V0.1.0.4）+ commit 草案（feat: V0.1.0.4）交用户
- [ ] PL003.8 首轮全量审计 A001 —— 用户触发 audit-project skill 全量审计 → audit-report 归档（编号经用户确认 → z.plan 附录 A001 + FIX001 任务组写入本文件）；验证：审计报告四节完整（含豁免清单"无豁免也写无"）+ FIX 条目做法/验证可执行
