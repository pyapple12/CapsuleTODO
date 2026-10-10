# 项目方案与审计归档（z.plan.md）

> 文件职责：方案文档与审计归档。与 `x.progress.md`（任务清单）、`CapsuleTODO_plan.md`（总体规划）分工：总体规划不动，方案演进与审计记录都落在本文件。
> 结构：一、已完成 ✅ → 二、待完成 → 三、主题规划（按需）→ 四、审计观察项豁免定案清单 → 附录 PL{NNN}（专题方案，立项时创建）→ 附录 PATCH{NNN}（小修小改方案，立项时创建）→ 附录 A{NNN}（审计报告，由 audit-report 归档环节生成）。

## 一、已完成 ✅

（从新到旧；版本号 = 提交四段式，详见 AGENTS.md 版本线）

- **A013 第 11 轮全量审计归档**（2026-10-04，**A/FIX 同号绑定规范首用**——A 跟 FIX 走，FIX011/FIX012 已被非审计批占用故取 013；P2×3+P3×8 无 P0/P1，实现面零新缺陷；修复走 FIX013）→ 附录 A013
- **PATCH001 归档板 7 天自动回收**（2026-10-04 闭环 5/5，V0.2.1.1：**零 schema 改动**[done_at PL010 已有] + 归档读路径惰性清扫[启动+每次清单变更自动触发] + 固定 7 天无设置项；**PATCH{NNN} 小修小改编号轨道同批建立**；同批捎带行内编辑输入框点击截胡修复）→ 附录 PATCH001
- **FIX012 打包发布里程碑批**（2026-10-04 闭环 11/11，**V0.2.0.0 首个正式版发布上线**：commit 49f88a3 + tag v0.2.0 + GitHub Release；正式图标/绿色单 exe 出厂/首启默认四项[置顶关·吸附关·主题跟随·右上落位]/开板末帧 pop 根治/双击标题缩回托盘/release 使用说明与 MIT）→ 附录 FIX012
- **FIX011 观察项清理批**（2026-10-03 闭环 11/11，V0.1.9.1：CSP 落地 + 九条轻松修；**§四豁免定案清单建立[永久 39/条件 7]，A001–A010 报告观察项节归并为指向**）→ 附录 FIX011
- **FIX010 第 10 轮收官审计修复**（2026-10-03 闭环 14/14，V0.1.8.9：拍子到点复核防幽灵板/归档按钮守卫前置/注释例外补齐/白名单②扩围/契约断言/回滚写复核/哨兵单源/测试通道门控/expose 清理；**A005–A010 六轮审计收官，实现面零新缺陷**）→ 附录 A010
- **FIX009 第 9 轮审计修复**（2026-10-03 闭环 12/12，V0.1.8.8：热键放弃标志换代际计数[双热键幽灵根除]/详情板错误归因随 id 四处+残留清空/勾选末拍收口/watchEffect 依赖收集先于早退/幽灵板四入口/bubble-changed 三跑收敛/Bubble 变体归位）→ 附录 A009
- **FIX008 第 8 轮审计修复**（2026-10-03 闭环 16/16，V0.1.8.7：**上轮修复引入的死锁违规根除**[复核块拆两步]/Suppressed 期窗可见一律 Hide/保存基准携带条目 id/哨兵先行覆盖在飞窗口/trim 口径对齐/行映射收敛/windows_subsystem）→ 附录 A008
- **FIX007 第 7 轮审计修复**（2026-10-02 闭环 18/18，V0.1.8.6：哨兵字段级/beforeBuildCommand/hotkey 死等待/in-flight 守卫/lastSaved 基准/单源四连/Show 竞态复核）→ 附录 A007
- **FIX006 第 6 轮审计修复**（2026-10-02 闭环 20/20，V0.1.8.5：origin 载荷方案等——本轮核心 = 抓出 FIX005 三项修复无效，确立"grep 在位 ≠ 语义生效"方法论）→ 附录 A006
- **FIX005 第 5 轮审计修复**（2026-10-01 闭环 31/31，V0.1.8.2–.4 三批：emit 修正/listen 防线/三组件 composable 收敛等）→ 附录 A005
- **PL022 设置板偏好同步与菜单文案**（2026-10-01，V0.1.8.1）→ 附录 PL022
- **PL021 托盘菜单自绘化**（2026-10-01，V0.1.8.0：HTML 菜单窗替代原生菜单 + 幽灵图标退出链修复）→ 附录 PL021
- **PL020 预览延迟出现与显隐状态机重构**（2026-10-01，V0.1.7.2：单一守候线程四态状态机；死锁铁律两轮实证）→ 附录 PL020
- **PL019 托盘交互四修与退出入口**（2026-10-01，V0.1.7.1）→ 附录 PL019
- **PL018 托盘图标与悬浮预览**（2026-10-01，V0.1.7.0）→ 附录 PL018
- **PL017 置顶开关与贴边吸附**（2026-09-30，V0.1.6.0）→ 附录 PL017
- **PL016 热键捕获两改进**（2026-09-30，V0.1.5.0：失焦实时刷新 + 圈选直达）→ 附录 PL016
- **FIX004 第 4 轮审计修复**（2026-09-30 闭环，V0.1.4.1–.2 两批）→ 附录 A004
- **PL015 全局气泡热键**（2026-09-30，V0.1.4.0：Win32 RegisterHotKey 直连 + 重复捕获去重）→ 附录 PL015
- **FIX003 第 3 轮审计修复 + 总目验缺陷五批**（2026-09-28，V0.1.2.3：APP 回归总验收后的修复收口）→ 附录 A003
- **PL008–PL014 APP 回归七组**（2026-09-26 起直干 main，V0.1.1.5 → V0.1.2.2：玻璃基座/横切工厂/清单页/归档板/气泡页/拖拽排序/白板设置收口——用户总目视验收过，问题走 FIX003）→ 附录 PL008–PL014
- **PL007 界面实验场并入**（2026-09-26，V0.1.1.4：ui-1.0-feature 分支 squash 合并，标签 ui1.0-final 钉存，分支保留归档）→ 附录 PL007
- **A003–A010 第 3–10 轮全量审计归档**（修复对应上列 FIX003–FIX010，不另立条）
- **PL006 二期收口 + FIX002**（2026-09-17，V0.1.1.1：二期功能全量提交）→ 附录 PL006 / A002
- **A002 第 2 轮全量代码审计**（2026-09-17 归档；修复走 FIX002）→ 附录 A002
- **PL005 白板**（2026-09-17 代码落地勾结；live 移交统一目验；提交随二期收口统一执行）→ 附录 PL005
- **PL004 页签导航与气泡**（2026-09-17 代码落地勾结；live 移交统一目验；提交随二期收口统一执行）→ 附录 PL004
- **FIX001 第 1 轮审计修复**（2026-09-17 收口，live 经用户确认；提交随一期收口统一执行）→ 附录 A001
- **PL003 桌面固定与一期收口**（2026-09-17 收口，用户目验全过；提交随一期收口统一执行）→ 附录 PL003
- **A001 第 1 轮全量代码审计**（2026-09-17 归档，首轮；修复走 FIX001）→ 附录 A001
- **PL002 清单持久化与内容管理**（2026-09-17 收口，用户目验全过；提交随一期收口统一执行）→ 附录 PL002
- **PL001 玻璃壳与最小清单闭环**（2026-09-17 收口，用户目验全过；提交随一期收口统一执行）→ 附录 PL001

## 二、待完成

**下一件大事：macOS/Linux 适配**（2026-10-04 用户定案——取代原三期位置提前；三期 AI 暂缓仅记录见计划书 §6；适配方案待立项讨论：玻璃材质分平台分支已有 cfg 骨架，托盘/热键/吸附的 Win32 直连模块需平台等价层）。

近期小项：

- **README 顶部截图 + Social preview**：等用户实际使用一段时间积累真实内容后再截（活数据界面），上传仓库 Settings → Social preview；Release 说明可 Edit 补图

历史收口备忘：APP 回归模式（PL008–PL014 直干 main + 终验收制）已完结归档；一期定案要点（置顶、全屏让位、位置记忆、任务栏不隐藏、开机自启不做 y.problems#4）与二期定案已全部落入实现，后续以豁免定案清单（§四）与 y.problems.md 为准。

## 三、主题规划

- （按需）跨附录主题出现时在此登记；远期与遗留项见 y.problems.md

## 四、审计观察项豁免定案清单

> 豁免唯一权威源：已定案项审计时（audit-project）不再重复报告。新定案条目由归档环节（audit-report）经用户确认后追加。分级：①**永久豁免**——不再报告不再讨论；②**条件豁免**——标注触发条件，条件变化时重新评估。
> **2026-10-03 定案**（FIX011 观察项清理批，用户定案）：A001–A010 十轮观察项整合去重后定案如下——永久豁免 39 条（含原条件豁免 9 条经评估条件不可达性高升格）、条件豁免 7 条、活化修复 1 条（CSP → FIX011.1）。A001–A010 各报告观察项节已同步清理为指向本清单，本节为唯一事实源。

### ① 永久豁免（39 条——不可达/设计固有/装饰层/收益低于成本/用户定案）

| #   | 观察项                                                                                                | 出处           | 永久理由                                  |
| --- | ----------------------------------------------------------------------------------------------------- | -------------- | ----------------------------------------- |
| P1  | todo_list 失败冻结旧清单无反馈                                                                        | A001           | 有意降级——冻结优于报错打断                |
| P2  | setup 期 expect（lib.rs:262/:349）                                                                    | A001/A003      | 装配期不变量断言，架构保证不可达          |
| P3  | db 路径进启动错误消息                                                                                 | A003           | 本地桌面应用助排障                        |
| P4  | 轮询 1000ms/热键常量/武装 500ms/miss 4×100ms 内联字面量                                               | A003/A004/A010 | 有注释依据或测试锚定的算法参数            |
| P5  | fullscreen 轮询线程不随窗关退出                                                                       | A003           | 进程生命周期线程                          |
| P6  | 最小化 -32000 坐标入 config                                                                           | A003           | 启动越界兜底自愈                          |
| P7  | bubble.rs 双锁 SQLite 读                                                                              | A003           | 低频 UI 读锁内耗时微小                    |
| P8  | 无虚拟化/类名 camelCase 混用                                                                          | A003           | 300×400 数据量小/uiverse 保形             |
| P9  | 录制态按旧热键触发一次捕获                                                                            | A004           | 系统 RegisterHotKey 固有时序              |
| P10 | Win10 pre-22H2 玻璃日志刷屏                                                                           | A004           | 平台基线 Win11 定案                       |
| P11 | capture 合成 keyup 清修饰键残留                                                                       | A005           | 有注释依据的定案权衡                      |
| P12 | 托盘菜单动作与标题双击缩回托盘失败仅 console.error（FIX013.9 扩围覆盖 main_hide_to_tray，2026-10-04） | A005           | 点项先收窗架构下无反馈可行面              |
| P13 | 预览窗首拉失败与真空态不可区分                                                                        | A005           | 冻结豁免同族                              |
| P14 | R3 迟到 destroy 竞态日志                                                                              | A005           | 定案行为                                  |
| P15 | mock max_bubbles 0 值漂移                                                                             | A005           | 前端钳制后不可达                          |
| P16 | 置顶反向残留（窗口已切落库失败）                                                                      | A006/A007      | 采纳方案固有，IO 错误才触发               |
| P17 | fullscreen 三类失败共享日志旗标                                                                       | A006           | 微瑕，白名单②语义内                       |
| P18 | TrayPreview listen 在 await refresh 之后                                                              | A007           | refresh 有 catch 不会 reject              |
| P19 | DRAG_THRESHOLD 5/6 同名异值                                                                           | A007           | 不同域有注释                              |
| P20 | seed_data expect+exit 并存/MOD_* pub 过宽                                                             | A007           | 工具不出货/历史面                         |
| P21 | 双 epoch 时钟并存                                                                                     | A008           | 语义隔离清晰，登记非问题                  |
| P22 | Idle 拍每 100ms 一次主线程往返                                                                        | A008           | 开销可忽略                                |
| P23 | mock Number\|\|5 漂移                                                                                 | A008           | UI 永不发 0                               |
| P24 | 四 DTO Clone derive 零调用                                                                            | A009           | 防未来惯例，零代价                        |
| P25 | AgeLevel Deserialize 占位                                                                             | A010           | 注释自证防未来对称                        |
| P26 | format! 表名插值两处                                                                                  | A010           | SQLite 设计限制，已裁决无注入面           |
| P27 | paths data_dir 备份接缝                                                                               | A010           | 九轮既有形态                              |
| P28 | fullscreen hwnd 启动期一次捕获                                                                        | A010           | hide 语义主窗永不重建                     |
| P29 | theme() 失败深色缺省                                                                                  | A010           | 一次性装饰层                              |
| P30 | system_now unwrap_or(0)                                                                               | A008           | 时钟回拨纯理论不可达                      |
| P31 | ★ settings 原子写无 fsync                                                                             | A001           | 断电+写入瞬间双条件，数据仅窗口位置       |
| P32 | ★ fullscreen 容差 8px 自动隐藏任务栏误判                                                              | A001           | 当前实机无此环境配置                      |
| P33 | ★ toggle/remove 失败无用户可见反馈                                                                    | A001/A008      | 本地 IPC 架构性极低，行保持原状即自然反馈 |
| P34 | ★ 气泡去重 COUNT+INSERT 非原子/无 UNIQUE 索引/全表扫/list 无 LIMIT                                    | A005/A008/A010 | Mutex 串行+单实例+小量级锁死不可达        |
| P35 | ★ 预览/菜单窗 -2000 首绘副屏闪现/py<0 虚拟屏顶                                                        | A005           | 特定多屏拓扑，当前环境不存在              |
| P36 | ★ 菜单 toggle 100ms 竞窗/current_monitor 失败/守候慢盘 5s 缺席                                        | A005           | 极端时序理论缺口                          |
| P37 | ★ settings 读路径合法非规范热键串不收敛                                                               | A009           | 用户手改才触发，仅回显外观面              |
| P38 | ★ EXITING 无复位                                                                                      | A009           | close 失败极端路径，语义偏转仍合用户意图  |
| P39 | ★ tray_preview 预览窗缺失静默 Ok                                                                      | A010           | 创建失败 setup 即中止，不可达             |

