# CapsuleTODO — Agent Guide

玻璃质感的桌面 Todo 看板：固定在桌面、以玻璃为基底，只呈现 Todo 清单供用户勾选的极简互动小程序；二期规划临时剪贴板（玻璃板上以气泡提示 + 白板，满 5 个气泡提醒清理），三期规划 AI 规范 Todo 与临时内容（均仅记录待细化）。总体规划见 `CapsuleTODO_plan.md`。

**当前状态**：**V0.2.2.2（2026-10-09）——PL024 图片气泡（future-plan-001 功能二后功能一首落）**：两条捕获入口图优先分叉（热键前置图检测——剪贴板有图直接捕获不合成 Ctrl+C，行为变更用户定案）+ 占位文案 `🖼 截图 {yyMMddHHmmss}`（GetLocalTime 零依赖本地时区）+ 载荷分层（BubbleItem 只加 kind，图走 bubble_get_image 按需 base64）+ 复制回三格式写入（PNG 注册格式 + CF_DIB + CF_DIBV5 = 截图工具原厂写入集，alpha 全链无损，用户定案"一期做全"）+ image crate 引入（全项目首个新依赖，只开 png feature）+ clipboard_image.rs 新模块（编解码纯函数 TDD + Win32 FFI 薄壳）+ bubbles 表 kind/image 两列幂等迁移；164 测试绿 + IAB 三断言过（捕获出图/开板渲染/双击不炸）；版本 V0.2.2.2（三段 0.2.2 不变，R 递增）。上一里程碑 **V0.2.2.1（2026-10-09）——PL023 清单笔记红点指示（future-plan-001 功能二首落）**：详情本体 PL010 已在，本 PL 纯指示——TodoItem 派生字段 has_note 全链（storage row_to_item 单点 trim 判空派生 + serde 契约断言 + types/mock 四点镜像）+ t-text 内联 6px 红点（--danger 同族色，零新事件冒泡行单击开详情板，归档板不做[用户定案]）+ IAB 三断言验证（红点全亮/点红点开板/清空笔记红点灭）；版本 0.2.1→0.2.2 minor。上一里程碑 **V0.2.1.4（2026-10-05）——configs/data 整目录忽略与 .gitkeep 骨架退场**（远端两目录随推送消失，本地 dev 运行时数据原样）。上一里程碑 **V0.2.1.3（2026-10-04）——v0.2.1 发布收口**（README 双语下载链接改指 v0.2.1 + tag v0.2.1 推送 + release/README 脚注随包更新）。上一里程碑 **V0.2.1.2（2026-10-04）——FIX013 第 11 轮审计修复全组闭环**：首启默认前端连带收口（置顶/吸附三处默认对齐 V0.2.0.0 定案关）+ 主题三态持久化全链（theme 字段/越界钳制/读写命令/广播载荷/启动回读/mock 镜像）+ 跨条目编辑竞态双侧修 + 清单双击罩死两查 + 两套件显式 destroy + 托盘 FFI 单源 + README 版本口径定案与目录树纠偏 + 白名单三条登记扩围（广播 emit 失败含 bubble 同构/PL015.5 并发接管/P12 标题双击）；**A/FIX 同号绑定规范首用（A013↔FIX013）**。上一里程碑 **V0.2.1.1（2026-10-04）——PATCH001 归档板 7 天自动回收 + 行内编辑点击截胡修复**：已完成条目 done_at 超 7 天自动删除（零 schema 改动，归档读路径惰性清扫[调用点=App 挂载+每次清单变更]，固定 7 天无设置项，NULL 永不回收，退回重勾自动重计时）；**PATCH{NNN} 小修小改编号轨道建立**（任务清单纪律节）。上一里程碑 **V0.2.0.1（2026-10-04）——CI 云端门禁上线与发布回填报账**：持续集成工作流入库（push main + PR 自动跑 fmt/clippy/test/doc/前端构建，windows-latest 全覆盖 cfg(windows) 测试 + rust-cache；README 挂 CI 动态徽章）；V0.2.0.0 发布回填（FIX012 闭环详情 + 后续排期定案：**macOS/Linux 适配提为下一件大事**、三期 AI 搁置仅记录、README 截图等使用积累）；方案文档已完成/待完成两区全量刷新（断档清账）。上一里程碑 **V0.2.0.0（2026-10-03/04）= 首个正式版发布上线**（0.1.9→0.2.0 minor：正式图标替换 + 绿色单 exe 出厂 + 首启默认四项[置顶关·吸附关·主题跟随·右上落位] + 开板末帧 pop 根治[四变体隔离实验定案 will-change] + 双击标题缩回托盘；commit 49f88a3 + tag v0.2.0 + GitHub Release 绿色 zip）。V0.1.9.1 = FIX011 观察项清理批（CSP 落地 + §四豁免定案清单建立[永久 39/条件 7] 唯一事实源）。V0.1.8.9 = FIX010 收官（A005–A010 六轮审计收官实现面零新缺陷；新增"修改后查缺补漏"四问硬性工作流）。版本线：V0.1.1.5 玻璃基座 → V0.1.1.6 横切工厂 → V0.1.1.7 清单与数据迁移 → V0.1.1.8 归档板 → V0.1.1.9 气泡页与剪贴板 → V0.1.1.10 复刻缺漏修复与设置板前置 → V0.1.1.11 拖拽排序与持久化 → V0.1.2.1 三段推进（0.1.1→0.1.2）+ 真窗口修复批 → V0.1.2.2 白板/设置持久化收口（PL014）→ V0.1.2.3 第 3 轮审计修复收口（FIX003 全组 + 总目验缺陷五批）→ V0.1.3.1 玻璃聚焦联动定案（SYSTEMBACKDROP 系统背板 + 粒子回位）→ V0.1.3.2 新增置顶与浮板交互修复 → V0.1.4.0 全局气泡热键与重复捕获去重（PL015）→ V0.1.4.1 第 4 轮审计 P2 修复（FIX004.1–4.7）→ V0.1.4.2 第 4 轮审计 P3 修复收口（FIX004.8–4.23）→ V0.1.5.0 热键圈选直达与失焦实时刷新（PL016）→ V0.1.6.0 窗口置顶开关与贴边吸附（PL017）→ V0.1.7.0 托盘图标与悬浮预览（PL018）→ V0.1.7.1 托盘交互四修与退出入口（PL019）→ V0.1.7.2 预览延迟出现与显隐状态机重构（PL020）→ V0.1.8.0 托盘菜单自绘化（PL021）→ V0.1.8.1 设置板偏好同步与菜单文案（PL022）→ V0.1.8.2 第5轮审计修复批一（FIX005.1–23,25）→ V0.1.8.3 三组件 composable 收敛与空态共存修复（FIX005.24）→ V0.1.8.4 第5轮审计收官（FIX005.26–31 全组闭环）→ V0.1.8.5 第6轮审计修复全组闭环（FIX006.1–20）→ V0.1.8.6 第7轮审计修复全组闭环（FIX007.1–18）→ **V0.1.8.7 第8轮审计修复全组闭环（FIX008.1–16）** → **V0.1.8.8 第9轮审计修复全组闭环（FIX009.1–12）** → **V0.1.8.9 第10轮收官审计修复全组闭环（FIX010.1–14，A005–A010 六轮收官）** → **V0.1.9.1 观察项清理批与豁免定案（FIX011.1–11，0.1.8→0.1.9 minor：CSP 落地 + A001–A010 观察项清账 + 豁免定案清单建立）** → **V0.2.0.0 打包发布里程碑（正式图标 + 绿色单 exe，0.1.9→0.2.0，tag v0.2.0 发布上线）** → **V0.2.0.1 CI 云端门禁上线与发布回填报账** → **V0.2.1.1 归档板 7 天自动回收（PATCH001 首条，0.2.0→0.2.1 minor；同批捎带行内编辑输入框点击截胡修复）** → **V0.2.1.2 第 11 轮审计修复全组闭环（FIX013 全组 12 条；A/FIX 同号绑定规范首用 A013）** → **V0.2.1.3 v0.2.1 发布收口（README 双语链接 + tag v0.2.1）** → **V0.2.1.4 configs/data 整目录忽略与 .gitkeep 退场（远端目录随推送消失）** → **V0.2.2.1 清单笔记红点指示（PL023，0.2.1→0.2.2 minor：has_note 裁决全链 + t-text 内联红点 + IAB 三断言）** → **V0.2.2.2 图片气泡（PL024，0.2.2 R 递增：热键前置图检测 + 复制回三格式 + image crate 首引 + clipboard_image 新模块）**。**APP 回归工作模式（用户定案 2026-09-26）**：不开分支直干 main；每 PL 自验三道闸（Rust TDD 全绿 / IAB 冒烟行为断言 / 截图比对 design 形态忠实）+ 一条 feat 提交推送；**中间零人工目验，PL014 后用户一次总目视验收**，问题走 FIX003。实验场现状（V0.1.1.4 并入内容）：`design/` 独立原型三页签全交互（纯 HTML/CSS/JS 零构建；assets/css|js 共 22 文件，引用顺序 = 级联/执行顺序不得乱序），清单删除钮二态盖翻确认、罩死行禁交互灰化、长按拖拽重挂模型，气泡单击开全文板/双击复制、拖拽与罩死同款推广，归档成品化与边缘三角随板缩放生长，玻璃滑杆与整板阅读工厂五容器统一；uiverse 组件五处引入（R001 规范，均用户目验定值）；配色基准 `design/color_spark.md`。三期 AI 规范待立项讨论（仅记录见计划书 §6）。沿系列基线：Tauri 2 + 纯 Rust 业务 + Vue 展示 + 玻璃材质（2026-09-29 用户定案：聚焦联动系统背板——聚焦 DWM SYSTEMBACKDROP 亚克力、失焦纯透明 + 前端 30% 分态纱；WinAppSDK/OS-XAML 组合栈与 WebView2 同线程互斥已实证封禁）；macOS/Linux 适配延后 [problems#1]。`.agents/skills/` 存放项目自建 skill（audit-project / audit-report / progress-task）。遗留与远期项登记 `y.problems.md`。

## 技术栈

| 组件     | 选型                                                                                                                                                                                                                                                      |
| -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 框架     | Tauri 2（Rust 后端 + 系统 WebView）                                                                                                                                                                                                                       |
| 前端 UI  | Vue 3 + TypeScript + Vite（只做展示，业务零含量）                                                                                                                                                                                                         |
| 核心逻辑 | 纯 Rust（状态机/持久化/业务规则，cargo test 直测）                                                                                                                                                                                                        |
| 存储     | rusqlite（SQLite，`data/` 双落址沿系列基线，一期定案后回改）                                                                                                                                                                                              |
| 玻璃效果 | 聚焦联动系统背板（2026-09-29 定案）：聚焦 DWMWA_SYSTEMBACKDROP 亚克力（深色基调声明 + 激活边框隐藏）、失焦 DWMSBT_NONE 纯透明 + 前端 30% 分态纱；组合栈路线（XAML Islands / WinAppSDK）与 WebView2 同线程互斥已实证封禁；macOS vibrancy / Linux blur 延后 |
| 通知常驻 | 一期定案：无系统通知常驻（板子常驻桌面即所见）；系统通知随后续需求另议                                                                                                                                                                                    |

## 启动命令（规划）

```bash
npm run tauri dev      # 开发运行（透明玻璃窗口；Tauri CLI 走 npm devDep；tauri 目录沿系列名 core/，CLI 按 tauri.conf.json 探测）
cargo test             # Rust 层全量测试（不依赖 UI）
npm run dev            # 前端热更开发
npm run build          # 前端构建校验（含 vue-tsc；产物 dist/ 内嵌进 exe，改前端后先 build 再 cargo build）
```

依赖：Rust toolchain、Node.js（26+）、Tauri CLI；平台依赖——系统 WebView、玻璃效果（Linux 需 compositor，见计划书 §2.4）。

## 架构要点

- **业务逻辑全在 Rust 侧**（项目学习目标，也是架构基线，沿系列）：Todo 状态机、持久化、业务规则全部放 core/src，前端只做展示与命令转发——禁止把业务逻辑写进 Vue 组件
- **玻璃效果**（2026-09-29 用户定案，覆盖 2026-09-28 旧定案）：**聚焦联动系统背板**——聚焦挂 `DWMWA_SYSTEMBACKDROP_TYPE = DWMSBT_TRANSIENTWINDOW`（系统亚克力，实时模糊背后桌面），失焦切 `DWMSBT_NONE` 回纯 alpha 透明；窗口一次性整定深色模式声明（背板基调对齐主题）+ 边框隐藏（`DWMWA_BORDER_COLOR = NONE`，激活描边不显形）；前端 30% 分态纱保留（聚焦纱退 0%，`window-focus` 事件驱动）；实现落 `core/src/glass_backdrop.rs`（DWM 属性级接入，不初始化组合引擎）。**组合栈路线实证封禁**：OS XAML Islands 与 WinAppSDK DesktopAcrylicController 的组合引擎初始化均与 WebView2 同线程互斥（渲染期 stowed exception 崩溃，先后顺序无关，二分实证元凶=组合引擎本体）——勿再尝试；AccentPolicy 老管道仍可用但明度锁死（观感劣于系统背板，已弃）。系统背板零参数（模糊强度/颜色不可调，模糊强度偏大属系统行为）。编译期 `#[cfg(target_os)]` 分支互不影响；macOS/Linux 适配延后至 Windows 版成熟后 [problems#1]
- **运行时数据**：沿系列双落址基线——config.json 落 `configs/`、数据库落 `data/`（dev=项目根、release=exe 同级）；备份 = 直接拷 configs/ + data/；一期方案定案后回改
- **常驻形态与 Todo 数据模型**（固定桌面方式、勾选交互、schema）待一期讨论定案后在此补充

## 目录规划

（实态，随版本演进更新——目录树与代码失实即纠偏，FIX013.7 纪律）

```
core/             # Tauri 2 后端（框架文件须与 Cargo.toml 同住）
  Cargo.toml      # 版本单一来源（version 三段式 X.Y.Z）
  tauri.conf.json # version 字段省略（回落 Cargo.toml）
  capabilities/   # ACL 权限（core:default + 拖动白名单）
  icons/          # 应用图标（正式五件套 + source.png 设计源，FIX012.1 替换一期占位）
  tests/          # storage_probe.rs（rusqlite bundled 冒烟探针）+ sort_order_migration.rs（PL013 五列/排序迁移集成测试，真实文件库）
  src/
    lib.rs        # 应用装配：桌面固定（置顶/全屏让位/位置记忆/单实例）+ 设置加载（含越界钳制）+ 命令注册
    main.rs       # 薄入口（调 capsule_todo::run()）
    todo.rs       # 业务纯逻辑：TodoItem DTO + 文本校验（禁 import tauri）
    bubble.rs     # 业务纯逻辑：气泡 DTO/快照 + 捕获文本校验（满额显隐裁决在前端，FIX004.23）
    whiteboard.rs # 业务纯逻辑：白板内容长度校验
    storage.rs    # SQLite Repository（todos/bubbles/whiteboard 表，参数化 SQL）
    paths.rs      # 运行时数据双落址解析（data/ 与 configs/，目录自建）
    settings.rs   # 运行时设置持久化（窗口位置 + 气泡提醒上限 max_bubbles + 气泡热键 + 置顶 always_on_top + 吸附 snap_to_edge + 主题三态 theme，JSON 原子写；上限/热键/主题越界钳制收敛此单点）
    fullscreen.rs # 全屏让位（逐边包含判定纯函数 + user32 轮询线程；置顶关闭时休眠）
    glass_backdrop.rs # 玻璃背板（V0.1.3.1：SYSTEMBACKDROP 聚焦联动 + 深色声明/边框隐藏，DWM 属性级 FFI）
    hotkey.rs     # 全局气泡热键（PL015：组合键解析纯逻辑 TDD + RegisterHotKey 注册线程，Win32 直连零依赖）
    capture.rs    # 圈选捕获（PL016：剪贴板等待纯状态机 + 合成 Ctrl+C 两段式 + 前台终端守卫；PL024 双通道演进 wait_clipboard_capture）
    clipboard_image.rs # 剪贴板图片读写（PL024：DIB/PNG 编解码纯函数 + base64 手写 + Win32 多格式读写 FFI 薄壳，image crate 编解码）
    snap.rs       # 贴边吸附纯函数（PL017：四边 ±阈值过冲窗口判定，RectI32 入参 TDD）
    tray.rs       # 托盘（PL018-021：预览显隐四态状态机 + 单一守候线程 + 自绘菜单窗锚定/点外守候 + 菜单勾选落库）
    commands/     # Tauri 命令层（mod.rs 上下文与错误封装 / todo.rs / bubble.rs / whiteboard.rs / settings.rs / tray_preview.rs / tray_menu.rs）
ui/               # Vue 前端（App.vue 三页签骨架 + src/components/ 十二组件 + src/composables/ 组合式 + src/styles/ 样式层 + src/dev/ 冒烟基座；types.ts 镜像 IPC DTO）
configs/          # 程序读的固定参数与用户参数（config.json 窗口位置，运行时写入，gitignore）
data/             # 运行时数据（todo.db，gitignore，运行时自建）
.agents/skills/   # 项目自建 skill（audit-project / audit-report / progress-task）
.temp/            # 临时脚本与文件（gitignore）；探针、验证记录放这里
y.problems.md     # 问题与远期改进备忘录（只增不删、编号递增；任务清单以 [problems#N] 引用）
```

## 任务清单纪律（x.progress.md，系列 2026-09-10 定案）

- **条目做法必须写到文件/函数级**：`—— 做法` 部分要指明改哪个文件、哪个函数/模块、新增什么结构/命令/表、怎么改（细到可照做）；禁止一句话概括后只留验证方式（CapsulePulse PL005 初版条目因做法缩略被退回重写，此为系列先例）
- **验证方式必须可执行、可断言**：写明用例名/命令/live 步骤；TDD 任务先写红灯用例再实现
- **编号轨道三线**（2026-10-04 定案）：`PL{NNN}` 功能里程碑 / `FIX{N}` 审计修复（audit-report 生成）/ `PATCH{NNN}` 小修小改——用户日常提出的小功能与小调整（不源自审计、不走审计轮），编号自 PATCH001 递增；方案入 z.plan.md 附录 PATCH{NNN}、执行条目入 x.progress.md `### PATCH{NNN}` 组，做法/验证标准与 PL 同规
- **A/FIX 编号绑定**（2026-10-04 定案）：审计报告附录 `A{NNN}` 与其修复任务组 `FIX{NNN}` **恒同号**（A013↔FIX013）；取号 = 下一可用 FIX 号（max+1，A 侧不独立取号），轮次叙述（"第 N 轮"）与编号解耦——FIX 号被非审计批占用时审计顺延取号防撞号（首例：第 11 轮审计 = A013/FIX013，因 FIX011/FIX012 已被观察项清理批与打包发布批占用）
- **已完成组全量保留、只增不删**；两个区内部从上到下 = 从旧到新，新组追加在区末尾（详见 x.progress.md 头部规则）
- 立项方案（z.plan 附录）的实现措施与任务条目同标准：一律拆到文件/函数级

## 自动格式化与静态检查（硬性工作流，替代 LSP）

> ZCode 无自动 LSP / Formatter 挂载机制，本节即强制工作流：**改完必须跑，不通过不得宣称完成**；环境缺工具时先报告用户，不得静默跳过。

- **编辑任何 `.rs` 文件后**：依次运行 `cargo fmt`、`cargo clippy --all-targets -- -D warnings`、`cargo check`，三条全过才算完成
- **编辑前端文件（`.ts` `.tsx` `.vue` `.css`）后**：运行 `npx prettier --write <file>`；若改动含逻辑（非纯样式），还需 `npx vue-tsc --noEmit` 通过
- **编辑 `.md` 后**：运行 `npx prettier --write <file>`（本文件与计划书等均适用）
- **Rust 文档注释与签名变更后**：`cargo doc --no-deps` 确认无 broken intra-doc link（可与其他检查合并跑）
- **提交前全量门禁**：`cargo fmt --check` + `cargo clippy -D warnings` + `cargo test` + `npm run build`

## 修改后查缺补漏（硬性工作流，A010 实证）

> 与上方门禁并列的强制复查动作：门禁兜编译与格式，本节兜**自查结构性盲区**。依据 = A007–A010 四轮审计连续抓出上轮修复未竟面（每轮三组）的实证——机械性遗漏（笔误/编译器可查/明显同构写点）自查可兜，以下四类是自查盲区，修改后必须主动执行对应动作；修复域内同构穷举（"还有哪些同款"）始终是第一动作，本节补的是穷举方向之外的面。

- **"校验 + 延迟执行"模式**（定时器回调/异步 then/事件回调/拍子/哨兵）：任何"先校验后动作"的修复，必须追问**"校验通过到实际执行之间，世界会变吗"**——条目可删、条目可切、状态可翻转；校验在入口 ≠ 行为时刻有效（判例：FIX009.4 堵全入口漏拍子到点复核；FIX008.7 同类先修却未类推）。
- **注释/文档类修复**：同表述**全仓 grep**（跨文件常多处同款）——只改条目框定处必留漏网（判例：FIX009.9 改两处，A010 抓出 tray.rs×2 + lib.rs×1 三处"唯一调用者"漏改）。
- **新增变体/结构/常量/字段**：回查项目**登记面先例**逐个补位——CommandError 新变体须补序列化契约测试断言（A004 Hotkey / A006 Window 先例）；新字段须核 settings 模块头、types.ts 镜像、mock 镜像；新容错降级须登记容错白名单（判例：FIX009.8 漏契约断言；settings 模块头停留 PL015）。
- **数据流上游修改**：改 emit 载荷/返回值/命令行为时，**追到全部下游消费端核完再收口**——跨组件回落、模板绑定、监听器都在列（判例：FIX009.4 未追 App.vue `?? item` 回落复活已删条目）。

> 边界认知（自查不可替代审计）：修复者复查可消除"改动域内"的遗漏，而独立时序推演、全库陌生化扫描、跨轮次惯例对照是审计轮的不可复制视角——宣称"按审计标准复查过"时不得说满，收敛判据以审计报告为准。

## 工程原则（设计哲学，所有项目通用）

> 设计哲学总纲（18 条 / 5 大类，与用户级 instructions.md 同源）；下文"代码规范"为项目细则，两者冲突时以本项目边界为准。

### 核心思想

- 以第一性原理思考问题：理解需求背后的真实目标，而非直接套用已有模式或技术方案。
- 优先解决本质问题，避免为假设中的未来需求提前设计复杂系统。
- 在保证长期可维护性的前提下，选择当前最简单、可靠、清晰的实现方案。

### 简洁与设计

- 遵循 KISS：优先选择简单直接的实现，避免不必要的复杂度。
- 遵循 DRY：避免重复逻辑，但不要为了消除少量重复而创建过度抽象。
- 遵循 SOLID 思想：职责清晰、降低模块耦合，提高可维护性和扩展能力。

### 架构

- 不长期保留废弃方案：优先删除过时代码，而不是增加兼容层、fallback 或临时迁移逻辑。
- 不进行未经验证的架构设计：避免提前引入抽象、配置和间接层。
- 从最小可工作的版本开始逐步演进，每次修改建立在已有可运行系统之上。
- 永远不要用未来可能需要的复杂性，牺牲当前产品的可用性。

### 代码质量

- 保持模块职责明确，避免一个模块承担过多职责。
- 优先使用成熟、稳定、维护良好的第三方库，而不是重复造轮子。
- 使用项目已有依赖解决问题之前，不要随意新增依赖。
- 在引入新方案前，先检查已有代码、依赖、文档和能力。
- 避免为了"看起来更优雅"而增加实际复杂度。

### 工程决策

- 优先选择长期可维护的方案，而不是只能临时运行的解决方案。
- 代码应该服务于业务目标，而不是为了展示技术复杂度。
- 如果简单方案已经满足需求，不要主动升级为复杂方案。

## 函数注释规则（Rust / TypeScript）

- **Rust**：每个函数定义上方紧跟 `///` 文档注释（中文），说明用途和核心逻辑（1-3 行）；公开 API 的参数、返回值、panic 条件用 `/// # 参数` / `/// # 返回` / `/// # Panics` 小节补充；**禁止无文档注释的 `pub` 函数**
- **Rust 模块**：每个 `.rs` 文件顶部用 `//!` 模块注释说明文件职责、设计理由（为什么这样做）、关联的配置或外部依赖
- **TypeScript / Vue**：导出函数与组合式函数用 `/** */` 中文注释说明用途和核心逻辑（1-3 行）；复杂内部函数同样注释
- **禁止以注释替代类型**：能用类型签名表达的约束（`Option` / `Result` / 联合类型）不写进注释
- 单行 `//` 注释用于解释"为什么"，不复述代码本身做什么

## 代码约定（Rust + Vue/TS）

- **注释语言**：所有注释与文档注释使用中文；注释符号：Rust 用 `//` `///` `//!`，TS/Vue 用 `//` `/** */`，禁止其他语言注释符号
- **命名风格**：Rust 由编译器强制——函数/变量 `snake_case`、类型 `CamelCase`、常量/静态 `UPPER_CASE`；TS/Vue 变量函数 `camelCase`、类型/组件 `PascalCase`、常量 `UPPER_CASE`
- **私有性**：Rust 模块内部使用收敛可见性（默认私有，跨模块用 `pub(crate)`，慎用 `pub`）；TS 模块内部用 `_` 前缀表示内部私用，外部模块不应直接调用
- **入口**：可执行 crate 有 `fn main()`；Tauri 命令参数与返回值走 serde 类型定义
- **类型**：Rust 强类型天然覆盖；TS 禁止 `any`，优先显式标注，接口用 `interface`/`type`，跨进程数据结构用 serde + TS 类型定义单一来源
- **数据结构**：配置聚合优先用 Rust struct + `serde::Deserialize`（对应设置持久化 / 通知参数 / 窗口配置），不做散装 HashMap 取值
- **use / import 顺序**：Rust——标准库 → 第三方 crate → 本地 crate/模块，组间空行（rustfmt 默认分组）；TS——外部包 → 内部模块，组间空行
- **字符串格式化**：Rust 优先 `format!("{var}")` 内联变量写法；TS 优先模板字符串，避免 `+` 拼接
- **集合构建**：Rust 优先 iterator 适配器链（`map`/`filter`/`collect`）而非手写 for 循环构建集合
- **布尔判断**：用 `if x` / `if !x`（TS 用 `if (x)` / `if (!x)`），而非与字面量显式比较
- **空值判断**：Rust 用 `Option::is_none()` / `is_some()` 及 `match`/`if let`；TS 用 `x == null` / `x != null` 同时覆盖 null 与 undefined，禁止与 `undefined` 单独比较
- **错误处理**：Rust 禁止在业务代码散落 `unwrap()`/`expect()`——核心逻辑用 `thiserror` 定义错误类型，应用顶层用 `anyhow`；错误必须向上传播或明确记录，禁止空 `catch`/`let _ =` 吞错；TS 避免 `catch {}`，至少记录日志
- **行长度**：每行尽量不超过 100 字符（rustfmt `max_width = 100` 默认值；超过时在运算符或逗号后换行）
- **格式化自动化**：空格、缩进、引号、尾逗号等排版细节**以 rustfmt 与 prettier 输出为准**，不手工对齐；普通字符串双引号（TS 由 prettier 配置保证）
- **路径处理**：Rust 强制使用 `std::path::Path` / `PathBuf`，禁止手拼字符串路径；运行时数据目录经配置解析，禁止硬编码机器路径；TS 使用 `path.join` / `URL`，禁止字符串拼接路径
- **临时文件**：所有临时生成的脚本/文件必须写入项目根目录下的 `.temp/` 文件夹（gitignore），禁止散落在源码目录
- **文件修改必须用 edit 工具**：修改既有文件（.rs/.ts/.vue/.md/.json/.toml）一律用 edit 的精确 oldString/newString 替换，禁止用 python -c 或 PowerShell 脚本做内容替换（易踩引号/缩进坑）；新建 .temp 探针脚本不受限
- **版本双轨**：机器版本只存 `core/Cargo.toml` 的 `version`（三段式 X.Y.Z，Cargo 强制 semver，本项目单 crate 非 workspace）；对外正式文本（commit 标题、README 徽章、发布说明）一律写四段式 `VX.Y.Z.R`——前三段与 Cargo.toml 一致，第四段 R 为修订号，只随提交历史记录、不入任何配置文件（递增规则见"Commit 提交规范"）；**README 徽章与下载链接的口径分工（FIX013.6 定案，2026-10-04）**：徽章 = 代码版本，随 Cargo.toml bump 即时推进（发布前短暂超前属正常态）；下载链接 = 实际发布产物，仅在 GitHub Release 发布后更新（提前改 = 指向不存在文件 404）；release/README.md 版本脚注 = 发布产物组成部分，随打包时点更新（当前 V0.2.0.0 = 现行发布件，属准确态）；`tauri.conf.json` 省略 version 字段（Tauri 回落读 Cargo.toml），`package.json` 的 version 为 npm 生态必填字段、不参与发布——豁免于单一来源，不派生不注入（对齐系列项目 FIX001.25 豁免先例）
- **Shell 命令**：执行命令前先检测当前 shell（`echo $SHELL` 或检查系统），本机 Windows 环境；避免使用 Linux-only 或 PowerShell-only 工具，优先跨 shell 兼容命令
- **PowerShell 中文编码**：bash 工具的 pwsh 会话带 `-NoProfile`，不加载 `$PROFILE`，输出中文前必须先设置编码：`[Console]::OutputEncoding = [System.Text.Encoding]::UTF8;`
- **Git 操作**：未经用户明确要求，不得擅自执行 `git add`、`git commit` 或任何其他 Git 写操作

## Commit 提交规范

- **标题行**：`<type>: V<版本>，<摘要>`——版本为四段式 `VX.Y.Z.R`（如 `V0.1.0.1`）：前三段与 `core/Cargo.toml` 的 `version` 一致，第四段 R 为修订号（不入配置文件，见下条）；摘要一句话概括核心
- **版本递增**：`feat`/`fix`/`refactor`/`perf` 提交前先 bump 第四段 R（R+1）；前三段在 `Cargo.toml` 中推进时 R 回到 1；`docs`/`test`/`style`/`chore` 不强制
- **type 全集**（conventional 风格）：`feat` 新功能 / `fix` 修复 / `refactor` 重构（行为不变）/ `perf` 性能 / `docs` 文档 / `test` 测试 / `style` 格式 / `build` 构建依赖 / `ci` CI / `chore` 杂项 / `revert` 回滚
- **正文**（默认必写，仅 docs/style 微调可省）：`- ` 列表逐条"具体做了什么"，每条一个功能块、一行自然中文、≤ 100 字符；体例＝`主题：内容（细节；细节）`，同主题多细节用 `；` 分隔。样例：`- 计时状态机：Idle/Running/Paused 三态 + start/pause/resume/total 四操作（暂停继承累计、时间源可注入）`
- **禁止项**：内部编号（PL001.1/FIX001.2 等任务编号）、验证/回归数字（如"全量回归 506 项通过"）、英文混排描述
- **提交范围**：一个版本的所有连带改动一次提交（源码 + 配置 + 文档同步）
- **GitHub 身份与远程**：远程 `git@takechance:pyapple12/CapsuleTODO.git`（SSH 别名 `takechance` = pyapple12 账号，密钥 id_b_takechance，经 socks5 127.0.0.1:10808）；仓库级身份 `git config user.name pyapple12` + `user.email takechance_bao@188.com`（覆盖全局 YiYi 身份，与系列同款做法）
- **流程**：每次提交前 AI 必须 `git status` + `git diff`（含 `--stat`）核对改动范围无误，再草拟完整 commit 内容（add 清单 + 标题 + 正文）交用户确认；用户审阅后自行执行 `git add`/`git commit`/`git push`（AI 不执行任何 git 写操作）

## 分支处理（UI 大改，沿 CapsulePulse 系列）

> **APP 回归期例外（用户定案 2026-09-26）**：PL008–PL014 直干 main 不开分支——每 PL 自验三道闸（Rust TDD / IAB 冒烟断言 / 截图比对 design 忠实）+ 一条 feat 提交推送；中间零人工目验，PL014 后用户总目视验收（问题走 FIX003）。走错单条 revert 回退，main 线性历史保持。

- **作用与应用**：长周期 UI 迭代（玻璃配方落回 ui/、实验场续调）在专属分支 `ui-1.0-feature` 进行，主线 main 期间不收中间态、保持可用；分支上每完成一个组件落点请用户浏览器目验一次
- **版本控制**：分支提交用单序列号 `ui1.0 V0.001` 起递增，标题 `<type>: ui1.0 V<序号>，<摘要>`；不占主线 V0.1.x.R 轨道，`core/Cargo.toml` 三段式分支期间冻结
- **合并办法**：用户目验 + 全量门禁通过后 `git merge --squash ui-1.0-feature` 回 main——单条提交进主线轨道（版本推进与 R 归一合并时定），正文按体例写全连带文档；**合并后分支保留归档**（用户定案 2026-09-26，覆盖旧删分支规则）：打标签 `ui1.0-final` 钉住收官点并推送，分支本地与远程原样保留、后续实验迭代沿分支继续

## 错误策略

主线：**严格抛错**——配置缺失/非法输入直接返回错误（Rust 返回 `Result` 并定义明确错误类型；TS 抛出具体 Error），不静默兜底、不降级假成功。

容错白名单（初始为空；仅产品明确允许的边界可容错，**新增容错须先在此登记**，写明场景、降级行为与理由三要素）：

- **焦点联动材质切换失败维持前态**（PL001.3 登记，2026-09-17）：场景——窗口焦点切换时 DWM 背板设置失败（DwmSetWindowAttribute 非零）或 window-focus 事件发送失败；降级行为——错误落控制台日志，材质/纱态维持切换前状态；理由——材质为纯装饰层，运行时焦点事件不可中断主流程，下次焦点切换自动重试自愈（沿 CapsulePulse FIX003.7 同款先例）。**〔复活在用〕2026-09-29 玻璃定案回归焦点联动（SYSTEMBACKDROP 系统背板版），背板切换失败落日志维持前态即本条行为；2026-09-28 曾随 DWM 材质移除标记失效，登记沿革留存**
- **全屏监视轮询失败维持置顶态**（PL003.3 登记，2026-09-17；FIX010.4 扩围）：场景——全屏让位后台线程的 Win32 轮询（GetWindowRect/GetMonitorInfoW）失败或前台窗句柄为空，及设置锁失败读不到 always_on_top；降级行为——错误落日志（失败态翻转时只报一次防刷屏），跳过本轮让位/恢复判定，置顶态维持不变，下轮轮询自动重试；理由——让位为体验增强层，轮询/锁失败不该抖动常驻行为，更不可假定置顶开后继续行动（恢复臂会把用户已关的置顶违意打开），恢复正常后自动收敛。
- **窗口设置文件不存在回默认位**（PL003.4 登记，2026-09-17）：场景——首启或用户删除 configs/config.json，载入 NotFound；降级行为——返回默认落位（主屏右上距边 40px，V0.2.0.0 由右下改右上）不报错；理由——首启无设置文件是正常态而非错误，回默认即"开箱即用"；JSON 损坏等其余错误仍严格报错不在此列。
- **窗口位置保存失败不阻断关闭**（PL003.4 登记，2026-09-17）：场景——关闭窗口时读取/保存 config.json 失败；降级行为——错误落日志，关闭照常进行；理由——退出意图优先，位置丢失代价小（可再拖一次），不能因保存失败把用户困在应用里（沿 Pulse"退出前落库失败仍退出"先例）。
- **设置文件越界 max_bubbles 静默钳制**（FIX003.8 登记，2026-09-28；FIX008.5 措辞收窄；**FIX013.2 扩围 theme 字段**，2026-10-04）：场景——config.json 为用户可手改的明文，max_bubbles 字段为**可解析为 u32 的越界整数**（<1 或 >20），及 theme 字段为**可解析为 u8 的越界整数**（>2）；负数/浮点/字符串等 serde 不可解析形态不属本条，归严格报错主线（JSON 解析失败启动退出非零）。降级行为——加载点经共享钳制函数（settings::clamp_max_bubbles / settings::clamp_theme）静默收敛到 1/20 边界与 0（跟随系统），不报错不崩；理由——配置非法不该崩常驻应用，钳制与设置板步进及命令层写路径同规（读写两路径单一来源）。
- **关闭时设置锁中毒跳过保存**（FIX003.8 登记，2026-09-28）：场景——CloseRequested 时 AppContext.settings 锁中毒（持锁线程 panic 后遗症）；降级行为——跳过位置与上限保存落日志（锁中毒 Debug 串），关闭照常进行，磁盘现值不动；理由——以默认值透传保存会静默覆盖用户已存 max_bubbles（数据回退），跳过的代价（位置回默认位）小于覆盖真实设置。
- **config.json 气泡热键非法静默回默认**（PL015.2 登记，2026-09-30）：场景——用户手改 config.json 的 bubble_hotkey 字段为解析器不认的组合；降级行为——加载点 hotkey::parse 失败静默回默认 Ctrl+Alt+C 落日志，不报错不崩；理由——配置非法不该崩常驻应用，与 max_bubbles 越界钳制同规（读写路径同源规范化）。
- **气泡热键解析/注册失败不阻断启动**（PL015.5 登记，2026-09-30；FIX013.11 扩围并发接管变体，A013 补登记 2026-10-04）：场景——启动时热键 parse 失败或 RegisterHotKey 失败（组合被其它程序占用）；降级行为——落日志继续启动，快捷键不可用但其余功能不受影响；设置板改热键重注册失败则回滚旧热键落库并红字提示（可见回退）；**并发变体（FIX010.6 行为面）**——回滚前复核当前落库值，被后发轮接管（代际失配致本线线程自灭）则放弃回滚落库（保守不动作，磁盘现值不动），锁再失败同样保守放弃；理由——热键是增强入口，缺失不伤主流程，占用属环境冲突应显形于设置板而非崩溃，放弃回滚防旧值覆盖后发轮已落库新值。
- **关窗时白板 flush 失败仍关闭**（FIX004.19 登记，2026-09-30）：场景——关窗请求触发前端白板强制落库（T 腿 onCloseRequested → flush）时 whiteboard_save 失败（库锁/IO），或 webview 卡死致 T 腿事件不可达；降级行为——错误落日志（flush 内部置错误行），窗口照常销毁（≤800ms 防抖尾巴丢失），Rust R3 腿 600ms 超时强关独立兜底（T+R3 双腿，探针实测 .temp/close-probe/）；理由——退出意图优先，沿"位置保存失败不阻断关闭"（PL003.4）先例，尾巴丢失代价小于把用户困在关不上的应用里。
- **守候线程 WATCH 锁中毒恢复取值**（FIX005.14 登记，2026-10-01；FIX009.11 扩围）：场景——预览显隐守候线程或托盘事件处理持 WATCH 锁 panic（锁中毒后遗症），及热键线程对 ON_HOTKEY 回调锁的写入（set_on_hotkey）与读取（消息循环回调分发）遇锁中毒；降级行为——`into_inner` 恢复取值继续跑（内部数据 panic 前已完整写入，不丢弃；ON_HOTKEY 回调仅 spawn 期写入一次，毒化前后值等价），状态机下一拍自愈；理由——显隐守候与热键分发是常驻增强层，锁中毒不该挂死托盘交互与热键响应，沿"焦点联动材质切换失败维持前态"（PL001.3）同款恢复式先例。
- **托盘菜单读设置锁失败按默认关**（FIX005.14 登记，2026-10-01；V0.2.0.0 缺省改关后回退值与措辞同步——原"按默认开取 true"）：场景——tray.rs snap_current/top_current 读 AppContext.settings 锁失败（中毒）；降级行为——静默取 false（缺省关）继续勾选翻转，下次事实源读取纠正；理由——托盘构建缺省语义即 serde default（V0.2.0.0 起为关），错读只影响一次取反方向且自愈，与"设置文件越界 max_bubbles 静默钳制"（FIX003.8）同规。
- **吸附动作读设置锁失败跳过吸附**（FIX006.14 登记，2026-10-02）：场景——lib.rs snap_if_needed 读 AppContext.settings 锁失败（中毒）判 snap_to_edge；降级行为——回 false 跳过本轮吸附落日志，下轮 Moved 重试自愈；理由——与上方 snap_current 回 false 方向一致（V0.2.0.0 缺省改关后两侧同为保守不动作），此处 false 只影响"是否吸附"这一动作，snap_current 的 false 只影响勾选框显示（与缺省一致），两侧均为自愈面且互不污染对方语义。
- **偏好广播读设置锁失败跳过广播**（FIX008.12 登记，2026-10-03）：场景——settings_set_* 落库后 emit_prefs_changed 读 PrefsSnapshot 锁失败（中毒）；降级行为——落日志跳过本次广播，设置板/托盘菜单勾选态滞后一次刷新，下次任意设置操作重新广播自愈；理由——广播是同步增强层，锁中毒不该阻断设置主流程（沿"托盘菜单读设置锁失败按缺省"FIX005.14/"吸附动作读设置锁失败跳过吸附"FIX006.14 同族先例）。
- **清单/气泡变更广播 emit 失败跳过广播**（FIX013.10 登记，2026-10-04；bubble-changed 为同构穷举扩围，PL016.1 既有"热键反馈静默"定案一并入册）：场景——命令层 emit_todo_changed（commands/todo.rs 变更命令）与 bubble-changed 发送（commands/bubble.rs 捕获收尾）的 tauri emit 失败；降级行为——错误落控制台日志，本次跨窗刷新缺失，发起窗自身刷新链路（组件时序/300ms 主拍）不受影响，下次任意变更重新广播自愈；理由——广播是同步增强层，emit 失败不该打断用户操作主流程（沿"偏好广播读设置锁失败跳过广播"FIX008.12 同族先例）。

## 素材与环境陷阱

- **运行时数据不入仓库**：config.json（configs/）与数据库/日志（data/）是用户运行时数据——dev 落项目根、release 落 exe 同级，均已 gitignore（双落址，沿系列定案）；测试用临时 db 一律落 `.temp/` 或系统临时目录，禁止对真实用户数据做测试写入
- **玻璃效果平台差异**：macOS 真 vibrancy / Windows Acrylic / Linux 依赖 compositor（计划书 §2.4/§8 风险项）；当前仅 Windows 实机可验——改玻璃相关代码须注明"Windows 实机验证 + macOS/Linux 按分支逻辑推演"，不提前宣布跨平台可用
- **深浅色主题**：跟随系统；硬编码纯色会破坏主题感知，玻璃卡片用半透明 + 系统模糊适配两种主题
- **Rust 学习曲线**（计划书 §8）：状态机/存储先写纯逻辑 + 单测、小步迭代；时间相关逻辑用注入时间源测试，禁止 sleep 等真实等待；Mutex/生命周期报错及时求助不硬扛（系列 CapsulePulse 已实战一轮，经验可复用）
- **Tauri 2 ACL 静默拒**：前端能力调用（事件 listen、`data-tauri-drag-region` 拖动、invoke 等）走 ACL 白名单，未授权时**静默失效、控制台零报错**（系列案例：CapsuleRetro listen 需 `core:event`（2026-09-06）、CapsulePulse 拖动需 `core:window:allow-start-dragging`（2026-09-08），`core:default` 均不含）；权限事实源 = 构建产物 `core/gen/schemas/acl-manifests.json`；新增前端能力调用后必须 live 验证功能生效，构建绿不代表功能可用
- **运行中 exe 锁文件**：调试运行 capsule-todo.exe 期间 cargo build 无法覆盖产物（os error 5）——重建前先停运行实例（系列实测，2026-09-08）
- **验证禁止更改电脑系统设置**：live 验证一律以系统当前态进行；禁止为验证去切换主题、动画效果（reduced-motion）等系统设置——替代手段 = 代码走查确认退避/令牌规则在位 + 用户日常使用复核（系列定案，2026-09-13）
- **dev 无 HMR**：`beforeDevCommand = npm run build`、`frontendDist = dist`（静态产物）——改前端后必须重新 build + 重启 dev 流程才生效，热更不存在（系列实测，2026-09-13 CapsulePulse）
- **tauri emit_filter 对 JS 监听无效**：tauri 2.x `match_any_or_filter` 语义（listener.rs:306）= JS `listen()` 缺省 target Any **恒通过无视 filter**——想"广播排除某窗"用 emit_filter 是无效的（FIX005.5 踩坑、A006 P2-1 实证），正确方案 = 载荷携带发起窗 label、前端自判早退（FIX006.1 落地形态）
- **Vue 生命周期钩子仅 setup 同步上下文生效**：onMounted/onUnmounted 等在异步回调或嵌套函数内注册会被静默忽略（生产无警告）——FIX005.26 曾把 onUnmounted 嵌套进 onMounted 回调 = 防护恒失效（A006 P2-4 实证）；需要在"挂载后"响应卸载时，把标志声明在 setup 层、置位放顶层 onUnmounted
- **Windows 杀软误报**：开发期单文件 exe 可能被误报，调试用绿色版形态
- **PowerShell 中文编码**：见"代码约定"同名额
