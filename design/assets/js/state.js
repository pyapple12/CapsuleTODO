// ===== 客户端模拟状态（实验场专用：无 Tauri 无持久化，演示捕获 = 示例片段池） =====
let todoSeq = 0;
const HOUR = 3600 * 1000; // 小时毫秒数：超时提醒阈值计算用
// 写周报的模拟详情内容（用户定案：超 1000 字，用于验证编辑区滚动）
const WEEK_NOTE = `本周完成事项：
一、玻璃材质迁移。卡片、页签、输入条三处背景从纯色叠加切换为 backdrop-filter 实时采样，实测 GPU 占用峰值从 12% 降到 7%，滚动帧率稳定在 60fps。踩坑记录：嵌套采样边界——外层 backdrop 会把内层 backdrop 的输出当作背景再模糊一次，双层叠加区域整体发糊，最终采用"内层只描边、不做磨砂"的方案绕过，视觉层次反而比原设计更好。
二、勾选组件本地化。uiverse 霓虹勾选框的原始实现依赖 fixed 定位的粒子层，在卡片 overflow 裁剪下粒子整体丢失；改为随行内绝对定位后，粒子抛洒半径从 24px 压缩到 14px，配合 keyframes 全程 transform 合成，动画期间零布局重排，快速连点也不掉帧。
三、数据层联调。todos 表新增 created_at 字段并建索引，查询按未完成优先排序；过期提醒的阈值判断保留在渲染层完成，避免每条查询携带时间条件，列表拉取保持全量。
四、气泡捕获链路。剪贴板文本捕获从轮询改为事件驱动，捕获延迟从平均 800ms 降到 60ms 以内；气泡去重采用文本指纹前 32 位哈希对比，重复捕获直接置顶原气泡并计数加一。
遇到的问题：一、Windows 下 DWM 材质切换在窗口最小化恢复的瞬间闪烁一次，怀疑是 backdrop 类型写入与合成器帧没有对齐，暂以延迟一帧写入规避，等真机验证。二、拖拽排序的让位动画在快速甩动时追不上指针，计划引入滞后区间，只有越过目标槽位中线 4px 才提交换位。三、输入法组合输入期间 keydown 会误触发清空，需要改用 compositionend 提交。
下周计划：一、详情编辑面板接入撤销栈，支持三十步回退；二、收敛 glass.css 的阴影令牌为三档规格；三、补齐触摸屏长按拖拽，为平板形态留口子；四、把 reduced-motion 的全链路过一遍；五、给清单行加键盘可达性，Tab 聚焦加空格勾选。
心得：这周最大的收获是吃透了"动效是覆盖层、重绘是事实源"这句话——所有位移都建立在数据一次性提交的基础上，顺序一旦颠倒，动画就成了修不完的补丁。第二个体会是占位符的价值：它让"拖走"与"放回"共用同一套布局数学，代码量直接省了一半。第三点是测试先行在动效上也成立：先写好断言的最终布局，动画随便怎么调，收不住布局就过不了关。`;
const now = Date.now();
const todos = [
  {
    id: ++todoSeq,
    text: "买牛奶",
    done: false,
    createdAt: now,
    note: "2L 全脂一盒\n鸡蛋一排\n顺路取快递",
  }, // 新鲜：无提醒
  { id: ++todoSeq, text: "写周报", done: false, createdAt: now - 30 * HOUR, note: WEEK_NOTE }, // 24~48h：黄字
  {
    id: ++todoSeq,
    text: "读一章 Rust 书",
    done: false,
    createdAt: now - 50 * HOUR,
    note: "第 4 章：所有权与借用。重点标注移动语义、借用检查器报错的三种典型场景，习题留到周末。",
  }, // >48h：红字
  {
    id: ++todoSeq,
    text: "给 CapsuleTODO 画个图标",
    done: true,
    doneAt: now, // 入档时间（归档板排序依据，2026-09-25 schema）
    createdAt: now,
    note: "三版草图：玻璃质感、描边、纯色剪影，定了玻璃质感方向。",
  }, // 已完成不显示提醒
  // 归档板演示数据（用户定案 ~10 条）：doneAt 阶梯分布，覆盖倒序排序与滚动场景
  {
    id: ++todoSeq,
    text: "定稿玻璃令牌三档阴影",
    done: true,
    doneAt: now - 2 * HOUR,
    createdAt: now - 50 * HOUR,
    note: "贴边/悬浮/环境三档，落影与棱光分离。",
  },
  {
    id: ++todoSeq,
    text: "修复详情板收板闪烁",
    done: true,
    doneAt: now - 26 * HOUR,
    createdAt: now - 72 * HOUR,
    note: "缩回曲线改平缓，越 0 镜像闪动已消除。",
  },
  {
    id: ++todoSeq,
    text: "归档板交互原型评审",
    done: true,
    doneAt: now - 3 * 24 * HOUR,
    createdAt: now - 96 * HOUR,
    note: "评审结论：勾退/删除两语义够用，批量缓议。",
  },
  {
    id: ++todoSeq,
    text: "梳理 uiverse 候选清单",
    done: true,
    doneAt: now - 4 * 24 * HOUR,
    createdAt: now - 120 * HOUR,
    note: "R001 规范过一遍，可令牌化的标记完毕。",
  },
  {
    id: ++todoSeq,
    text: "配置预览服务自启",
    done: true,
    doneAt: now - 5 * 24 * HOUR,
    createdAt: now - 144 * HOUR,
    note: "http-server 8123 端口，禁缓存参数必带。",
  },
  {
    id: ++todoSeq,
    text: "拖拽让位公式专项复盘",
    done: true,
    doneAt: now - 6 * 24 * HOUR,
    createdAt: now - 168 * HOUR,
    note: "中心线判定 + 差值补偿，坐标快照定格起拖视口。",
  },
  {
    id: ++todoSeq,
    text: "清理过期探针脚本",
    done: true,
    doneAt: now - 7 * 24 * HOUR,
    createdAt: now - 192 * HOUR,
    note: ".temp 只留可复用的采样模板。",
  },
  {
    id: ++todoSeq,
    text: "备份 configs 与 data",
    done: true,
    doneAt: now - 8 * 24 * HOUR,
    createdAt: now - 216 * HOUR,
    note: "双落址目录整体拷贝，恢复演练通过。",
  },
  {
    id: ++todoSeq,
    text: "读完《弹性布局实践》第三章",
    done: true,
    doneAt: now - 9 * 24 * HOUR,
    createdAt: now - 240 * HOUR,
    note: "重点：限高容器的收缩分配与最小尺寸。",
  },
  {
    id: ++todoSeq,
    text: "预约牙医（洗牙）",
    done: false,
    createdAt: now,
    note: "工作日晚上有号，周五前订好。",
  }, // 新增演示条目
  {
    id: ++todoSeq,
    text: "交房租",
    done: false,
    createdAt: now,
    note: "每月 5 号前，银行卡留够余额。",
  },
  {
    id: ++todoSeq,
    text: "回复设计评审邮件",
    done: false,
    createdAt: now,
    note: "重点回复动效时长那两条意见，语气放软。",
  },
  {
    id: ++todoSeq,
    text: "给妈妈订生日蛋糕",
    done: false,
    createdAt: now,
    note: '栗子口味，裱字写"妈"不写全名。',
  },
  {
    id: ++todoSeq,
    text: "整理显示器下的线材",
    done: false,
    createdAt: now,
    note: "买一卷理线槽，周六动手收拾。",
  },
];
let bubbleSeq = 0;
const bubbles = [
  { id: ++bubbleSeq, text: "示例片段：把设计实验场的配方读数抄回 glass.css" },
  { id: ++bubbleSeq, text: "示例片段：uiverse 动效候选 R001-5 成功对勾曲线" },
  // 测试数据（用户要求 2026-09-25）：中长文 = 行内两行截断观察用（板内不滚）；
  // 超长文 = 板内滑杆观察用（实测门槛约 300 字，611px > 板内容区 277px）
  {
    id: ++bubbleSeq,
    text: "中长气泡：玻璃质感是三层叠加的产物——半透明底色给体积，backdrop-filter 给实时模糊，描边与落影给边界，单独任何一层都不构成玻璃，叠在一起才是玻璃。这一条在行里超过两行，只露两行并以省略号收尾；双击进板看全文，内容不足板高、滑杆不出现。",
  },
  { id: ++bubbleSeq, text: "复制的一段话：玻璃的质感来自克制，而不是效果的堆叠" },
  {
    id: ++bubbleSeq,
    text: "超长气泡：玻璃质感是三层叠加的产物——半透明底色给体积，backdrop-filter 给实时模糊，描边与落影给边界，单独任何一层都不构成玻璃，叠在一起才是。往细处说：模糊半径不追大，8 到 20px 足够柔化背景又不把层次糊成一团；着色永远浅，21% 的 accent 淡染只负责提示身份，信息对比交给前景墨色；边界必须交代，1px 内描边加一道向下落影，玻璃才有厚度而不是一块滤镜。这一条故意写得足够长——行内只露出两行加省略号，双击弹出只读全文板后，内容超过玻璃板高度，右侧缝隙里的 4×16 玻璃浮钮滑杆出现，按住上下拖拽即滚动全文，松手即停，滚到尽头自动贴边。临时剪贴板的定位本来就是十五秒记忆：捕获动作的成本必须远低于打开备忘录，看完即弃；溢出提醒只是温和地催你清台面，而不是替你决定什么值得留下。悬而未决的事也记在这里：清单页同款死区配方要不要统一，等三个列表的溶解节奏对照过再定；双层毛玻璃先挂在 problems 备忘录，等真窗口 DWM 阶段一并验证。最后一段纯粹为了把滑杆撑出来：滚动在手写实现里比看起来简单——scrollHeight 减 clientHeight 得到总行程，scrollTop 除以它就是比例，比例乘轨道高就是浮钮位置；难的是手感，拖拽要跟手、松手要即停、边界不弹跳，每一条都值得单独调。",
  },
];
const SAMPLE_SNIPPETS = [
  "示例片段：随手复制的一段网址 https://example.com/design-lab",
  "示例片段：临时记一下——明早把白板草稿誊进待办",
  "示例片段： conferences 关键词清单，稍后整理",
  // 长文模拟（用户要求补 2026-09-25）：行内两行截断省略号 + 板内滑杆出现时机——
  // 板内容区约 277px 高，此条需足够长才能撑出滑杆（实测门槛 ≈15 行）
  "长文片段：玻璃质感是三层叠加的产物——半透明底色给体积，backdrop-filter 给实时模糊，描边与落影给边界，单独任何一层都不构成玻璃，叠在一起才是。往深处说，玻璃的克制来自三个取舍：模糊半径不追大，8 到 20px 足够柔化背景又不把层次糊成一团；着色永远浅，21% 的 accent 淡染只负责提示身份，信息对比交给前景墨色；边界必须交代，1px 内描边加一道向下落影，玻璃才有厚度而不是一块滤镜。这条模拟数据故意写得足够长——在气泡行里只露出两行并以省略号收尾，第二行之后的文字安静地留在数据里；双击它弹出只读全文板，当内容超过玻璃板高度时，右侧缝隙里的 4×16 玻璃浮钮滑杆就会出现，按住上下拖拽即可滚动全文，松手即停，滚到尽头自动贴边。临时剪贴板的定位本来就是十五秒记忆：捕获动作的成本必须远低于打开备忘录，看完即弃；溢出提醒只是温和地催你清台面，而不是替你决定什么值得留下。悬而未决的两件事也记在这里：清单页同款死区配方要不要统一，等三个列表的溶解节奏对照过再定；双层毛玻璃先挂在 problems 备忘录，等真窗口 DWM 阶段一并验证。最后一段纯粹为了把滑杆撑出来：滚动在手写实现里比看起来简单——scrollHeight 减 clientHeight 得到总行程，scrollTop 除以它就是比例，比例乘轨道高就是浮钮位置；难的是手感，拖拽要跟手、松手要即停、边界不弹跳，每一条都值得单独调。",
];
let MAX_BUBBLES = 5; // 气泡提醒阈值：设置板步进可改（1~20，会话内有效刷新重置）
let confirmingClear = false;
let confirmTimer = 0;
let clearWidthTimer = 0;
let copiedTimer = 0;
let bubbleCopyTimer = 0; // 气泡单击开板延迟句柄（留双击窗口给复制，todos.js 置 detail.js 清）
let wbTimer = 0;
let wbSaved = "";
let snippetIdx = 0;

const $ = (id) => document.getElementById(id);