（P31–P39 = 原条件豁免经 2026-10-03 评估"条件不可达性高"升格永久。）

### ② 条件豁免（7 条——触发条件到达时重评）

| #   | 观察项                                                    | 出处           | 触发条件与重评指引                                    |
| --- | --------------------------------------------------------- | -------------- | ----------------------------------------------------- |
| T1  | OS 关机等非常规退出不落位置 + WM_QUERYENDSESSION 行为     | A001/A007      | 需实机关机验证；丢失代价小，产品定位升级时评估双保险  |
| T2  | 剪贴板错误文案含英文技术细节                              | A002           | 用户目验反馈文案问题时打磨                            |
| T3  | TrayMenu/TrayPreview 及 origin 链路零 IAB 覆盖            | A005/A007      | 补 mock 事件源时同步补断言（真机 live 已验）          |
| T4  | 归档板 450ms settle 快速双击中间态 + 无 isOpen 守卫       | A004/A009      | 快速双击/早关板可见问题时修（重开自愈）               |
| T5  | 代际计数残余：stall 占键窗口内回滚轮注册失败 → 无热键自愈 | A010           | 实测复现时评估；自愈收敛无双幽灵（白名单⑧语义内）     |
| T6  | 换热键失败路径主线程冻结最坏 ~2.2s                        | A004/A009/A010 | 用户反馈卡顿或改 async 命令架构时重评（同步取舍在案） |
| T7  | capture 300ms 等待窗并发复制被覆盖                        | A009           | 静默族设计定案变更时重评                              |

（活化条目 CSP 不在本清单——已列 FIX011.1 修复，打包回归 live 验证。）

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

### 二、参考级观察项 → 已定案归档（FIX011）

> 明细已整合至 §四《审计观察项豁免定案清单》（永久 39 / 条件 7 / CSP 活化 FIX011.1），本节不再保留（2026-10-03 用户定案，本清单为唯一事实源）。

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

### 二、参考级观察项 → 已定案归档（FIX011）

> 明细已整合至 §四《审计观察项豁免定案清单》（永久 39 / 条件 7 / CSP 活化 FIX011.1），本节不再保留（2026-10-03 用户定案，本清单为唯一事实源）。

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

### 二、参考级观察项 → 已定案归档（FIX011）

> 明细已整合至 §四《审计观察项豁免定案清单》（永久 39 / 条件 7 / CSP 活化 FIX011.1），本节不再保留（2026-10-03 用户定案，本清单为唯一事实源）。

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

### 二、参考级观察项 → 已定案归档（FIX011）

> 明细已整合至 §四《审计观察项豁免定案清单》（永久 39 / 条件 7 / CSP 活化 FIX011.1），本节不再保留（2026-10-03 用户定案，本清单为唯一事实源）。

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

## 附录 A005：全量代码审计报告（第5轮，2026-10-01）

> 范围：core/ 全部 .rs + Cargo.toml + tauri.conf.json + capabilities + ui/ 全部 .ts/.vue/.css（约 19200 行）。方式：三路并行只读通读（Rust 业务组 / Tauri 集成组 / 前端组）+ A004/FIX004 回归复核（主会话 grep 批量 + 子代理 git diff d6820ba..HEAD 补盲）。基线 commit d6820ba → HEAD eaa7e84。
> 状态：📌 待修复（FIX005 任务清单见 x.progress.md；观察项 12 组经用户复核全部不提升）

### 零、上轮修复复核清单（A004/FIX004.1–24）

| 上轮条目                                                                                                                                                                    | 现状             | 证据                                                                                                                                                                |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P2 七条（DetailOverlay 防抖清理 / SettingsOverlay 录制态复位 / BubblesView duplicate 复位 / useDragReorder forceRemount / mock 对齐 / TodoList 空态共存 / hotkey 线程加固） | ✅ 全部在位      | DetailOverlay.vue:145-173、SettingsOverlay.vue:166-169、BubblesView.vue:312、useDragReorder.ts:406、mock-invoke.ts:226-322、TodoList.vue:316-334、hotkey.rs:211-266 |
| P3 十五条（吞错改 eprintln / 文档失实 / 契约断言 / 死代码收敛 / 钳制收敛 / 白名单注释等）                                                                                   | ✅ 全部在位      | settings.rs:74-76、hotkey.rs:131-136、mod.rs:164-171、bubble.rs:98 cfg(test)、settings.rs:105-107 等                                                                |
| **结论**                                                                                                                                                                    | **零回退零漏改** | 后续 7 个 PL 大改动无一冲掉 A004 修复                                                                                                                               |

### 一、P0-P3 修复清单

无 P0/P1。**P2 五条（跨组同根已合并）**：

| #    | 文件:行号                                                                                                           | 类型  | 描述                                                                                                                                                                                                                                                                                                                | 建议                                                                                                             | 性质                              | 影响面                  |
| ---- | ------------------------------------------------------------------------------------------------------------------- | ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- | --------------------------------- | ----------------------- |
| P2-1 | lib.rs:458-479                                                                                                      | 1/2   | **CloseRequested 无窗口 label 过滤**：预览窗可获焦（打勾点击），Alt+F4 落在 tray-preview 上——①预览窗 outer_position 写入 config.json 主窗 x/y（位置记忆污染）②R3 腿 600ms 销毁预览窗且本 session 永不重建，hover 预览失效至重启                                                                                     | 分支首行 `window.label() != "main"` 即 return                                                                    | 新增                              | 位置记忆/预览窗生命周期 |
| P2-2 | lib.rs:511-526 + App.vue:217-219                                                                                    | 1/11  | **Focused 无 label 过滤且全局广播**：预览窗获焦时玻璃背板挂到预览窗 hwnd；主窗收到预览窗 focused=true → 纱态与真实焦点相反                                                                                                                                                                                          | Focused 分支过滤 label=="main"                                                                                   | 新增                              | 主窗玻璃纱态/预览窗视觉 |
| P2-3 | lib.rs:480-510                                                                                                      | 1/8   | **Moved 吸附状态机无 label 过滤**：托盘窗 set_position 喂进全局拖拽状态机并可触发 snap（preview reanchor 路径确定污染时间戳；菜单窗吸附路径需验证）                                                                                                                                                                 | Moved 分支过滤 label=="main"                                                                                     | 新增                              | 吸附状态机/托盘窗定位   |
| P2-4 | lib.rs:329-341 + tray_menu.rs:21-24 + lib.rs:208-215                                                                | 1     | **主窗销毁后托盘僵尸态**：Alt+F4 主窗（关窗=destroy）后进程因两隐藏窗存活——托盘左键/聚焦主窗/单实例唤起三入口全部 get main=None 静默 no-op，板子永久消失仅剩托盘。与 PL018"关闭入口=隐藏语义"定案相悖                                                                                                               | 主窗 CloseRequested 托盘存活时改 hide（对齐定案，**方向待用户拍板**）或三入口 None 时重建主窗                    | 新增                              | 托盘常驻形态核心交互    |
| P2-5 | commands/todo.rs:9-13,156-165 + App.vue:226-228 + TodoList/DetailOverlay/ArchiveOverlay 组件链 + TrayPreview.vue:65 | 1/4/9 | **todo 变更事件三重缺陷（同根合并）**：①Rust emit_todo_changed 全局广播含发起窗 → 本窗 refresh 即时替换 items → 勾选/归档 300ms 主拍被截断（TodoList.vue:102 注释预警的"勾选瞬间坍缩"复活）+ 刷新双跑（6 invoke）②前端组件链各自 emit → 双跑另一半 ③todo_reorder 唯一漏发 todo-changed → 拖拽排序后托盘预览窗不刷新 | Rust emit_filter 排除发起窗（跨窗同步保留，本窗回到 A004 前前端链时序）+ reorder 补发 + TrayPreview 冗余 emit 删 | 新增（A004 #14 修复在更高层复活） | 勾选动效/刷新链/预览窗  |

**P3 共 25 条**：

| #   | 文件:行号                                                       | 类别 | 描述与建议                                                                                                                                                                                |
| --- | --------------------------------------------------------------- | ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | settings.rs:120-130                                             | 1/13 | 置顶开关先落库后切窗口：窗口切换失败时 ctx/磁盘已翻转不回滚、不发广播——分叉至用户再拨。建议先窗口成功再落库或失败回滚                                                                     |
| 2   | storage.rs:54 + :459-486                                        | 1/11 | StorageError::NotFound 文案硬编码"待办条目不存在"，气泡路径复用 → 删/读不存在气泡报"待办"（确定性错值）。泛化文案或独立变体                                                               |
| 3   | tray_menu.rs:15,21-22 + lib.rs:329-341 等 + tray.rs:236,247,406 | 13   | `let _` 吞显隐/聚焦错误共 9 处零日志（合并条），白名单未登记。改 eprintln 或按三要素登记                                                                                                  |
| 4   | tray.rs:128-134 vs commands/settings.rs:174-189 + 两处裸 json!  | 4/11 | prefs-changed 双实现（tray.rs 两次取锁拼快照有半新半旧窗口）+ 载荷裸 json! 无 serde struct 单一来源。emit_prefs_changed 提 pub(crate) + 定义 PrefsSnapshot                                |
| 5   | lib.rs:63-68 vs tray.rs:83-88                                   | 4    | now_ms() 两文件逐字重复。收敛单点                                                                                                                                                         |
| 6   | tray.rs:431 vs lib.rs:216                                       | 3/1  | cursor_in_tray 硬编码 50×60 命中矩形（忽略 Enter 真实 rw/rh）且两处注释矛盾（50×50 vs 50×60）——DPI 大于该值时边缘 hover 失效。WatchState 补存真实宽高                                     |
| 7   | lib.rs:396                                                      | 3    | 预览窗创建高度 180.0 内联魔法数（宽高常量已单源、高缺位）。补 PREVIEW_HEIGHT                                                                                                              |
| 8   | capabilities/default.json:8                                     | 12   | core:window:allow-destroy 全前端零调用（退出/关窗全走 Rust 侧）——死权限扩大 ACL 面。删除（先经构建产物复核）                                                                              |
| 9   | tray.rs:78-80,138-151                                           | 13   | 锁中毒 into_inner 恢复 + snap/top_current 锁失败静默缺省——白名单未落账（注释自称"候选"）。补登记三要素                                                                                    |
| 10  | tray.rs:96-122                                                  | 4    | on_pref_menu 对同一 id 两处 match 分发（算值/执行分离）。合并单 match                                                                                                                     |
| 11  | commands/settings.rs:77                                         | 13   | 回滚路径 parse(&old).expect("旧热键必合法")——业务 expect，不变量依赖两处远端逻辑。改 match + 落日志                                                                                       |
| 12  | commands/tray_preview.rs:14-19                                  | 2    | tray_preview_resize(height: f64) 零校验（NaN/负/超大直透）。入口 clamp(80, 800)                                                                                                           |
| 13  | tray.rs:392,403                                                 | 6    | spawn_menu_watch 引用 crate::win_input（cfg windows 模块）函数体未 cfg 化——非 Windows 编译必失败。函数体 cfg 化                                                                           |
| 14  | TodoList.vue:145                                                | 1/5  | editEl.value?.focus() 恒 no-op 死语句（v-for 模板 ref 被收集为数组，本文件注释自记同款坑）——"点自己行重聚焦"永不生效；pendingExit 态点行二次 commit 重复 IPC。改 DOM 直查 + 去二次 commit |
| 15  | TrayPreview.vue:11-15                                           | 11   | 本地 PreviewTodo 未走 types.ts 单一来源（TodoView 镜像已有）。改 import                                                                                                                   |
| 16  | TrayPreview.vue:128-132,201-204 + App.vue:436-441               | 5    | 死样式三段（tp-title/tp-done/.placeholder 模板零引用）。删                                                                                                                                |
| 17  | mock-invoke.ts:78,243                                           | 5/10 | state.lastCopied 只写不读（注释称"供断言"实不可达）。暴露读取口或删                                                                                                                       |
| 18  | TodoList.vue:2-6                                                | 6    | import 顺序违规（外部包夹在内部模块后）。上提                                                                                                                                             |
| 19  | TodoList/BubblesView/ArchiveOverlay                             | 4    | 三组件逐字重复四套逻辑（showEmpty 420ms / DelButton 收口 / pinLeaveHeight / mountScrollKit）——FIX004.6 同根修复落了三份。收敛 composable                                                  |
| 20  | SettingsOverlay.vue:221 vs settings.rs                          | 12/3 | BUBBLE_MAX_LIMIT=20 前端与 Rust clamp(20) 双处硬编码（mock 第三处）。加命令读上限或注释钉死联动                                                                                           |
| 21  | BubblesView.vue:384-388                                         | 8/13 | void listen("bubble-changed") 无 catch + unlisten 异步赋值与卸载竞态（句柄后到 = 泄漏）。补 catch + 挂载标志                                                                              |
| 22  | storage.rs:373-456                                              | 4    | reorder_todos/reorder_bubbles 事务脚手架约 30 行逐字重复。抽私有助手                                                                                                                      |
| 23  | storage.rs:3 + commands/todo.rs:123                             | 6    | 两处注释仍写"排序按 id 升序"（实际 sort_order 优先）——文档失实                                                                                                                            |
| 24  | AGENTS.md:47-58                                                 | 6    | 目录树漂移：settings.rs 职责行缺两开关、commands 缺 tray_menu/tray_preview、缺 capture.rs/snap.rs/tray.rs 行                                                                              |
| 25  | storage.rs:38-40                                                | 5    | is_duplicate 常驻 pub 零生产调用（added_item 同族漏网）。cfg(test) 收敛                                                                                                                   |

