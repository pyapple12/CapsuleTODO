// ===== DEV 冒烟基座（PL008.1）：纯浏览器环境的 invoke 内存模拟 =====
// 背景：真 invoke 依赖 Tauri IPC（window.__TAURI_INTERNALS__），纯浏览器不可用——
// vite dev + IAB 是 APP 回归期（PL008–PL014）的自动化验证通道，本模块在 DEV 且
// 非 Tauri 环境时注入假 internals，把 invoke 路由到内存态模拟。
// 红线：import.meta.env.DEV 死分支，生产构建被 vite 静态消除（dist 不得含本模块痕迹）。
// 内存种子沿 design/assets/js/state.js 实验场定案（8 清单含归档 + 5 气泡 + 长文白板），
// 使冒烟断言可与实验场形态互相印证。

/** 待办条目（镜像 ui/types.ts TodoItem：PL010 三字段扩容后契约对齐） */
interface MockTodo {
  id: number;
  text: string;
  done: boolean;
  created_at: number | null;
  done_at: number | null;
  note: string;
  /** 拖拽排序键（PL013；种子行未赋值回退 id 序） */
  sort_order?: number;
}

/** 龄期档位（镜像 AgeLevel） */
type MockAgeLevel = "None" | "Yellow" | "Red";

/** 龄期阈值（与 Rust todo.rs AGE_YELLOW_MS/AGE_RED_MS 同源） */
const AGE_YELLOW_MS = 24 * 3600 * 1000;
const AGE_RED_MS = 48 * 3600 * 1000;

/** 龄期裁决（对齐 Rust age_level：严格大于才升级） */
function ageLevel(created_at: number | null, now: number): MockAgeLevel {
  if (created_at == null) return "None";
  const age = now - created_at;
  if (age > AGE_RED_MS) return "Red";
  if (age > AGE_YELLOW_MS) return "Yellow";
  return "None";
}

/** 清单视图条目（镜像 TodoView） */
type MockTodoView = MockTodo & { age_level: MockAgeLevel };

/** 气泡条目（镜像 BubbleItem） */
interface MockBubble {
  id: number;
  text: string;
  /** 拖拽排序键（PL013；种子行未赋值回退 id 序） */
  sort_order?: number;
}

/** 气泡页快照（镜像 BubbleSnapshot；满额提醒显隐由前端本地阈值裁决，FIX004.23） */
interface MockBubbleSnapshot {
  items: MockBubble[];
}

/** 气泡提醒阈值（PL014 接配置前的硬编码对齐值） */
let MAX_BUBBLES = 5;
let MOCK_ALWAYS_ON_TOP = true; // PL017 设置板置顶开关 mock 态
let MOCK_SNAP_TO_EDGE = true; // PL017 设置板贴边吸附开关 mock 态
let MOCK_BUBBLE_HOTKEY = "Ctrl+Alt+C"; // PL015 mock 内存态；busy 标志模拟占用失败
let mockBubbleHotkeyBusy = false;

const HOUR = 3600 * 1000;
const WEEK_NOTE = `本周完成事项：
一、玻璃材质迁移。卡片、页签、输入条三处背景从纯色叠加切换为 backdrop-filter 实时采样，实测 GPU 占用峰值从 12% 降到 7%，滚动帧率稳定在 60fps。踩坑记录：嵌套采样边界——外层 backdrop 会把内层 backdrop 的输出当作背景再模糊一次，双层叠加区域整体发糊，最终采用"内层只描边、不做磨砂"的方案绕过，视觉层次反而比原设计更好。
二、勾选组件本地化。uiverse 霓虹勾选框的原始实现依赖 fixed 定位的粒子层，在卡片 overflow 裁剪下粒子整体丢失；改为随行内绝对定位后，粒子抛洒半径从 24px 压缩到 14px，配合 keyframes 全程 transform 合成，动画期间零布局重排，快速连点也不掉帧。
三、数据层联调。todos 表新增 created_at 字段并建索引，查询按未完成优先排序；过期提醒的阈值判断保留在渲染层完成，避免每条查询携带时间条件，列表拉取保持全量。
四、气泡捕获链路。剪贴板文本捕获从轮询改为事件驱动，捕获延迟从平均 800ms 降到 60ms 以内；气泡去重采用文本指纹前 32 位哈希对比，重复捕获直接置顶原气泡并计数加一。
遇到的问题：一、Windows 下 DWM 材质切换在窗口最小化恢复的瞬间闪烁一次，怀疑是 backdrop 类型写入与合成器帧没有对齐，暂以延迟一帧写入规避，等真机验证。二、拖拽排序的让位动画在快速甩动时追不上指针，计划引入滞后区间，只有越过目标槽位中线 4px 才提交换位。三、输入法组合输入期间 keydown 会误触发清空，需要改用 compositionend 提交。
下周计划：一、详情编辑面板接入撤销栈，支持三十步回退；二、收敛 glass.css 的阴影令牌为三档规格；三、补齐触摸屏长按拖拽，为平板形态留口子；四、把 reduced-motion 的全链路过一遍；五、给清单行加键盘可达性，Tab 聚焦加空格勾选。
心得：这周最大的收获是吃透了"动效是覆盖层、重绘是事实源"这句话——所有位移都建立在数据一次性提交的基础上，顺序一旦颠倒，动画就成了修不完的补丁。第二个体会是占位符的价值：它让"拖走"与"放回"共用同一套布局数学，代码量直接省了一半。第三点是测试先行在动效上也成立：先写好断言的最终布局，动画随便怎么调，收不住布局就过不了关。`;

/** 内存态（模块级单例：跨组件共享，等价 Rust 侧 db 单一事实源） */

const state = {
  todoSeq: 0,
  bubbleSeq: 0,
  todos: [] as MockTodo[],
  bubbles: [] as MockBubble[],
  whiteboard: "",
  lastCopied: "",
};

/** 种子装载（装载时快照 Date.now，断言可复现） */
function seed(): void {
  const now = Date.now();
  const t = (text: string, done: boolean, ageHours = 0, note = ""): MockTodo => ({
    id: ++state.todoSeq,
    text,
    done,
    created_at: now - ageHours * HOUR,
    done_at: done ? now : null,
    note,
  });
  state.todos = [
    t("买牛奶", false, 0, "2L 全脂一盒\n鸡蛋一排\n顺路取快递"),
    t("写周报", false, 30, WEEK_NOTE),
    t(
      "读一章 Rust 书",
      false,
      50,
      "第 4 章：所有权与借用。重点标注移动语义、借用检查器报错的三种典型场景，习题留到周末。",
    ),
    t("给 CapsuleTODO 画个图标", true, 0, "三版草图：玻璃质感、描边、纯色剪影，定了玻璃质感方向。"),
    t("定稿玻璃令牌三档阴影", true, 50, "贴边/悬浮/环境三档，落影与棱光分离。"),
    t("修复详情板收板闪烁", true, 72, "缩回曲线改平缓，越 0 镜像闪动已消除。"),
    t("归档板交互原型评审", true, 96, "评审结论：勾退/删除两语义够用，批量缓议。"),
    t("预约牙医（洗牙）", false, 0, "工作日晚上有号，周五前订好。"),
    t("交房租", false, 0, "每月 5 号前，银行卡留够余额。"),
    t("回复设计评审邮件", false, 0, "重点回复动效时长那两条意见，语气放软。"),
  ];
  const b = (text: string): MockBubble => ({ id: ++state.bubbleSeq, text });
  state.bubbles = [
    b("示例片段：把设计实验场的配方读数抄回 glass.css"),
    b("示例片段：uiverse 动效候选 R001-5 成功对勾曲线"),
    b(
      "中长气泡：玻璃质感是三层叠加的产物——半透明底色给体积，backdrop-filter 给实时模糊，描边与落影给边界，单独任何一层都不构成玻璃，叠在一起才是玻璃。这一条在行里超过两行，只露两行并以省略号收尾。",
    ),
    b("复制的一段话：玻璃的质感来自克制，而不是效果的堆叠"),
    b(
      "超长气泡：玻璃质感是三层叠加的产物——半透明底色给体积，backdrop-filter 给实时模糊，描边与落影给边界。模糊半径不追大，8 到 20px 足够柔化背景又不把层次糊成一团；着色永远浅，21% 的 accent 淡染只负责提示身份；边界必须交代，1px 内描边加一道向下落影，玻璃才有厚度而不是一块滤镜。这一条故意写得足够长——行内只露两行加省略号，开板后内容超过玻璃板高度，板内滑杆出现，按住拖拽即滚动全文。临时剪贴板的定位本来就是十五秒记忆：捕获的成本必须远低于打开备忘录，看完即弃；溢出提醒只是温和地催你清台面。滚动在手写实现里比看起来简单——scrollHeight 减 clientHeight 得到总行程，scrollTop 除以它就是比例，比例乘轨道高就是浮钮位置；难的是手感，拖拽要跟手、松手要即停、边界不弹跳。",
    ),
  ];
  state.whiteboard = "白板草稿：玻璃质感是三层叠加的产物……（长文种子，验证板内滑杆与防抖保存）";
}