### 二、参考级观察项 → 已定案归档（FIX011）

> 明细已整合至 §四《审计观察项豁免定案清单》（永久 39 / 条件 7 / CSP 活化 FIX011.1），本节不再保留（2026-10-03 用户定案，本清单为唯一事实源）。

### 三、亮点

### 三、亮点

A004 二十四项修复零回退（7 个 PL 大改动无一冲掉）；SQL 全参数化、时间源注入、锁序纪律无反向；三窗 DOM 严格分域无跨窗触碰；App.vue 监听句柄全部入 onUnmounted；mock↔serde 契约零漂移；types.ts ↔ BubbleCaptureOutcome tagged 契约零漂移；PL022 prefs-changed 两条 emit 链载荷键名一致。

## 附录 A006：全量代码审计报告（第6轮，2026-10-02）

> 范围：全仓通读（约 19500 行）。方式：三路并行（Rust 业务 / Tauri 集成 / 前端）+ A005/FIX005 回归复核（主会话 grep 31 项 + 子代理 git diff 级补盲 + 主会话源码级裁决 3 项关键声称：tauri-2.11.5 listener.rs match_any_or_filter 语义 / @tauri-apps/api window.cjs onCloseRequested 自动 destroy 与 ACL invoke 路径 / settings Default x/y=0 落位链）。基线 eaa7e84 → 7907cb4。
> 状态：📌 待修复（FIX006 任务清单见 x.progress.md；观察项默认全不提升）

### 零、上轮修复复核清单（A005/FIX005）

grep 级 31 项全命中；diff/语义级复核抓出 3 项"修复无效或改残"（grep 在位 ≠ 语义生效）：

| FIX005 条目                                                              | 现状                      | 裁决证据                                                                                                                                                                     |
| ------------------------------------------------------------------------ | ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| FIX005.5 "排除发起窗"（emit_filter）                                     | ❌ 对 JS 监听者无效       | tauri-2.11.5 listener.rs:306 match_any_or_filter：`Any \|\| filter`——JS listen() 缺省 target=Any 恒通过无视 filter；勾选坍缩截断原样在位                                     |
| FIX005.26 listen 竞态防护（disposed）                                    | ❌ 恒失效                 | onUnmounted 注册在 onMounted 回调内——Vue 生命周期钩子仅 setup 同步上下文生效，disposed 永为 false                                                                            |
| FIX005.1 非主窗关窗过滤                                                  | ⚠️ 改残                   | 早退分支未 prevent_close——托盘窗获焦 Alt+F4 走 runtime 默认销毁                                                                                                              |
| 其余 28 项（.4 EXITING/.13 死权限/.24 三 composable/.27 reorder 收敛等） | ✅ 在位                   | 批量 grep + 三路 diff 复核                                                                                                                                                   |
| 特别核验：hide 语义 vs T 腿自动 destroy                                  | ✅ 当前完好但靠脆弱副作用 | JS onCloseRequested handler 后自动 destroy（window.cjs:1637）→ plugin:window\|destroy invoke → ACL 静默拒绝（FIX005.13 已删 allow-destroy）——hide 语义是被权限删除意外保住的 |

### 一、P0-P3 修复清单

无 P0/P1。**P2 四条**：

| #    | 文件:行号                                                 | 类型 | 描述                                                                                                                                                                                   | 建议                                                                 | 性质                       | 影响面            |
| ---- | --------------------------------------------------------- | ---- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- | -------------------------- | ----------------- |
| P2-1 | commands/todo.rs:15-19 + App.vue:226 + TrayPreview.vue:82 | 1/4  | emit_filter 方案对 JS 监听者无效（源码实证 Any 恒通过）：主窗每次 todo 变更仍即时收广播 → 勾选 300ms 主拍截断 + 刷新双跑原样在位；注释与行为不符                                       | 载荷携带发起窗 label，前端两监听处 origin 自判早退；emit_filter 删除 | 遗留（A005 P2-5 修复无效） | 勾选动效/刷新链   |
| P2-2 | lib.rs:470-472                                            | 1/2  | 非主窗 CloseRequested 早退未 prevent_close（FIX005.1 改残）：托盘窗获焦 Alt+F4 → runtime 默认销毁 → 预览窗 session 内失效至重启                                                        | 早退前对非 main 窗一律 prevent_close                                 | 新增                       | 预览窗生命周期    |
| P2-3 | lib.rs:198,260 + settings.rs:81-82                        | 1/12 | 首启落位左上角 (0,0)：config 缺失 → unwrap_or_default → Default x/y=0 → position_on_monitor 判 true → 白名单③"主屏右下 40px"被绕过，default_position 死路径。复现：删 config.json 启动 | Default x/y 改屏外哨兵（如 i32::MIN）或 setup 按 Option 分支         | 新增（PL014.2 起潜伏）     | 启动落位/配置体系 |
| P2-4 | BubblesView.vue:330-340                                   | 1/8  | FIX005.26 竞态防护恒失效：onUnmounted 嵌套在 onMounted 回调内注册被 Vue 忽略，disposed 永 false，"已卸载当场注销"为死代码；快速切页泄漏监听                                            | disposed 声明与置位移 setup 层（顶层 onUnmounted 内）                | 新增（FIX005.26 修复无效） | Vue 前端          |

**P3 共 14 条**：

| #   | 文件:行号                                                              | 类型  | 描述与建议                                                                                                                                                                     |
| --- | ---------------------------------------------------------------------- | ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | lib.rs:492-499 + App.vue:250-252                                       | 1/8   | 关窗链路脆弱平衡：T 腿 handler 无 preventDefault，JS 自动 destroy 当前被 ACL 静默拒绝才保住 hide 语义——权限恢复即被击穿。T 腿补 event.preventDefault()（关窗决策归 Rust 双腿） |
| 2   | tray.rs:121-130                                                        | 1/4   | 托盘置顶臂先落库后切窗（与 FIX005.6 命令壳层顺序漂移）+ 主窗 None 静默跳过——状态分叉托盘路径残留。对齐先窗口成功再落库                                                         |
| 3   | commands/tray_preview.rs:16 vs TrayPreview.vue:37-38                   | 1/3   | 预览窗空态缩高失效（FIX005.17 副作用）：钳制下限 80 > 空态实际高度 ≈51 → 命令拒绝停留旧高度 + 错误日志。下限降至前端可达值（如 40）并注释联动                                  |
| 4   | BubblesView.vue:219-223 + TodoList.vue:282                             | 5/6   | FIX005.24 收敛残留：BubblesView 旧注释块（描述已删实现）；TodoList observer `!ul \|\|` 恒 false 死条件                                                                         |
| 5   | useScrollKit.ts:54                                                     | 2/6   | unmount 摘标记硬编码 dataset.mounted 未走 opts.mountedFlag.key——接口承诺与实现不一致                                                                                           |
| 6   | App.vue:216-237                                                        | 2/13  | 启动链四个 await listen 无 catch——任一 reject 中断整个 onMounted（refresh×5/关窗 T 腿注册全跳过）。逐 listen 补 catch                                                          |
| 7   | App.vue:231-237 + TrayMenu.vue:53-59                                   | 11/4  | prefs-changed 载荷两窗内联类型——Rust PrefsSnapshot 已单源，前端补 types.ts 镜像（PrefsView）                                                                                   |
| 8   | commands/settings.rs:16-29                                             | 6/11  | 新命令三处小瑕：文档"上限上限"笔误/返回裸 u32 违背 Result 约定/:29 注释"1~20"字面未随常量口径                                                                                  |
| 9   | commands/tray_preview.rs:17,25 + tray_menu.rs:66 + settings.rs:138,141 | 11/13 | CommandError::Settings 挪用为窗口/校验通用错误（扩面）——增 Window(String) 变体或登记豁免                                                                                       |
| 10  | lib.rs:73-81 vs tray.rs:142-147                                        | 13/4  | snap_if_needed 锁失败回退 false 未登记白名单，且与 snap_current 回退 true（已登记）同场景两方向——补登记或统一                                                                  |
| 11  | lib.rs:296                                                             | 13    | 默认热键 parse(...).expect——A005 P3-11 同族漏网，改 match+日志                                                                                                                 |
| 12  | lib.rs:136-137 + tauri.conf.json                                       | 3/12  | default_position 硬编码 300×400 与 conf 双处来源——读 inner_size 或注释钉死联动                                                                                                 |
| 13  | examples/seed_data.rs:108                                              | 13    | unwrap_or(0) 静默吞读库错误（同文件其他路径均 exit(1)）——对齐                                                                                                                  |
| 14  | tray.rs:216-227                                                        | 1     | Enter 重置武装打断"预览窗→图标"连续性（Shown 被重置 Armed → 预览闪没 0.5s 重停）——Shown 态仅更新锚点不重武装                                                                   |

### 二、参考级观察项 → 已定案归档（FIX011）

> 明细已整合至 §四《审计观察项豁免定案清单》（永久 39 / 条件 7 / CSP 活化 FIX011.1），本节不再保留（2026-10-03 用户定案，本清单为唯一事实源）。

### 三、亮点

SQL 全参数化、锁序单向、tray.rs 窗口调用零锁内违规（专项走查）；FIX004 时代防护全部完好；v-html 零注入面、零 any、零空 catch；组件链 emit 覆盖核查全场景闭合；上限单源三点一致；A005 修复 28/31 实质在位。本轮最大价值 = diff/语义级复核抓出 grep 级复核的三项"修复无效"——grep 在位 ≠ 语义生效。

## 附录 A007：全量代码审计报告（第7轮，2026-10-02）

> 范围：全仓通读（约 19600 行）。方式：三路并行（Rust 业务 / Tauri 集成 / 前端）+ A006/FIX006 回归复核（主会话 grep 20 项全命中 + 子代理 git diff 级 + 语义级推演——延续 A006 "grep 在位 ≠ 语义生效"教训）。基线 7907cb4 → 386b049。
> 状态：📌 待修复（FIX007 任务清单见 x.progress.md；观察项默认全不提升）

### 零、上轮修复复核清单（A006/FIX006 20 项）

| 结果                | 条目                                                                                                                                                                                                                 |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ✅ 语义级在位 19 项 | origin 载荷方案（端到端实证）/ 非主窗 prevent_close / 默认热键两级 match / CommandError::Window 归户 / 启动链 catch×4 / PrefsView 镜像 / disposed setup 层 / 摘标记对称 / main-drag 粒度 / Shown 连续性 / 陷阱入库等 |
| ⚠️ 语义级残留 1 项  | FIX006.3 哨兵只修半边：Default impl 改 i32::MIN，但 x/y 字段级 #[serde(default)] 仍回 0——config 存在但手删 x/y 字段经第二路径复活左上角落位（settings.rs:59-63，确定性复现）                                         |
| 并发纪律专项        | tray.rs 全部窗口调用零锁内违规、WATCH 与 settings 锁零嵌套、EXITING 销毁链跨文件推演闭环                                                                                                                             |

### 一、P0-P3 修复清单

无 P0/P1/P2（A006 修完后 P2 级清零）。**P3 共 18 条**（跨组同根合并）：

| #   | 文件:行号                                                   | 类别 | 描述与建议                                                                                                                                                                                               |
| --- | ----------------------------------------------------------- | ---- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | settings.rs:59-64                                           | 1/12 | 哨兵字段级残留：x/y 改 #[serde(default = "sentinel_xy")] 回 i32::MIN，补"config 存在但缺 x/y"锁定用例                                                                                                    |
| 2   | core/tauri.conf.json                                        | 12   | 无 beforeBuildCommand——npm run tauri build 不重建前端，打包产物内嵌陈旧 dist。打包发布前必须补                                                                                                           |
| 3   | hotkey.rs:208-225                                           | 8/1  | reregister 父侧 swap(0) 自我拆台：先清零后"等旧线程退出"条件恒 false（死等待），Unregister/Register 竞窗致换热键偶发误报占用 → 回滚同一组合再竞失败 → 热键悬空至重启。去 swap(0) 改等旧线程自行 CAS 清零 |
| 4   | TodoList.vue:70-93 + ArchiveOverlay.vue:236-252             | 1    | 同行 300ms 主拍内二次点击 = 双 toggle 翻回：无 in-flight 守卫，二次 invoke 把落库翻回，拍子到点行"弹回"；archived 按陈旧 wasUndone 误上抛。主拍在飞二次点击改视觉回退或忽略                              |
| 5   | DetailOverlay.vue:145,170                                   | 1    | rename/note 相等判定引用陈旧 props.todo（App 刷新不回写 detailTodo）：改名 A→B 再改回 A 恒相等 → 不保存 → UI/DB 静默分叉。保存成功后维护 lastSaved 基准                                                  |
| 6   | TrayMenu.vue:54-57 + TrayPreview.vue:85-88                  | 13/2 | await listen 无 catch（FIX006.10 同族漏网，A006 只框定 App.vue）：两窗常驻不重建，注册失败 = session 级失效。对齐 catch 同款                                                                             |
| 7   | DetailOverlay.vue:156-212 + BubblesView.vue:186-212         | 2/13 | 保存失败无可见反馈两处（详情板 debounce/flush 仅 console.error；气泡清空先 DOM 塌缩后落库失败呈"假空"）。对齐错误行模式                                                                                  |
| 8   | mock-invoke.ts:126-132                                      | 10/3 | mock validateText 上限 100 与 Rust MAX_TEXT_LEN=24 漂移（注释自称对齐已失真）。常量对齐 24                                                                                                               |
| 9   | TodoList.vue:52-58,97-99,142 ≈ BubblesView ≈ ArchiveOverlay | 4    | FIX005.24 收敛遗漏面：rowEl 双份/pinLeaveHeight 三份/180ms 消歧双份——收敛进 composable                                                                                                                   |
| 10  | AddBar.vue:36 + TodoList.vue:329 + DetailOverlay.vue:235    | 3/4  | maxlength="12" 三处散装，与 Rust MAX_TEXT_LEN=24 的"2 倍余量"锚点仅在 Rust 注释——types.ts 出 UI 上限常量三处引用                                                                                         |
| 11  | commands/settings.rs:37,76,144,183                          | 4    | settings_path().map_err(...) 同一习语一处文件内重复四次——收敛 settings_path_or_err() 单点                                                                                                                |
| 12  | commands/settings.rs:204                                    | 4/6  | emit_prefs_changed fn 内 use Emitter（FIX005.9 引入）——并入顶部导入                                                                                                                                      |
| 13  | tray.rs:270,308                                             | 4/3  | 锚定 MARGIN=8 双函数各声明一份——提模块级常量单源                                                                                                                                                         |
| 14  | lib.rs:531,547                                              | 3/4  | 拖动静默阈值 150ms 两处字面无编译期关联——提 DRAG_QUIET_MS                                                                                                                                                |
| 15  | lib.rs:139                                                  | 6    | 注释残渣"与实验场卡片 300×400 一致"（FIX006.16 后失真）——删或改写                                                                                                                                        |
| 16  | lib.rs:65-66                                                | 6/13 | snap_if_needed 注释"白名单候选"过时（FIX006.14 已登记）——更新措辞                                                                                                                                        |
| 17  | commands/whiteboard.rs:28                                   | 4    | 手写 map_err 与 mod.rs 既有 From<WhiteboardError> 重复——? 直转                                                                                                                                           |
| 18  | tray.rs:357-384                                             | 1/8  | Show/Suppressed 跨线程竞态（需验证）：守候锁内提交 Show 后锁外执行前右键可完成 Suppressed+hide → 预览伴菜单弹出且滞留至下次 hover。Show 执行前重入锁复核 phase 仍为 Shown                                |

### 二、参考级观察项 → 已定案归档（FIX011）

> 明细已整合至 §四《审计观察项豁免定案清单》（永久 39 / 条件 7 / CSP 活化 FIX011.1），本节不再保留（2026-10-03 用户定案，本清单为唯一事实源）。

### 三、亮点

A006 全部 20 项修复语义级在位零冲掉（上轮"grep≠语义"教训已内化为审计方法）；origin 载荷方案端到端九调用点推演无刷新丢失；EXITING 销毁链跨文件推演闭环（三腿互洽）；并发纪律锁外铁律零违规、锁零嵌套；SQL 全参数化、v-html 零注入、零 any；FIX004 时代防护全部完好；capabilities 拆分后权限面与 label 分流严格对应。

## 附录 A008：全量代码审计报告（第8轮，2026-10-03）

> 范围：全仓通读。方式：三路并行（Rust 业务 / Tauri 集成 / 前端）+ FIX007 回归复核（主会话 grep 18 项锚点 + 子代理 `git show 6043196` diff 级比对 + P2 两条主会话亲验代码链）。基线 6043196（V0.1.8.6，工作区净）。
> 状态：📌 待修复（FIX008 任务清单见 x.progress.md；观察项默认全不提升）

### 零、上轮修复复核清单（FIX007 18 项）

| 结果                 | 条目                                                                                                                                                                                                                                                                                      |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ✅ 语义级在位 15 项  | 哨兵字段级三扇门（含锁定用例）/ beforeBuildCommand / hotkey 去 swap(0)（时序自洽）/ toggleTimers / TrayMenu+TrayPreview catch / saveError 与清空失败提示 / mock 24 / UI_TEXT_MAX_LEN / settings_path_or_err / Emitter 上提 / ANCHOR_MARGIN / DRAG_QUIET_MS / 注释两处 / whiteboard ? 直转 |
| ⚠️ 修复引入回归 1 项 | **FIX007.18**：重入复核块把 menu_is_open 写进 WATCH 锁内（见 P2-1，本轮最大发现——触发窗口恰是修复目标场景本身；死锁铁律在模块文档三处自证，正因此可静态判定）                                                                                                                             |
| ⚠️ 修复不完整 2 项   | **FIX007.9** 只收敛 2/3：useClickDisambiguate 建了但全仓零引用，180ms 消歧仍双份活实现（三处两活一死，见 P3-5）；**FIX007.5** lastSaved 基准跨条目残留两路径（见 P3-1）                                                                                                                   |
| 并发纪律专项         | 主循环 :355-357 窗口调用全部锁外 ✅，但 :378-381 新增复核块违规 ❌；WATCH 与 settings 锁零嵌套其余在位                                                                                                                                                                                    |

### 一、P0-P3 修复清单

无 P0/P1。**P2 共 1 条，P3 共 14 条**（跨组同根已合并）。收敛态势：P2 在 A006 清零后本轮 +1（上轮修复引入，非存量暴露）；P3 18→14。

| #     | 文件:行号                                                  | 类别  | 描述与建议                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | 性质                    | 影响面                   |
| ----- | ---------------------------------------------------------- | ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------- | ------------------------ |
| P2-1  | tray.rs:378-381                                            | 8     | **FIX007.18 复核块锁内调窗 = 死锁铁律违规**：`st` 守卫存活至块尾，`&&` 右操作数 menu_is_open（内部 is_visible 同步等主线程应答）在持锁状态求值——主线程此间到达 on_tray_enter:227 / on_tray_right_button:254 的 lock_watch() 即互等，进程全挂需强杀；函数文档 :403-404 自证"只允许锁外调用"。修法：锁内只拷 `matches!(st.phase, Phase::Shown{..})`，menu_is_open 移锁外（同文件 :355-357 与 reanchor_to_tray:324-328 已示范正确形态）                                                                                                                    | 新增（修复引入回归）    | Tauri 后端（进程级挂死） |
| P3-1  | DetailOverlay.vue:148/186/168/235/248                      | 1     | **lastSaved 基准跨条目残留两路径**：(a) watcher 创建序 :106→:148→:186——切 A→B 时 :148 装载 B 文本时基准仍是 A 会话值，误建 pending {id:B, text:B.text}，:186 只重置基准不清 pending/timer → 300ms 后无果 rename + changed 全量重拉（A 改过名再切换即确定性触发）；(b) flushPending 的 .then 落地晚于 :186 重置，无 `pending.id === props.todo?.id` 校验把 A 的值写进基准 → 用户把 B 改成恰为污染串时跳过保存 = UI/DB 静默分叉（FIX007.5 要根治的 bug 跨条目复活）。修法：基准与 pending 携带条目 id，.then 落地前校验 id，id-watch 同时清 pending+timer | 新增（FIX007.5 引入面） | Vue 前端                 |
| P3-2  | tray.rs:207-213                                            | 1/8   | **Suppressed 臂缺可见窗 Hide 兜底**（A007 #18 残余）：复核通过到 w.show() 落地间右键仍可完成 Suppressed+收窗，随后 show 后到 → 预览伴菜单滞留至下次 hover。修法：step_watch Suppressed 臂加 `window_visible → Hide`（输参已有，纯函数 + 一条单测），兼根治 P2-1 修复后的残余竞态                                                                                                                                                                                                                                                                        | 遗留（需验证）          | Tauri 后端               |
| P3-3  | core/src/main.rs:1                                         | 12    | **缺 windows_subsystem 属性**（Tauri 模板常规行）：release 直启 exe 必伴生控制台黑窗。补 `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`（debug 保留控制台看日志）。**双刃注意**：release 后全部 eprintln 落日志承诺静默失效，打包发布时需同步决策文件日志或接受。与 CSP 同列"打包前必办"                                                                                                                                                                                                                                          | 新增                    | 配置体系（打包阻断）     |
| P3-4  | settings.rs:119 + AGENTS.md 白名单⑤                        | 13    | **白名单⑤承诺与实现边界漂移**：登记场景"max_bubbles 越界(<1 或 >20) 静默钳制"，但 -1/5.5/"5" 走 serde 严格解析失败 → exit(1) 拒绝启动（0/99 钳制正常）。同场景一处静默一处严格。修法二选一：a) 白名单⑤措辞收窄为"可解析为 u32 的越界整数"（零代码）；b) 字段级容错解析（须先登记新白名单）                                                                                                                                                                                                                                                              | 新增                    | 文档/错误策略            |
| P3-5  | useListRow.ts:35-54 + TodoList.vue:99 + BubblesView.vue:49 | 5/4   | **useClickDisambiguate 零引用死代码**：FIX007.9 声称收敛 180ms 消歧双份，实际新建导出全仓无调用，两组件本地 clickTimer 原样保留——180ms 现存三处（两活一死）。修法二选一：迁移两组件到 composable，或删死导出（迁移牵动 installDragHooks cancelPendingClick 回调签名，FIX007.9 裁量注记在案）                                                                                                                                                                                                                                                            | 遗留（FIX007.9 未竟）   | Vue 前端                 |
| P3-6  | TodoList.vue:73-91 + ArchiveOverlay.vue:235-247            | 1     | **in-flight 守卫不覆盖 `await invoke` 在飞窗口**：条目在 invoke resolve 后才进 toggleTimers/restoreTimers，IPC 往返期（本地 1~10ms）二次点击穿透守卫双 toggle。A007 #4 的 300ms 主拍已闭合，残余仅 IPC 窗口（理论，需验证）。修法：进 try 前同步登记哨兵，finally 兜底删除                                                                                                                                                                                                                                                                              | 遗留（A007 #4 残余）    | Vue 前端                 |
| P3-7  | bubble.rs:43                                               | 1/4   | **校验口径漂移**：validate_bubble_text 按**未 trim 原文**计数，todo 侧按 **trim 后**计数，两处文档同措辞且入库形态均为 trim 后——首尾带空白且贴 2000 上限时误拒（todo 同构输入放行）。修法：先 trim 再计数，补边界用例                                                                                                                                                                                                                                                                                                                                   | 新增                    | 业务状态机               |
| P3-8  | storage.rs:359-370 vs 447-463                              | 4/6   | **气泡行映射闭包双份**（list_bubbles 与 get_bubble 各写一份，todo 侧已有 row_to_item 共用先例）；连带 add_bubble 文档缺"调用方须传 trim 后文本"契约（trim 执行点在命令层，直调 storage 即失守）。修法：抽 row_to_bubble 镜像 todo 形态 + 补文档                                                                                                                                                                                                                                                                                                         | 新增                    | 存储层                   |
| P3-9  | lib.rs:291-295                                             | 13/10 | **设置锁读失败静默回默认热键**：unwrap_or_else 直接以默认热键注册，用户自定义热键无声替换，零日志（同函数 parse 失败分支有完整两级日志），白名单⑧未含锁读失败。修法：补一行 eprintln 对齐措辞，或白名单登记                                                                                                                                                                                                                                                                                                                                             | 新增                    | Tauri 后端               |
| P3-10 | lib.rs:150-164                                             | 13/10 | **available_monitors 失败静默判"越界"回默认位**：unwrap_or(false) 使显示器枚举失败与真实越界同路，零日志（对比 :82 吸附路径同族失败有日志）。修法：补日志或扩白名单③措辞                                                                                                                                                                                                                                                                                                                                                                                | 新增                    | Tauri 后端               |
| P3-11 | commands/settings.rs:205-215                               | 13    | **emit_prefs_changed 锁失败跳过广播未登记**：有日志、下次切换自愈，但这是 FIX005.14/FIX006.14 同族第三处，规则上"新增容错须先登记"。修法：AGENTS.md 白名单补登记（或 FIX005.14 条目扩围）                                                                                                                                                                                                                                                                                                                                                               | 新增                    | 错误策略                 |
| P3-12 | hotkey.rs:266-268                                          | 1/13  | **注册等待超时后孤儿线程不回收**：recv_timeout(500ms) 超时 → 命令层回滚落库旧热键，但已 spawn 的新线程若只是慢而非败，稍后仍注册成功 → 实际生效热键与 config/UI"已回退"提示相反，下次换热键才收敛（系统高载才触发）。修法：注册成功后二次确认父侧未放弃否则自行 Unregister，或接受现状注释登记                                                                                                                                                                                                                                                          | 新增（需验证）          | Tauri 后端               |
| P3-13 | mock-invoke.ts:324 vs hotkey.rs:102-116                    | 10/11 | **mock 热键回显序与 Rust 漂移**：Rust to_display 固定 Ctrl→Alt→Shift→Win（有测试断言），mock 比较器 indexOf(b)-indexOf(a) 降序 = Win→Shift→Alt→Ctrl，同输入回显形态不同，IAB 断言可能锁死假形态。修法：mock 比较器反转                                                                                                                                                                                                                                                                                                                                  | 新增                    | Vue 前端（仅 DEV）       |
| P3-14 | todos.css:19 + bubbles.css:8                               | 5     | **.placeholder 空态选择器零引用**（模板统一用 .empty，design 1:1 迁移残留）。修法：删除或加"design 对齐保留"注释                                                                                                                                                                                                                                                                                                                                                                                                                                        | 新增                    | Vue 前端                 |