/** 文本校验（对齐 Rust TodoError 语义：trim 后非空、≤100 字符） */
function validateText(text: string): string | null {
  const trimmed = text.trim();
  if (!trimmed) return "待办文本不能为空";
  if (trimmed.length > 100) return "待办文本过长";
  return null;
}

/** 排序视图（PL013 起对齐 Rust：未完成按 sort_order、已完成按 sort_order 殿后；
 * sort_order 缺省（未迁移种子行）回退 id 序） */
function sortedTodos(): MockTodo[] {
  return [...state.todos].sort((a, b) => {
    if (a.done !== b.done) return a.done ? 1 : -1;
    const sa = a.sort_order ?? a.id;
    const sb = b.sort_order ?? b.id;
    return sa - sb || a.id - b.id;
  });
}

/** 命令路由表：键 = Tauri 命令名，值 = args → 结果（reject = CommandError 透传） */
type CommandHandler = (args: Record<string, unknown>) => unknown;
const handlers: Record<string, CommandHandler> = {
  todo_list: (): MockTodoView[] =>
    sortedTodos().map((item) => ({ ...item, age_level: ageLevel(item.created_at, Date.now()) })),
  todo_add: (args) => {
    const text = String(args.text ?? "");
    const err = validateText(text);
    if (err) throw err;
    const item: MockTodo = {
      id: ++state.todoSeq,
      text: text.trim(),
      done: false,
      created_at: Date.now(),
      done_at: null,
      note: "",
    };
    state.todos.push(item);
    return item;
  },
  todo_toggle: (args) => {
    const id = Number(args.id);
    const item = state.todos.find((x) => x.id === id);
    if (!item) throw `待办条目不存在：${id}`;
    item.done = !item.done;
    item.done_at = item.done ? Date.now() : null; // 勾选置值、退回清空（对齐 Rust toggle）
    return item;
  },
  todo_rename: (args) => {
    const id = Number(args.id);
    const text = String(args.text ?? "");
    const err = validateText(text);
    if (err) throw err;
    const item = state.todos.find((x) => x.id === id);
    if (!item) throw `待办条目不存在：${id}`;
    item.text = text.trim();
    return item;
  },
  todo_set_note: (args) => {
    const id = Number(args.id);
    const item = state.todos.find((x) => x.id === id);
    if (!item) throw `待办条目不存在：${id}`;
    item.note = String(args.note ?? "");
    return null;
  },
  todo_archive_list: () =>
    state.todos
      .filter((t) => t.done)
      .sort((a, b) => (b.done_at ?? 0) - (a.done_at ?? 0) || b.id - a.id),
  todo_remove: (args) => {
    const id = Number(args.id);
    const idx = state.todos.findIndex((x) => x.id === id);
    if (idx < 0) throw `待办条目不存在：${id}`;
    state.todos.splice(idx, 1);
    return null;
  },
  // 重排（PL013）：ids = 未完成条目目标全量序——校验语义对齐 Rust（长度/幽灵/重复）
  todo_reorder: (args) => {
    const ids = (args.ids as unknown[]).map(Number);
    const actives = state.todos.filter((t) => !t.done);
    const uniq = new Set(ids);
    if (ids.length !== uniq.size || ids.length !== actives.length) {
      throw "重排 id 集合不合法：与未完成集不一致";
    }
    const set = new Set(actives.map((t) => t.id));
    for (const id of ids) {
      if (!set.has(id)) throw `重排 id 集合不合法：幽灵 id ${id}`;
    }
    ids.forEach((id, order) => {
      const item = state.todos.find((t) => t.id === id);
      if (item) item.sort_order = order;
    });
    return null;
  },
  bubble_list: (): MockBubbleSnapshot => ({
    // 展示 = sort_order 升序（PL013 拖拽序；缺省回退 id 序）——对齐 Rust list_bubbles
    items: [...state.bubbles].sort(
      (a, b) => (a.sort_order ?? a.id) - (b.sort_order ?? b.id) || a.id - b.id,
    ),
  }),
  // 捕获（Rust = 读真剪贴板；mock 环境无剪贴板，返回模拟文本走完整校验+去重链路，
  // FIX004.5：返回 BubbleCaptureOutcome 形状与真机同构——重复返回 duplicate 不入库）。
  // sort_order = 现存最小值 − 1（排头插入，对齐 Rust add_bubble 的 unshift 语义）
  bubble_capture: () => {
    const text = `[mock 捕获] ${new Date().toLocaleTimeString()} 的剪贴板内容`;
    const err = text.trim() ? null : "气泡文本不能为空";
    if (err) throw err;
    if (state.bubbles.some((b) => b.text === text)) {
      return { status: "duplicate" };
    }
    const min = Math.min(0, ...state.bubbles.map((b) => b.sort_order ?? b.id));
    const item: MockBubble = { id: ++state.bubbleSeq, text, sort_order: min - 1 };
    state.bubbles.push(item);
    return { status: "added", item };
  },
  // 复制回（Rust = 写真剪贴板；mock 环境写不进去，记录最近复制内容供断言）
  bubble_copy: (args) => {
    const id = Number(args.id);
    const item = state.bubbles.find((x) => x.id === id);
    if (!item) throw `气泡条目不存在：${id}`;
    state.lastCopied = item.text;
    return null;
  },
  bubble_add: (args) => {
    const text = String(args.text ?? "").trim();
    if (!text) throw "气泡文本不能为空";
    if (text.length > 2000) throw "气泡内容过长（上限 2000 字符）";
    const min = Math.min(0, ...state.bubbles.map((b) => b.sort_order ?? b.id));
    const item: MockBubble = { id: ++state.bubbleSeq, text, sort_order: min - 1 };
    state.bubbles.push(item);
    return item;
  },
  bubble_remove: (args) => {
    const id = Number(args.id);
    const idx = state.bubbles.findIndex((x) => x.id === id);
    if (idx < 0) throw `气泡条目不存在：${id}`;
    state.bubbles.splice(idx, 1);
    return null;
  },
  bubble_clear: () => {
    const removed = state.bubbles.length;
    state.bubbles = [];
    return removed;
  },
  // 重排（PL013）：ids = 全量气泡目标序——校验语义对齐 Rust
  bubble_reorder: (args) => {
    const ids = (args.ids as unknown[]).map(Number);
    const uniq = new Set(ids);
    if (ids.length !== uniq.size || ids.length !== state.bubbles.length) {
      throw "重排 id 集合不合法：与气泡集不一致";
    }
    const set = new Set(state.bubbles.map((b) => b.id));
    for (const id of ids) {
      if (!set.has(id)) throw `重排 id 集合不合法：幽灵 id ${id}`;
    }
    ids.forEach((id, order) => {
      const item = state.bubbles.find((b) => b.id === id);
      if (item) item.sort_order = order;
    });
    return null;
  },
  // 设置（PL014.2）：读取/写入气泡提醒上限（内存态，设置板步进回显用）
  settings_get_max_bubbles: () => MAX_BUBBLES,
  settings_set_max_bubbles: (args) => {
    MAX_BUBBLES = Math.min(20, Math.max(1, Number(args.value) || 5));
    return null;
  },
  // 窗口偏好（PL017）：置顶与贴边吸附开关（内存态，设置板 toggle 断言用）
  settings_get_always_on_top: () => MOCK_ALWAYS_ON_TOP,
  settings_set_always_on_top: (args) => {
    MOCK_ALWAYS_ON_TOP = Boolean(args.on);
    return null;
  },
  settings_get_snap_to_edge: () => MOCK_SNAP_TO_EDGE,
  settings_set_snap_to_edge: (args) => {
    MOCK_SNAP_TO_EDGE = Boolean(args.on);
    return null;
  },
  // 气泡热键（PL015，FIX004.5）：读取/写入组合键（内存态 + parse 同规校验；
  // mock.bubbleHotkeyBusy = true 可模拟"热键被占用"失败态供 IAB 断言回退）
  settings_get_bubble_hotkey: () => MOCK_BUBBLE_HOTKEY,
  settings_set_bubble_hotkey: (args) => {
    const combo = String(args.combo ?? "");
    const mods = ["CTRL", "ALT", "SHIFT", "WIN"];
    const parts = combo
      .toUpperCase()
      .split("+")
      .map((p) => p.trim());
    const modOk = parts.filter((p) => mods.includes(p)).length >= 1;
    const main = parts.filter((p) => !mods.includes(p));
    const mainOk =
      main.length === 1 && (/^[A-Z0-9]$/.test(main[0]) || /^F([1-9]|1[0-2])$/.test(main[0]));
    if (!modOk || !mainOk) throw `热键组合不合法：${combo}`;
    if (mockBubbleHotkeyBusy) throw "热键注册失败（热键可能被其它程序占用）";
    MOCK_BUBBLE_HOTKEY = parts
      .sort((a, b) => mods.indexOf(b) - mods.indexOf(a) || (a < b ? -1 : 1))
      .map((p) => (p === "CTRL" ? "Ctrl" : p.charAt(0) + p.slice(1).toLowerCase()))
      .join("+");
    return MOCK_BUBBLE_HOTKEY;
  },
  whiteboard_load: () => {
    // FIX004.19 IAB 断言开关：localStorage['mock-wb-fail']='1' 模拟载入失败
    //（dev-only 模块，生产构建被 vite 静态消除）
    if (localStorage.getItem("mock-wb-fail") === "1") {
      throw new Error("mock 模拟：白板载入失败");
    }
    return state.whiteboard;
  },
  whiteboard_save: (args) => {
    state.whiteboard = String(args.content ?? "");
    return null;
  },
};