### 二、参考级观察项 → 已定案归档（FIX011）

> 明细已整合至 §四《审计观察项豁免定案清单》（永久 39 / 条件 7 / CSP 活化 FIX011.1），本节不再保留（2026-10-03 用户定案，本清单为唯一事实源）。

### 三、亮点

FIX005/FIX006 全部修复面保持完好零冲掉；死锁铁律的模块文档自证完备（正因三处注释在位，P2-1 才可静态判定）；A006 组项（置顶臂顺序/HEIGHT_MIN/CommandError 归户/去 expect/default_position 单源）全在位；SQL 全参数化、v-html 零注入、零 any 维持；types.ts 与 serde DTO 镜像核对一致（含 tag="status" snake_case 形态）；P2 级两条（死锁/lastSaved）均主会话亲验代码链成立，非子代理转述。
**方法论注记**：连续两轮出现"修复引入/修复不完整"（A007 抓 FIX005 三项 → 本轮抓 FIX007 三面），下轮修复后建议对修复项做一次 diff 级互查再收口。

## 附录 A009：全量代码审计报告（第9轮，2026-10-03）

> 范围：全仓通读。方式：三路并行（Rust 业务 / Tauri 集成 / 前端）+ FIX008 回归复核（主会话 grep/sed 16 锚点 + 三子代理 `git show ee08b0f` diff 级 + P3 关键条目主会话亲验代码链）。基线 ee08b0f（V0.1.8.7，工作区净）。
> 状态：📌 待修复（FIX009 任务清单见 x.progress.md；观察项默认全不提升）

### 零、上轮修复复核清单（FIX008 16 项）

| 结果                | 条目                                                                                                                                                                                                                                                                                           |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ✅ 语义级在位 16 项 | 16/16 diff 级确认零回退：复核块拆两步 / Suppressed 兜底（含单测）/ 基准带 id 六处判定 / 哨兵先行+catch 摘除 / windows_subsystem（PE GUI）/ 白名单两处 / 死代码零残留 / trim 口径+新用例 / row_to_bubble 两点直传 / 两处补日志 / REREGISTER_ABORTED 三段 / mock 升序 / .placeholder 清理 / 收尾 |
| ⚠️ 修复面残余 3 项  | **FIX008.13** 放弃标志可被回滚轮复位抹掉（宣称"消除分叉"未全闭合，见 P3-1）；**FIX008.9** trim 契约只补了 bubble 半边（P3-7 同构未及面）；**FIX008.13** diff 插行致 :214 注释行号锚失准（P3-10）                                                                                               |
| 并发纪律专项        | 6 处 lock_watch() 锁点全检零窗口调用——死锁铁律无第四次违规 ✅                                                                                                                                                                                                                                  |

### 一、P0-P3 修复清单

无 P0/P1/P2（P2 持续归零）。**P3 共 11 条**：

| #     | 文件:行号                                                 | 类型  | 描述与建议                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | 性质                        | 影响面                 |
| ----- | --------------------------------------------------------- | ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------- | ---------------------- |
| P3-1  | hotkey.rs:218 复位 + :253/:262 双查 + settings.rs:93 回滚 | 8/1   | **放弃标志被回滚轮复位抹掉 → 双热键幽灵**：轮 1 超时置 ABORTED → 命令层回滚立即 `reregister(旧A)`（settings.rs:93）→ :218 复位抹掉轮 1 信号（线程 1 stall 中未达双查）→ 线程 1 苏醒双查见 false 进循环（B 生效），线程 2(A) :258 plain store 覆盖 tid → A+B 双热键并存且 B 失管（下次换热键只回收其一）。触发 = stall 跨越回滚窗口（系统高载，需验证）。修法：bool 换代际计数 AtomicU64 GEN（reregister 进入 +1，线程入口捕获、双查比代际——"复位"语义消失）+ :258 改 CAS；回滚路径同批回归 | 遗留（A008 P3-12 残余）     | Tauri 后端             |
| P3-2  | TodoList.vue:96-98 + ArchiveOverlay.vue:250-252           | 1     | **catch 摘哨兵无"末拍收口"交接**（FIX008.7 引入面）：收口约定 = 每个在飞条目最终走到拍子回调、末拍 `size===0` 才 emit changed。行 1 成功进主拍 + 行 2 IPC 失败且在飞超 300ms → 行 1 拍子到点 size 仍 >0 不发、行 2 catch 摘除也无人发 → 本轮 changed 整体丢失，行 1 滞留"视觉已勾"。修法：catch 摘除后补 `if (size === 0) emit("changed")`（两处同构）                                                                                                                                     | 新增（FIX008.7 引入面）     | Vue 前端               |
| P3-3  | TodoList.vue:210-214                                      | 1     | **watchEffect 拖拽期早退致依赖丢失、效果惰性化**：`deferDuringDrag()` 早退在 `void props.items.length` 之前——拖拽期首次重跑即零依赖收集，effect 此后失活，罩死灰化复核滞后至收场。修法：`void props.items.length` 上移到早退判断之前（冻结渲染不冻结依赖登记）                                                                                                                                                                                                                             | 新增（存量暴露）            | Vue 前端               |
| P3-4  | BubblesView.vue:187-208/240-254 + TodoList.vue:105-146    | 1/2   | **行离场动画窗口点击开"幽灵板"**：删除/清空后行处于 leave 塌缩（320~480ms 无 mask-dead）仍可点击 → 180ms 后携已删条目开板展示死文本（气泡侧 readonly 无任何写入防御）。修法：onRowClick 入口校验条目仍在 items                                                                                                                                                                                                                                                                             | 新增                        | Vue 前端               |
| P3-5  | App.vue:228-233/316 + BubblesView.vue:68/321-331          | 4/8   | **气泡页激活时单次 bubble-changed 三跑 bubble_list**（App 监听 + BubblesView 监听 + refresh emit 链回环）——徽章双刷新冗余 IPC。修法：emit 链与 App 监听二留一                                                                                                                                                                                                                                                                                                                              | 新增                        | Vue 前端               |
| P3-6  | DetailOverlay.vue:261-264/277-280                         | 13    | **flush 失败错误归因未随 id 锚定**（FIX008.2 同构未及面）：切源后旧条目 flush 失败时 saveError 落在新条目板上（A 的错误挂 B 板）。修法：与基准同构，错误仅当前板内条目显示或文案带 id                                                                                                                                                                                                                                                                                                      | 新增（FIX008.2 未及面）     | Vue 前端               |
| P3-7  | storage.rs:208-211/248                                    | 6     | **todo 侧 trim 契约文档缺失**（FIX008.9 只补了 add_bubble 半边）：add/rename 直调绕过命令层即落未 trim 原文，文档未声明契约。修法：镜像 add_bubble:341-344 措辞补齐                                                                                                                                                                                                                                                                                                                        | 新增（FIX008.9 同构未及面） | 存储层（文档）         |
| P3-8  | commands/mod.rs:107-111                                   | 11/13 | **BubbleError 校验错误错桶进 Clipboard 变体**："内容为空/过长"属业务规则拒绝，与变体文档"剪贴板读写失败"不符（对照 Todo/Whiteboard 均有专属变体）；被 bubble.rs:248 测试锁死。修法：加 `Bubble(String)` 变体 + 改断言，或注释登记历史沿袭                                                                                                                                                                                                                                                  | 新增                        | Tauri 后端             |
| P3-9  | tray.rs:305 + :11-12                                      | 6     | **线程模型注释两处失实**：(a) anchor_preview 称"守候线程持锁调用，避免重入死锁"——实际两调用点均锁外，按此注释理解恰会复制出死锁违规形态；(b) 模块文档"显隐只由守候线程执行"——右键路径 :266-270 事件直调收窗。修法：两处注释改与现状一致                                                                                                                                                                                                                                                    | 新增（存量）                | Tauri 后端（审计锚点） |
| P3-10 | hotkey.rs:214                                             | 6     | **注释行号锚失准**（FIX008.13 插行漏改）：":254-259 配套"实际 CAS 已移 :264/:284。修法：更新行号或改语义描述免行号                                                                                                                                                                                                                                                                                                                                                                         | 新增（FIX008.13 漏改）      | 文档                   |
| P3-11 | hotkey.rs:199/:276                                        | 13    | **ON_HOTKEY 锁中毒 `into_inner` 恢复未登记白名单**（WATCH 锁同款已登记⑩，此处两处漏登；语义实际安全——回调 spawn 期一次写入）。修法：白名单⑩措辞扩围覆盖热键回调锁（零代码）                                                                                                                                                                                                                                                                                                                | 新增（存量）                | 错误策略               |

### 二、参考级观察项 → 已定案归档（FIX011）

> 明细已整合至 §四《审计观察项豁免定案清单》（永久 39 / 条件 7 / CSP 活化 FIX011.1），本节不再保留（2026-10-03 用户定案，本清单为唯一事实源）。

### 三、亮点

FIX008 16/16 diff 级零回退；死锁铁律 6 锁点全检零违规（FIX008.1 修复后无第四次）；类型镜像零漂移（TodoItem 六字段 / BubbleCaptureOutcome snake_case / PrefsView 全对齐）；FFI 签名全组核对与 Win32 契约一致；135 测试断言复核为真断言非恒真。
**收敛态势**：P2 持续归零，P3 18→14→11 递减；新增条目中 3 条为 FIX008 修复未竟面——但级别全部降至文档/低影响面（上两轮的未竟面是死锁与数据分叉级），"修复不完整"模式危害递减，互查纪律生效中。

## 附录 A010：全量代码审计报告（第10轮·收官轮，2026-10-03）

> 范围：全仓通读。方式：三路并行（Rust 业务 / Tauri 集成 / 前端）+ FIX009 回归复核（主会话 12 锚点 + 三子代理 `git show 1e6f42d` diff 级 + 三个关键条目主会话亲验代码链）。基线 1e6f42d（V0.1.8.8，工作区净）。
> 状态：📌 待修复（FIX010 任务清单见 x.progress.md；观察项默认全不提升）

### 零、上轮修复复核清单（FIX009 12 项）

| 结果                | 条目                                                                                                                                                                                                                                                                                         |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ✅ 语义级在位 12 项 | 代际计数三段联动（fetch_add/双查比代际/tid CAS，四交错推演含 stall 跨回滚窗已根除）/ 末拍收口×2 / 依赖收集首行 / 幽灵板四校验（click 时刻）/ notify 语义分布 / 归因判定八处+id-watch 清残留（双时机无互误清窗口）/ trim 契约三处 / Bubble 变体四点 / 注释纠偏两处 / 语义描述锚 / 白名单⑩扩围 |
| ⚠️ 修复面残余 2 组  | **FIX009.4** click 时刻校验 ≠ 180ms 拍子到点时刻事实（见 P3-1）；**FIX009.8/9** 各漏 1-3 处同族位（见 P3-3/P3-5）                                                                                                                                                                            |
| 并发纪律专项        | 死锁铁律全组锁点复核零违规（含 lib.rs:511 主线程回调直调边界判定不违铁律）；锁序 storage→settings 无反向                                                                                                                                                                                     |

### 一、P0-P3 修复清单

无 P0/P1/P2（连续三轮归零）。**P3 共 13 条**（跨组同根合并）：

| #     | 文件:行号                                                            | 类型 | 描述与建议                                                                                                                                                                                                                                                                                                                                                                                                   | 性质                          | 影响面                 |
| ----- | -------------------------------------------------------------------- | ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------- | ---------------------- |
| P3-1  | TodoList.vue:152 + BubblesView.vue:261 + App.vue:108/116             | 1    | **180ms 拍子期间条目被删 → 幽灵板**（FIX009.4 残余）：四处校验在 click 时刻，开板 emit 在拍子回调携闭包快照到点不复核；App.onOpenDetail `find(...) ?? item` 回落把已删条目复活为 detailTodo（onOpenBubble 连复核都没有）。确定性交错 = 删除二态确认第二击未决期间单击同条目正文。伴生：onRowDblClick 校验失败路径在 clearTimeout 之前 return，已武装拍子不被掐。修法：拍子回调内补同款复核（一并覆盖伴生面） | 新增（FIX009.4 未竟面）       | Vue 前端               |
| P3-2  | ArchiveOverlay.vue:96-97                                             | 1    | **归档按钮幂等守卫不看在飞拍子**：320ms 塌缩拍子未落地时切页，守卫命中提前 return（clearTimeout 在守卫后够不着）→ 拍子照发 btnHidden=true——用户已在清单页而归档入口隐身，需再切一轮自愈（320ms 内两次互切确定性复现）。修法：clearTimeout/在飞判定前置到守卫前（design 同构同病，修即有意分叉需登记）                                                                                                        | 新增（存量，design 移植形态） | Vue 前端               |
| P3-3  | tray.rs:62/:339 + lib.rs:421                                         | 6    | **三处"show/hide 唯一调用者"绝对化表述漏改**（FIX009.9 只改了模块头与 anchor_preview 两处）：与模块文档新例外自相矛盾，按旧注释会误判右键直调 hide 为违规。修法：三处统一补"预览 Shown 态唯一调用者；右键收窗例外（见模块注记）"                                                                                                                                                                             | 新增（FIX009.9 漏改）         | Tauri 后端（审计锚点） |
| P3-4  | fullscreen.rs:101-107                                                | 13/8 | **设置锁中毒降级违意开置顶**：`unwrap_or(true)` 假定置顶开后继续让位/恢复行动链——用户已关置顶时恢复臂 set_always_on_top(true) 违意打开且维持；注释引用白名单②但②登记场景是"轮询失败维持不变"，锁失败不在列且语义反向（有一次性日志）。修法：锁失败臂改 `continue` 跳过本轮（对齐白名单②"维持不变"），或按三要素登记                                                                                          | 新增（存量暴露；理论链 ≤ 中） | 全屏让位线程           |
| P3-5  | commands/mod.rs:143-190                                              | 10   | **序列化契约测试缺 Bubble 变体断言**（FIX009.8 未及面——A004 补 Hotkey、A006 补 Window 的惯例每个新变体一条）。修法：补一条断言                                                                                                                                                                                                                                                                               | 新增（FIX009.8 未及面）       | 错误契约测试           |
| P3-6  | commands/settings.rs:83-104                                          | 8    | **并发换热键回滚覆盖分叉**（需验证）：后发轮接管致先发轮线程自灭报 Err → 先发轮回滚写 old 覆盖后发轮已落库新值 → UI 显新值磁盘为旧值，下次保存自愈；前端录制态机使触发窗极窄。修法：回滚写前复核当前热键仍为本轮 normalized                                                                                                                                                                                  | 新增（存量暴露，需验证）      | 热键设置一致性         |
| P3-7  | lib.rs:268                                                           | 10   | setup 锁中毒启动中止丢 Debug 详情（同场景关窗路径 ：513 落 `{err:?}` 口径不一）。修法：补 `{err:?}`                                                                                                                                                                                                                                                                                                          | 新增（存量）                  | 启动失败可诊断性       |
| P3-8  | settings.rs:1                                                        | 6    | 模块头职责清单停留 PL015，缺 PL017 置顶/吸附两字段（与 A004 storage 模块头同构）。修法：头部补齐                                                                                                                                                                                                                                                                                                             | 新增（存量暴露）              | 文档                   |
| P3-9  | settings.rs:91-92 vs :52-54                                          | 4    | 哨兵 i32::MIN 双源：Default impl 写字面量未复用 `sentinel_xy()`（FIX007.1 建的单点），漂移即复活"首启贴左上角"。修法：Default 改调 `sentinel_xy()`                                                                                                                                                                                                                                                           | 新增（存量暴露）              | 配置体系               |
| P3-10 | storage.rs:102-112                                                   | 5    | `open_in_memory`/`open_in_memory_with_now` 测试通道常驻 pub 未 `#[cfg(test)]` 门控（A004 收敛先例在案），release 二进制携带测试通道。修法：加门控或注释登记                                                                                                                                                                                                                                                  | 新增（存量暴露）              | 存储层 API 卫生        |
| P3-11 | mock-invoke.ts:339-342 + :250-258                                    | 10/5 | **mock 基座对齐面两条**：whiteboard_save 无 10_000 长度校验（IAB 复现不了拒绝语义）；`bubble_add` handler 死分支（全仓 UI 与真机 generate_handler 均无此命令）。修法：补校验抛错 + 删死分支                                                                                                                                                                                                                  | 新增                          | IAB 冒烟通道           |
| P3-12 | TodoList.vue:68-103 + ArchiveOverlay.vue:231-256                     | 4    | 勾选/退回哨兵状态机约 30 行双实现——FIX007.4/008.7/009.2 三轮修复均双写同构补丁，漂移成本实证。修法：收敛 useToggleBeat composable（差异注入），或注释登记维持现状                                                                                                                                                                                                                                            | 新增（存量暴露）              | Vue 前端               |
| P3-13 | DetailOverlay.vue:302 + BubblesView.vue:290 + SettingsOverlay.vue:72 | 5    | 三处 defineExpose 零调用（App 不持模板 ref 或仅用部分面）。修法：删除或注释登记 API 预留                                                                                                                                                                                                                                                                                                                     | 新增                          | Vue 前端               |

### 二、参考级观察项 → 已定案归档（FIX011）

> 明细已整合至 §四《审计观察项豁免定案清单》（永久 39 / 条件 7 / CSP 活化 FIX011.1），本节不再保留（2026-10-03 用户定案，本清单为唯一事实源）。

### 三、亮点

FIX009 12/12 diff 级零回退；代际计数三段联动四交错推演全部收敛（A009 P3-1 双热键幽灵已根除）；死锁铁律全组锁点零违规；类型镜像逐字段核对零漂移（TodoItem 六字段 / BubbleCaptureOutcome snake_case / PrefsView / mock 归档排序）；FFI 签名全组核对无误；SQL 全参数化、v-html 零注入、零 any 维持；capabilities 与前端窗口 API 面匹配无缺权。
**收官态势**：P2 连续三轮归零；P3 18→14→11→13（本轮微升系 FIX009 三组未竟面 + 存量暴露集中清点，无行为级新缺陷——最高条目仍是体验/文档面）；五轮审计主线索（死锁铁律/哨兵时序/错误归因/线程模型）全部闭合。**实现面（SQL/事务/校验/状态机/退出链/FFI）零新缺陷——达成收官条件。**

## 附录 FIX011：观察项清理批（2026-10-03，用户定案）

> 背景：A005–A010 收官后，A001–A010 十轮观察项（约 90 条原始条目）整合去重定案（见 §四）——永久豁免 39、条件豁免 7、活化修复 CSP，另有九条早期轮次遗留的轻松修（文档/命名/测试面）与 CSP 一并组成 FIX011 清理批，一次性清账。
> 方案要点：
>
> - **CSP 配置（FIX011.1，打包必办活化）**：`app.security.csp` = `default-src 'self'; connect-src ipc: http://ipc.localhost; img-src 'self' data:; style-src 'self' 'unsafe-inline'; font-src 'self' data:; object-src 'none'`——self 覆盖 dist 静态资源、ipc 两协议覆盖 Tauri IPC 通道、data: 覆盖图标数据 URI、unsafe-inline 限定 style-src（tauri/wry 运行时注入样式所需）、object-src none 加固；dev 无 HMR 静态产物同 CSP 生效，打包回归时三页/托盘窗 live 复验
> - **九条轻松修**：whiteboard 测试上限常量化（对齐 todo.rs 同族形态）/ storage sort 注释 NULL 排序措辞精确化 / hotkey 代际失配自注销双分支补日志（错误串底账）/ storage 测试域 use 统一 mod 级导入 / 拖动静默阈值注释引用常量名 / BUBBLE_MAX_LIMIT ref 驼峰命名 / 设置按钮 aria-label 中文化 / BubblesView onRowClick 删冗余 async / App.vue import 组间空行
> - **定案纪律**：A001–A010 各报告观察项节同步清理为指向 §四（本清单唯一事实源，杜绝单一文件两处描述漂移）
>
> 状态：✅ 已完成（2026-10-03 11/11 闭环；CSP live 复验随打包回归执行）

## 附录 FIX012：打包发布里程碑批（2026-10-03 立项，V0.2.0.0）

> 背景：A005–A010 审计收官 + 观察项清账后进入发布里程碑。打包三决策用户定案：**正式图标 = 用户设计胶囊稿**、**分发形态 = 绿色单 exe**（zip 内 exe + 使用说明，非安装器）、**数据落址 = exe 同级**（双落址基线 release 形态，零代码）。首启默认四项同步定案：置顶关/吸附关/主题跟随系统/默认落位主屏右上。
> 方案要点：
>
> - **图标管线**：2048 RGBA 设计源入库（source.png）→ `tauri icon` 生成 → 裁剪 Windows 五件套（单 exe 分发不需要 icns/UWP/移动端）→ bundle.icon 对齐；托盘经 default_window_icon 自动跟随
> - **首启默认**：serde 钩子 + tauri.conf alwaysOnTop 双层改 false；主题种子 0（跟随，与 design 手动播种有意分叉）；default_position y 顶对齐改右上；tray 锁失败回退值/白名单措辞/测试断言全连带同步
> - **pop 根治**：四变体隔离实验（V0 基线/V1 去过冲/V2 will-change/V3 去阴影）→ 唯 V2 消除 = transform 过渡结束合成层重光栅；三板 `.board-glass` 加 will-change: transform（诊断页 .temp/switch-pop-diag/ 留档）
> - **双击标题缩回托盘**：main_hide_to_tray 命令复用 Alt+F4 关窗链（保存位置 + 隐藏），真退仍只走托盘菜单
> - **发布链路**（待办尾部）：release/README.md（绿色包使用说明，非 GitHub README）→ commit V0.2.0.0 → tag v0.2.0 → 干净 staging 打 zip（.temp/release-pack/，严禁直压 target/release/）→ gh release create → 发布自查
>
> 状态：✅ 已完成（2026-10-04 发布上线：commit 49f88a3 + tag v0.2.0 + GitHub Release 首个正式版；后续排期 = macOS/Linux 适配提为下一件大事、三期 AI 暂缓）

---

## 附录 PATCH001：归档板 7 天自动回收（2026-10-04 立项，[problems#5]）

> 背景：归档板只进不出，勾选完成的条目永久留存，随使用积累成陈年旧账（y.problems#5，用户提出）。关键发现 = **零 schema 改动**：done_at 字段 PL010 已有（勾选落当前时间、退回清空），回收判定与归档排序共用同一事实。
> 方案要点（2026-10-04 用户定案）：
>
> - **回收规则**：已完成且 `done_at < now − 7 天` 即删除；恰好满 7 天不删（沿 age_level"等于阈值不升级"先例）；7 天固定写死业务常量、不做设置项（用户定案 KISS）；NULL done_at（迁移遗留）永不回收（PL010"不模拟历史时间"先例）；退回重勾自动重计时（toggle 清空重写 done_at 的免费语义）
> - **清扫时机 = 归档读路径惰性清扫**（打卡软件模式在常驻板上的等价形态）：挂 todo_archive_list_core 出视图前——该命令调用点天然覆盖 App 启动（App.vue 挂载拉归档）+ 每次清单变更（onListChanged 三源齐拉），超期条目在出视图前已删、前端永无中间态；零新线程零定时器（备选每小时定时线程否决：用户可见行为全同，纯增复杂度）
> - **错误策略**：严格抛错主线不变，清扫失败 = 归档命令报错可见，不新增容错白名单条目
> - **预期行为**：V0.2.1 首拉归档即清掉现存超期条目（用户知悉，"突然少一批"属正常）
> - **同批捎带**：清单行内编辑输入框点击截胡修复（TodoList.vue 行级 click/dblclick 对输入框本体的截胡——单击文本中间被 focusEditEnd 重钉行尾、双击选词重启编辑流丢草稿；用户实测 bug，同根双修）
>
> 状态：✅ 已完成（2026-10-04，V0.2.1.1；实施记录见 x.progress.md PATCH001 组）

---

## 附录 A013：全量代码审计报告（第 11 轮，2026-10-04）

> 范围：全仓通读。方式：主会话 FIX010 十三锚点 grep 复核 + 三路并行全文通读（Rust 业务组 / Tauri 集成组 / 前端组，各路 `git diff e16304f..HEAD` 逐行补盲）。基线 `1665b52`（V0.2.1.1，工作区净）。豁免对照 §四清单（永久 39 + 条件 7）。**编号说明：A/FIX 同号绑定规范首用（A 跟 FIX 走，见 AGENTS.md 任务清单纪律），第 11 轮取号 A013/FIX013。**
> 状态：✅ 已完成（2026-10-04，FIX013 全组 12/12 闭环；IAB live 断言经用户定案豁免随日常使用复核，如实记录）