/** mock invoke 主入口（签名对齐 @tauri-apps/api/core invoke）。
 * plugin:* 命令（event|listen / window|is_focused 等）= Tauri 官方插件 IPC——
 * 一律安全假响应（unlistenId / false），防组件 onMounted 链路被拒中断（实测教训：
 * App.vue 的 await listen 抛错后 refresh 永不执行，页面恒空态） */
async function mockInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (cmd.startsWith("plugin:")) {
    if (cmd.includes("listen") || cmd.includes("register")) return ++callbackSeq as T;
    if (cmd.includes("is_focused")) return false as T;
    return null as T;
  }
  const handler = handlers[cmd];
  if (!handler) throw `未知命令（mock 未实现）：${cmd}`;
  return handler(args ?? {}) as T;
}

/** 是否应启用 mock：DEV 构建 且 非 Tauri 运行时（真 internals 缺席） */
export function shouldMock(): boolean {
  return import.meta.env.DEV && !("__TAURI_INTERNALS__" in window);
}

/** 事件回调注册计数（transformCallback 分配 id 用） */
let callbackSeq = 0;

/** 安装假 internals：invoke 路由内存模拟 + 补全 event/window API 依赖的最小 internals 面——
 * 实测教训：只补 invoke 时 App.vue onMounted 的 `await listen(...)` 因缺 transformCallback
 * 抛 TypeError 中断，后续 refresh 永不执行（页面恒空态）。listen 语义 = 注册即成功、
 * 永不触发（mock 环境无 Rust 事件源）；isFocused = false（纱态默认透明态） */
export function installMockInvoke(): void {
  if (!shouldMock()) return;
  seed();
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
    invoke: mockInvoke,
    transformCallback: (callback: unknown, once?: boolean) => {
      void callback;
      void once;
      return ++callbackSeq;
    },
    unregisterCallback: (id: number) => {
      void id;
    },
    convertFileSrc: (filePath: string, protocol = "asset") => `${protocol}://localhost/${filePath}`,
    // window/event 等 plugin API 走 `plugin:xxx|yyy` invoke 通道——统一给安全假响应，
    // 防组件侧任何 onMounted 链路被拒打断
    metadata: { currentWindow: { label: "mock" }, currentWebview: { label: "mock" } },
  };
  console.info("[mock] invoke 通道已切换内存模拟（DEV-only，生产构建不存在）");
}