### 零、上轮修复复核清单（全组：FIX010 + FIX011 + FIX012 + CI 三提交 + PATCH001）

| 组       | 条目                          | 现状 | 证据                                                                     |
| -------- | ----------------------------- | ---- | ------------------------------------------------------------------------ |
| FIX010   | .1 拍子到点复核防幽灵板       | ✅   | TodoList/BubblesView 拍子内 `some((it` ×6；App.vue `?? item` 零命中      |
| FIX010   | .2 归档按钮守卫看在飞拍子     | ✅   | ArchiveOverlay.vue:99 clearTimeout 前置于 ：100 守卫                     |
| FIX010   | .3 "唯一调用者"例外补齐       | ✅   | tray.rs ×3 + lib.rs ×1 全带"右键收窗"限定                                |
| FIX010   | .4 fullscreen 锁失败 continue | ✅   | fullscreen.rs:109-126                                                    |
| FIX010   | .5 Bubble 契约断言            | ✅   | commands/mod.rs:74                                                       |
| FIX010   | .6 换热键回滚写复核           | ✅   | commands/settings.rs:84-97                                               |
| FIX010   | .7 setup 锁日志 Debug 详情    | ✅   | lib.rs:269                                                               |
| FIX010   | .8 settings 模块头补齐        | ✅   | settings.rs:2                                                            |
| FIX010   | .9 哨兵单源                   | ✅   | settings.rs:93-95                                                        |
| FIX010   | .10 测试通道门控              | ✅   | storage.rs:104/:110                                                      |
| FIX010   | .11 mock 基座对齐             | ✅   | mock-invoke.ts:250/:334-336                                              |
| FIX010   | .12 勾选哨兵登记              | ✅   | TodoList/ArchiveOverlay 登记注释                                         |
| FIX010   | .13 defineExpose 清理         | ✅   | DetailOverlay.vue:302 / BubblesView.vue:296 删除留档                     |
| FIX010   | .14 收尾门禁                  | ✅   | CI 云端同口径复证                                                        |
| FIX011   | .1 CSP                        | ✅   | tauri.conf.json:12                                                       |
| FIX011   | .2 whiteboard 测试常量化      | ✅   | commands/whiteboard.rs:61 `MAX_CONTENT_LEN + 1`                          |
| FIX011   | .3 storage sort 注释措辞      | ✅   | storage.rs:318                                                           |
| FIX011   | .4 hotkey abort 双分支日志    | ✅   | hotkey.rs:261/:286                                                       |
| FIX011   | .5 storage tests use 收敛     | ✅   | diff 级复核在位                                                          |
| FIX011   | .6 拖静默阈值注释语义化       | ✅   | lib.rs:33/:38/:545-547                                                   |
| FIX011   | .7 BUBBLE_MAX_LIMIT ref 命名  | ✅   | SettingsOverlay 声明+3 引用同步                                          |
| FIX011   | .8 tooltip 中文化             | ✅   | SettingsOverlay aria-label                                               |
| FIX011   | .9 BubblesView 删冗余 async   | ✅   | 函数体无 await                                                           |
| FIX011   | .10 App.vue import 组间空行   | ✅   | App.vue:8/:17                                                            |
| FIX011   | .11 收尾验证                  | ✅   | CI run 37139480687/37149067675 success                                   |
| FIX012   | .1 正式图标                   | ✅   | icons/ 五件套 + tauri.conf.json:29-35                                    |
| FIX012   | .2 绿色单 exe                 | ✅   | main.rs:7 GUI 子系统                                                     |
| FIX012   | .3 首启默认四项               | ⚠️   | Rust 四项 diff 级全过；**前端三处默认值 + settings 字段注释漏改 → P2-1** |
| FIX012   | .4 开板末帧 pop 根治          | ✅   | archive.css:49 / detail.css:42 `will-change`                             |
| FIX012   | .5 双击标题缩回托盘           | ✅   | tray_menu.rs:73-83 + lib.rs:258 + App.vue 全链（反馈缺口 → P3-6）        |
| FIX012   | .6 tooltip 补齐               | ✅   | 同 FIX011.8                                                              |
| FIX012   | .7 release/ 使用说明          | ✅   | 在库（PATCH001 已补归档语义）                                            |
| FIX012   | .8 commit + tag               | ✅   | tag v0.2.0（与 ui1.0-final 并存）                                        |
| FIX012   | .9 zip 规则与工件             | ✅   | .gitignore:21-24 + release/ 工件                                         |
| FIX012   | .10 GitHub Release            | ✅   | gh 实证 Latest v0.2.0                                                    |
| FIX012   | .11 发布收尾自查              | ✅   | 后续排期在 z.plan 待完成区                                               |
| CI       | ci.yml 入库                   | ✅   | .github/workflows/ci.yml                                                 |
| CI       | .2 构建顺序修正               | ✅   | job 步骤序：前端构建先于 cargo 组                                        |
| CI       | .3 收口步骤移除               | ✅   | `test -f` 零命中                                                         |
| CI       | README CI 徽章                | ✅   | README.md:5                                                              |
| CI       | 云端复证                      | ✅   | run 37149067675 七步逐项 ✓（rust-cache 热缓存 1m31s 非跳步）             |
| PATCH001 | .1 常量+红灯测试              | ✅   | todo.rs:22 + 四用例（139 绿）                                            |
| PATCH001 | .2 存储层回收方法             | ✅   | storage.rs:345-352（两路最严审零缺陷）                                   |
| PATCH001 | .3 归档读路径挂接             | ✅   | commands/todo.rs:164-172                                                 |
| PATCH001 | .4 版本与文档连带             | ⚠️   | 文档五连在位；**README 版本徽章连带漏 → P3-3**                           |
| PATCH001 | .5 收尾验证                   | ✅   | 门禁全绿 + CI 复证 + 端到端隔离实测                                      |

**零节总结论**：四组 46 项逐项核验，44 ✅ / 2 ⚠️（均收编 P 级，无行为级回退）。

### 一、P0-P3 修复清单（无 P0/P1；P2 ×3 + P3 ×8）

| #    | 文件:行号                                                                                               | 类型    | 描述与建议                                                                                                                                                                                                                                                                                                                                                                                                                              | 性质                         | 影响面            |
| ---- | ------------------------------------------------------------------------------------------------------- | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | ----------------- |
| P2-1 | ui/App.vue:190-191、:202 + TrayMenu.vue:16-18 + mock-invoke.ts:58-59 + settings.rs:78、:81（连带 ：63） | 12/11/6 | **FIX012.3 首启默认定案连带漏改面**：Rust 侧 V0.2.0.0 起置顶/吸附缺省关（settings.rs:40-48），前端三处默认仍 `ref(true)`（App.vue 设置板回落 ref 含 ：202 注释 / TrayMenu 常驻菜单窗初始勾选 / mock 内存态）——降级态显示漂移 + **IAB 冒烟以 mock 为事实源锁定与真机相反形态**；settings.rs 两字段 doc 仍写"回填 true"与行为方向相反、:63 struct 职责清单缺两开关。修法：三处齐改 false + 注释同步 + 字段 doc 改"回填 false" + 63 行补齐 | 新增（FIX012 修复不完整）    | 跨模块            |
| P2-2 | SettingsOverlay.vue:94 + core/src/settings.rs（无 theme 字段）                                          | 12      | **主题手动档（浅/暗）不持久化**：设置板六项唯主题三态每次启动重置回跟随系统；z.plan 仅定案"种子 0"未定案"不持久化"，"该进配置未进"不可豁免。修法：WindowSettings 增 theme 字段（serde default 0）+ settings_set 命令 + 前端启动回读                                                                                                                                                                                                     | 遗留（存量暴露）             | 配置体系+Vue 前端 |
| P2-3 | TodoList.vue:188-192、:226-231                                                                          | 1       | **跨条目切换 pendingExitId 残留 → 新行编辑器 ≤1.5s 被强拆、草稿丢失**（需验证）：编辑 A 提交在飞 → 双击行 B（1665b52 守卫只拦同条目）→ rename(A) resolve 置 pendingExitId=A → watch 不清（editDraft 已是 B 文本）→ 1500ms 兜底无条件清 editingId → B 输入框强制卸载（不派发 blur，草稿丢失）。修法：startInlineEdit 先清 pendingExitId + 掐兜底句柄                                                                                     | 遗留（存量暴露）             | Vue 前端          |
| P3-1 | TodoList.vue:170-181                                                                                    | 1/4     | **onRowDblClick 缺 rowMaskDead/isSuppressed 前置**：罩死行双击仍进编辑、拖拽收场 350ms 内双击重启编辑（对照组 BubblesView.vue:274-275 两查俱全，单击侧齐全，仅双击漏）。修法：对齐补两查                                                                                                                                                                                                                                                | 遗留                         | Vue 前端          |
| P3-2 | composables/titleParticles.ts:306-313（:20 注释）+ useThresholdDrag.ts:50-53                            | 2/6     | **onUnmounted 注册于异步上下文清理恒失效**（FIX005.26 同款铁律复发）：App.vue onMounted await 链后调用，监听清理从未注册；titleParticles.ts:20 注释与行为不符。实际影响≈0（App 无卸载路径）故 P3。修法：getCurrentInstance 守卫 + 显式 destroy（对齐 useGlassBar）                                                                                                                                                                      | 遗留                         | Vue 前端          |
| P3-3 | README.md:6、:33、:118                                                                                  | 6       | **README 版本口径落后**：徽章 Version-0.2.0 与双语 zip 链接 v0.2.0，Cargo.toml 已 0.2.1。需定口径（徽章随 Cargo 即时推进或随 release 更新）并写入 AGENTS 版本双轨条款防重复报告                                                                                                                                                                                                                                                         | 新增（PATCH001 bump 连带漏） | 文档              |
| P3-4 | AGENTS.md:54、:46                                                                                       | 6       | **目录树两处与实态漂移**：bubble.rs 行仍写"满 5 阈值"（FIX004.23 已删 Rust 侧阈值）；tests/ 漏列 sort_order_migration.rs                                                                                                                                                                                                                                                                                                                | 新增（存量暴露）             | 文档              |
| P3-5 | tray.rs:466-475、:496-505                                                                               | 4       | **同文件 FFI 重复声明**：`#[repr(C)] Point` + `GetCursorPos` extern 两处同构。修法：提文件级单点                                                                                                                                                                                                                                                                                                                                        | 遗留                         | Tauri 后端        |
| P3-6 | commands/tray_menu.rs:74-83                                                                             | 11/13   | **main_hide_to_tray close 失败仅 eprintln 仍返回 Ok**——双击标题无用户可见反馈；贴近豁免 P12 但主体是标题双击非菜单项，豁免归属归档定案（扩 P12 表述或登记新条目或改严格）                                                                                                                                                                                                                                                               | 新增（FIX012.5 引入）        | Tauri 后端        |
| P3-7 | commands/todo.rs:15-19                                                                                  | 13      | **todo-changed 广播失败仅落日志——容错白名单未登记**（同族 FIX008.12 已登记，此条自 PL018.6 在位无登记）。修法：白名单补三要素                                                                                                                                                                                                                                                                                                           | 遗留                         | Tauri 后端        |
| P3-8 | AGENTS.md 白名单 PL015.5 条                                                                             | 13      | **登记面未覆盖 FIX010.6 变体**："并发接管放弃回滚"降级未入白名单文（代码注释已详述，仅登记缺）。修法：PL015.5 扩围                                                                                                                                                                                                                                                                                                                      | 遗留                         | 文档              |

> 豁免过滤记录：Tauri 组"Idle 拍每 100ms 主线程往返"命中 **P22**、"多屏负 y 副屏 py<0 翻转失准"命中 **P35**——均永久豁免不报。

### 二、参考级观察项（记录不修，含回落理由）

| 文件:行号                                            | 描述                                                                               | 回落理由                                 |
| ---------------------------------------------------- | ---------------------------------------------------------------------------------- | ---------------------------------------- |
| todo.rs:91                                           | `now - created` 理论 i64 溢出（手改 db 极端值）                                    | 无可达正常路径，纯理论【需验证】         |
| storage.rs:320-352                                   | list/list_done/purge 无索引全表扫                                                  | 零线程定案设计依据；单用户量级实测无感   |
| commands/todo.rs:164-172                             | 读命令携破坏性副作用（purge）命名无暗示                                            | 函数头文档注释已载明定案形态，仅口径记录 |
| commands/todo.rs + App.vue:152                       | purge 后归档详情板 ghost 条目面（删除在打开后，操作得可见错误）                    | 自愈可见非静默错值，触发链极窄【需验证】 |
| lib.rs:267-309                                       | setup 两次取设置锁可复用                                                           | 极小卫生面                               |
| TrayPreview.vue:38、:51                              | `?? 164` / `slice(0,5)` 散装魔法数                                                 | 预览窗私有展示参数自洽                   |
| ArchiveOverlay.vue:153-156 + DetailOverlay.vue:82-86 | 450ms settle 双写                                                                  | 与 CSS 揭示过渡硬耦合                    |
| mock-invoke.ts:129-134、:185-191                     | validateText 硬编码 24；`todo_set_note` 缺 10_000 笔记校验（IAB 测超长笔记假通过） | dev-only，随下批 mock 对齐顺带           |
| main.ts:44-47                                        | `catch { return "main" }` 静默回落                                                 | 纯浏览器 DEV 分支不可达                  |
| archive.css:49 / detail.css:42                       | 三浮板常驻 `will-change` 合成层                                                    | 四变体实验定案注释在案                   |
| TodoList.vue:286-297                                 | `watch(items, {deep:true})` 每刷新深遍历                                           | pendingExit 判定所需，N 小               |
| useDragReorder.ts:121-124、:147-150                  | DOM 快照 Number() 无 NaN 防御                                                      | 行模板恒带 data-row-id，异常被 serde 拒  |

### 三、亮点

PATCH001 新增面两路独立最严审零缺陷（谓词/边界/分层/锁纪律/TDD 三案例）；FIX010 十四项 diff 级零回退；死锁铁律全组锁点零违规；FFI 签名全组核对无误；SQL 全参数化；types.ts ↔ serde 逐字段零漂移；28 invoke 命令名与 generate_handler 一一匹配；capabilities 匹配；依赖零未用；CSP 落地核验通过。**收官态势：连续四轮无 P0/P1，实现面（SQL/状态机/FFI/退出链）维持零新缺陷。**

---

## 附录 PL023：清单笔记红点指示（2026-10-09 立项，[future-plan-001#功能二]）

> 背景：todo 详情本体 PL010 已落全链（todos.note 列 + DetailOverlay 详情板 + todo_set_note 防抖保存 + types/mock 镜像），但清单行上看不出哪条写过笔记。用户提出"有详情内容的条目显示红点，方便查看"——本 PL 纯做视觉指示：**零 schema 改动、零新命令、零新事件处理器**（future-plan-001 讨论稿的体量预估据此大幅收窄）。
> 方案要点（2026-10-09 用户定案）：
>
> - **裁决在 Rust**：TodoItem 加派生字段 `has_note: bool`，存储层 row_to_item 单点算 `!note.trim().is_empty()`——对齐 age_level"阈值裁决不漂移"纪律（A001 dim 12），"什么算有内容"的语义单源不散落前端；get/list/list_done 共用 row_to_item，toggle/rename 返回 get(id) 自动覆盖；add() 字面量补 has_note: false
> - **红点 = 纯指示器**：t-text 内部 inline 圆点（`v-if="item.has_note"`），点击天然冒泡行单击 → 既有开详情板路径（180ms 防双击拍子 / 拖拽落点抑制 / 罩死行禁交互，全套既有守卫自动生效），前端零新事件
> - **挂点定案**：初版文本尾内联 6px → **用户目验微调（2026-10-09）钉玻璃条左上角 4px 内缩、放大至 9px**——条内贴角为硬约束（todo-row `overflow: clip` 裁出界元素，红点不得骑角）；纵向对齐成列扫读更整齐；罩死态随删除钮同族淡出（opacity 0.35）。色用 `--danger` 与龄期红字同族
> - **语义边界**："有内容" = trim 后非空——纯空白笔记不算（validate_note 允许空白入库，红点不亮），与清空笔记红点灭同一条规则零特判
> - **范围收敛**：归档板不加红点（用户定案；TodoItem 载荷带字段，日后启用零成本）
> - **版本**：feat → Cargo.toml 0.2.1 → 0.2.2，提交 feat: V0.2.2.1
>
> 状态：✅ 已完成（2026-10-09，V0.2.2.1；实施记录见 x.progress.md PL023 组——IAB 三断言：种子 6 行红点全亮 / 点红点详情板开 / 清空笔记红点 6→5）

---

## 附录 PL024：图片气泡——剪贴板图片捕获与复制回（2026-10-09 立项，[future-plan-001#功能一]）

> 背景：气泡一期只捕获文本（tauri 剪贴板插件 read_text/write_text 单一能力），用户截图后的图无法上板。本 PL 让两条捕获入口（热键圈选 / 气泡页手动按钮）自动识别剪贴板图片存为图片气泡，双击把图写回剪贴板——捕获→粘贴走→清理闭环对图片成立。技术核心 = Win32 剪贴板多格式读写 FFI 直连（tauri 插件无图片能力，项目 hotkey/capture/tray 零依赖直连同款）。
> 方案要点（2026-10-09 用户定案）：
>
> - **入口与行为变更**：热键按下**前置检测剪贴板已有图**——有图直接捕获成图片气泡（**不合成 Ctrl+C**，现状下截图后按热键会白合成+等文本 300ms 超时静默，图被无视——本 PL 根本性前提）；无图走现圈选流程，合成后读取分叉（圈选的是图则捕获图）；图优先于文本（截图工具常同放图+文本两格式）；气泡页手动按钮同一读取函数同规分叉
> - **占位文案**：`🖼 截图 261009143205`（yyMMddHHmmss 本地时间连写，GetLocalTime FFI 取本地时区零依赖；自动命名不可改名一期）
> - **数据模型**：bubbles 幂等补两列 `kind TEXT NOT NULL DEFAULT 'text'` + `image BLOB`（column_set 探列沿 PL010/PL013 先例）；BubbleItem 只加 kind（serde 枚举 "Text"/"Image" 沿 AgeLevel 先例），**列表载荷不带图**（防多图拖爆列表拉取），图片走新命令 bubble_get_image(id) 按需回 base64 data URL（详情板打开时一次）；图片气泡**跳过去重**（每次截图都是新内容）；单图 ≤4MB（BubbleError::ImageTooLong）+ PNG 魔数校验（UnsupportedFormat）防垃圾字节
> - **读取编码**：PNG 注册格式（RegisterClipboardFormatW("PNG")）主路径 = 截图工具原厂字节零转换；CF_DIB 回退 = 解析 BITMAPINFOHEADER 支持 24/32bpp BI_RGB + BI_BITFIELDS（行序翻转 + BGR(A)→RGBA）后编码 PNG——编码引 **image crate（只开 png feature，全项目首个新依赖，用户随讨论定案）**；罕见变体（16bpp/调色板）明确报"图片格式暂不支持"（严格抛错主线非降级）
> - **复制回（用户定案必须有——否则功能失去意义）**：bubble_copy 按 kind 分叉；写入集 = **PNG 注册格式 + CF_DIB + CF_DIBV5 三格式**（= Win+Shift+S 原厂写入集，像素同源仅头结构差异，alpha 掩码全链无损）；CF_BITMAP 由剪贴板自动合成不手写；透明灰区知情（alpha-忽视老应用读 CF_DIB 黑底属其局限，主场景截图全不透明无涉）；非 Windows 明确报"当前平台暂不支持"（延后基线）
> - **错误策略**：图片读取失败热键路径静默落日志（反馈静默定案不变）、手动路径可见报错——与文本路径完全对齐；无新增容错白名单
> - **补定两件（2026-10-09 目验反馈，PL024.8）**：①捕获错误行 1s 自动隐藏 + 0.3s 渐显渐隐 + age-alert 同款文字阴影 + 显示层截断第二个全角冒号后的系统英文（Rust 底账完整）②CF_HDROP 图像文件分支——读取链三级化 PNG→CF_DIB→HDROP（单图像文件扩展名判定→读文件→非 PNG 解码转 PNG），覆盖"选中/复制图片文件 + 热键"场景，image crate features 扩 jpeg/gif/webp/bmp
> - **补定一件（2026-10-09 目验反馈；2026-10-10 追加 boardRead 跟随定案，PL024.8c）**：图片详情板滑杆与整板阅读规范——图片自适应宽零横向滑杆（`overflow-x: hidden`）；图片模式整板阅读套件与**文字块完全一致**（useGlassBar 与 useBoardRead 双挂；滑杆锚 `.detail-overlay`、`right: 4` 居中于内容区右缘与板右缘之间；▲▼三角 + 溶解带同规）；图片异步渲染的挂载时机 + 模式化挂载/销毁 + useVeils 帘豁免扩围（防复现"详板自家滑杆被罩"原 bug）
>
> 状态：🚧 进行中（2026-10-09——.1–.7 代码面完成[164 测试绿 + 八项门禁绿 + IAB 三断言过]，.8a/.8b 目验反馈补定待实施；live 真机三断言待用户执行，AI 不可代按剪贴板交互）

---

## 附录 PL025：图片气泡载荷落盘化（图片文件存程序目录 + asset 协议直读 + 原格式保留 + 复制回镜像来源）（2026-10-10 立项）

> 背景：PL024 把图片字节整存 DB（bubbles.image BLOB）——4MB 上限挡大图、DB 膨胀、读写搬字节、文件图被强转 PNG（有损/变大/丢动画）。本 PL 把图片载荷从 DB 迁出：一律落盘到程序目录 `data/images/`，DB 只存文件名；asset 协议直读；文件图保留原格式；复制回按来源镜像。
> 方案要点（2026-10-10 用户定案）：
>
> - **统一落盘**：截图与文件图都写 `data/images/`（dev=项目根 / release=exe 同级，沿双落址）；DB 只存相对文件名 `image_file`；`bubbles.image` 列迁移后 **DROP**（先迁移、后删列，幂等）
> - **原格式保留**：文件图原字节直写、保留原扩展名（jpg/gif/webp/bmp/png，**不转 PNG**——无有损二次编码、更小、保动画）；截图 PNG 直写（仅 CF_DIB 来源的截图编码一次 PNG）
> - **asset 协议直读**：图片目录 = 程序目录；启动运行时 `app.asset_protocol_scope().allow_directory(images_dir, true)` 注册（API 实测存在于 tauri 2.11.5/2.12.0）；`tauri.conf.json` 开 `assetProtocol` + CSP `img-src` 加 `asset: http://asset.localhost`；详情**不再回 base64**
> - **命名**：文件图 `🖼️ 图片 <文件名>`；截图 `🖼️ 截图 {yyMMddHHmmss}`（emoji `🖼`→`🖼️`）
> - **去重**：内容哈希（FNV-1a 64 手写零依赖，按**原字节**算）
> - **复制回 = 镜像来源**（用户定案）：文件图 → `CF_HDROP`（把应用内副本文件放剪贴板，**原样零解码**，复制的是文件）；截图 → `PNG + CF_DIB + CF_DIBV5`（PNG 字节直写；DIB/DIBV5 解码像素，复制的是可粘贴的图）
> - **失效兜底**：`image_file` 存在但文件缺失 → 详情板「文件已移动或删除，请删除气泡」+ 删除按钮
> - **数据模型**：bubbles 幂等补三列 `image_file TEXT` / `image_hash TEXT` / `image_source TEXT`（`'screenshot' | 'file'`，复制回分支判据；存量默认 `'screenshot'`）；文件名 `{id}.{ext}`
> - **错误策略**：迁移幂等、失败严格报错（不静默丢图）；删气泡删文件 best-effort；文件缺失走失效态不崩不静默；无新增容错白名单
> - **边界**：非 Windows asset 协议可用但文件图来源（CF_HDROP）仅 Windows；超大图存储层无上限但 WebView 仍要解码整图（缩略图/降采样记为后续项）；动图文件图保留原文件不丢
> - **修订（2026-10-10 目验反馈，用户定案；执行条目 `.b`）**：①落盘改**子文件夹** `data/images/bubble_{yyMMddHHmmss}_{n}/{原名}.{ext}`（气泡专用前缀，留白板空间；截图内层 `snap_{yyMMddHHmmss}_{n}.png`；`n` 同秒从 1 递增）——复制回 `CF_HDROP` 指向内层原名文件 → **粘出原名**（零改名/零临时文件）；删气泡**递归删文件夹**；②**大图降采样预览**（宽 > 2000px 或 > 2MB → 捕获时生成 `{folder}/preview.png`，长边 ≤ 1600；详情读预览、复制回/查看用原图；失败降级不阻断）；③**双击详情图开原图**（`ShellExecuteW` 零依赖，仅 Windows）；④**占字文案判别**（单占字态：捕获成功 `已捕获` / 双击复制 `已复制` / 重复 `重复捕获，无效！`）。**定案不做**：捕获按钮不改逻辑（焦点所限，按钮收不了他应用选区）；虚拟文件剪贴板格式（FileGroupDescriptor + FileContents）不实施；复制回后判重已由内容哈希覆盖。编号约定：初版已实现为 `PL025.Na`，本次修订为 `PL025.Nb`（**不改写 .a**）。
> - **补修（2026-10-10 目验反馈，PL025.8）**：①失效块溢出（`.detail-image-broken` 补 `box-sizing: border-box`）②详情删除后气泡列表不刷新（`bubble_remove`/`bubble_clear` 发 `bubble-changed`）③失效文案加粗 + 阴影（字号维持 12px）④失效删除按钮对齐「一键清空」样式 + 按下即时视觉（不做二态确认）
>
> 状态：🚧 实施中（2026-10-10——初版 `PL025.Na` 已落并门禁绿；目验反馈修订 `PL025.Nb` 已落；补修 `PL025.8` 已落；执行拆条见 x.progress.md PL025 组）
