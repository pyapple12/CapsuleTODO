# CapsuleTODO 使用说明

玻璃质感的桌面 Todo 看板——常驻桌面一角，随手记、随手勾。

---

## 快速上手

1. 把 `capsule-todo.exe` 放到任意文件夹（建议单独建一个，如 `D:\CapsuleTODO\`）；
2. 双击运行——首次启动会在 exe 同级自动创建 `configs\` 和 `data\` 两个文件夹（配置和数据都住这里，**不要删**）；
3. 玻璃小板出现在屏幕右上角，开始用吧。

> 绿色单文件版，无需安装、不写注册表；换电脑 = 整个文件夹拷走。

## 日常使用

### 待办清单

- **添加**：顶部输入框输入，回车确认（最多 12 字）；
- **勾选完成**：点条目左侧圆圈——播放勾选动画后收入归档；
- **归档自动清理**：已完成条目在归档中保留 7 天，超时自动删除（不可恢复）；
- **修改标题**：双击条目直接在行内编辑；
- **查看详情 / 写笔记**：单击条目正文打开详情板；**有笔记的行左上角常显红点**；单条笔记请控制在约 **10,000 字符 / 100 块**以内（超出属非正常使用，不保证保存）；粘贴内容会超过上限时会静默失败（Ctrl+V 无反应，原文保持不变）；
- **图文详情**：详情板按段落成块，`Ctrl+V` 可直接粘贴截图 / 图片；图块可拖动排序，`Ctrl+Enter` 保存并关板；
- **删除**：条目右侧删除钮，点两次确认；
- **调整顺序**：按住条目拖动。

### 气泡（临时剪贴板）

- **文字**：复制文字后按全局热键（默认 **Ctrl + Alt + C**）；选中文字直接按热键也行；
- **图片**：截图到剪贴板、或复制一个图片文件后按热键，图片也会收进气泡页；
- **看内容**：单击气泡看全文 / 看大图；图片详情里**双击图片用系统默认程序打开原图**；
- **复制回**：双击气泡把内容写回剪贴板——文字照常；图片保真（截图带透明通道，图片文件保留原名）；
- 支持拖动排序；攒满上限（默认 5 条）出现红色提醒。

### 白板

随写随存的临时记事板，切换页签即可使用，内容自动保存。

### 窗口

- **拖动位置**：按住顶部标题栏拖；
- **双击标题**：缩回托盘（彻底退出见下）；
- **贴边吸附 / 窗口置顶**：默认关闭，可在设置或托盘菜单开启。

### 托盘图标

- **悬停**：预览待办清单；**左键**：显示 / 聚焦主窗；**右键**：菜单（聚焦 / 吸附 / 置顶 / 退出）。

## 设置

| 设置项       | 说明                                                                                          |
| ------------ | --------------------------------------------------------------------------------------------- |
| 跟随系统     | 开 = 主题随 Windows 深浅色自动切换（默认开）；关后可手动选浅色/暗色，选择会被记住（重启保持） |
| 气泡提醒数量 | 攒满多少条气泡时提醒清理（1 ~ 20）                                                            |
| 捕获热键     | 全局热键组合，点击后按下想要的组合即可录入；被占用会红字提示并回退                            |

## 数据与备份

- `configs\config.json`——窗口位置、气泡上限、热键等设置；
- `data\todo.db`——待办、归档、气泡、白板的全部内容；
- `data\images\`——图片气泡与详情图块的文件（每个图片一个小文件夹）。

**备份** = 把这两个文件夹复制走；**恢复** = 覆盖回来。

## 退出

点右键托盘图标 → **退出** 才会真正关闭（自动保存窗口位置）；双击标题 / Alt+F4 只是缩回托盘。

## 常见问题

**按热键没反应？** 该组合可能被其他软件占用，打开设置换一个；仍不行时检查输入法或安全软件是否拦截了全局热键。

**图片气泡显示"文件已丢失"？** 该图片文件被手动删掉了（在 `data\images\` 里），详情里点「删除气泡」清掉即可，不影响其它气泡。

**杀毒软件报毒？** 绿色单文件 exe 无数字签名，部分杀软会误报；本项目开源（GitHub: pyapple12/CapsuleTODO），可自行核对源码后添加信任。

**窗口不见了？** 多半缩在托盘里，点托盘图标唤回；也可能记忆位置在已拔掉的副屏上，重启应用会自动回到主屏默认位（右上角）。

**系统要求？** Windows 10 / 11（64 位）。

## 更新日志

### v0.2.3

**新增**

- 📝 **详情图文混排**：详情板从纯文本升级为「段落 / 图块」序列，文字与图片可混排；
- 📋 **直接粘贴图片**：在详情板 `Ctrl+V` 粘贴截图或图片，自动落盘到 `data\images\`，图块可拖动调序；
- 🌐 **远端图片**：粘贴网页图片时先显示「下载中…」占位，后台下载成功后替换为图片，失败自动降级为链接；
- 🧷 **多段粘贴与复制**：支持 HTML 多图粘贴、文本 + 图片多格式复制、图块拖拽排序与撤销；
- 🪟 **按需毛玻璃**：仅详情板 / 浮板打开且窗口聚焦时挂 Windows 亚克力，收板 / 失焦立即恢复透明；
- 🎨 **应用图标更新**：正式版图标替换，并修正任务栏 / Alt+Tab 图标清晰度。

**优化**

- 详情编辑：段落拆合、跨块选区、方向键跨块、输入法组字、全选替换与空块归一化统一收口；
- 滚动与拖拽：详情板贴底、顶部软边、浮板裁剪、图块拖拽换位与滚动锁定统一；
- 图片链路：同内容复用、落盘预览、坏图回退、远端下载预算与锁外 I/O 加固；
- 托盘与窗口：左右键 / hover 预览互斥、按需背板、单实例唤前与位置记忆收口。

**修复**

- 修复详情粘贴在空笔记、切源、选区替换和撤销时的内容丢失 / 错位；
- 修复详情图块删除后撤销、图块拖拽落点、详情板滚动跳动与贴底回弹；
- 修复远端图占位、过期保存与图片任务竞态导致的内容不一致；
- 修复图片气泡复制 / 详情大图 / 坏 PNG / DIB alpha / 剪切板多格式写入问题；
- 修复输入法 Enter、全选、退格、方向键与跨块选区在详情中的按键冲突；
- 修复归档板、气泡板、设置板和托盘菜单的罩死、滑杆、点击穿透与状态刷新问题；
- 修复主题切换 / DWM 背板 / 单实例 / 窗口拖动焦点闪烁等平台观感问题；
- 修复 HTML 粘解析边界、块数 / 字符上限校验与失败恢复路径；
- 修复前端 DEV mock、诊断构建、CI 与前端测试门禁的一致性问题。

### v0.2.2

**新增**

- 🖼️ 图片气泡：热键可捕获截图 / 图片文件，支持看大图、复制回（保真）；
- 🔍 查看原图：图片详情双击用系统默认程序打开原图；
- ⚡ 大图预览：大图自动降采样预览，详情秒开、原图保留；
- 🔴 笔记红点：有笔记的清单行常显红点，点击直达详情板；
- 💬 占字提示判别：已捕获 / 已复制 / 重复捕获，无效！。

**优化**

- 图片改存 `data\images\`，数据库更轻、备份更直观；图片按原格式保存，更小、动图不丢帧。

**修复**

- 旧剪贴板图片不再短路新选中内容；
- 图片气泡文件丢失时提示 + 一键删除，不再卡死；
- 详情删除图片气泡后列表即时同步，不再残留死条目；
- 失效页溢出（滚动条 / 玻璃滑杆）；
- 重复捕获后捕获钮卡死；
- 详情板图片滑杆 / 三角与"关板后滑杆残留"；
- 气泡→待办切换时图片残留串板；
- 白板错误行被气泡样式覆盖；
- 捕获错误行不自动消失（现 1 秒渐隐，不露系统英文细节）；
- 图片壳横向多余滑杆；
- 罩死行笔记红点未淡出。

### v0.2.1

- 归档自动回收（已完成条目保留 7 天）；
- 主题选择会被记住；
- 修复行内编辑光标跳行尾、跨条目编辑草稿丢失、罩死行误入编辑。

---

CapsuleTODO V0.2.3 · 仅供学习交流使用

---

# CapsuleTODO User Guide

A glassy desktop Todo board that stays in a corner of your desktop — jot and tick as you go.

## Quick Start

1. Put `capsule-todo.exe` in any folder (e.g. `D:\CapsuleTODO\`);
2. Double-click to run — on first launch it creates `configs\` and `data\` next to the exe (**do not delete**);
3. The glass panel appears in the top-right corner — you're ready.

> Portable single-exe build, no installer, no registry writes; migrating = copy the whole folder.

## Daily Use

### Todo List

- **Add**: type in the top field and press Enter (max 12 chars);
- **Complete**: click the circle on the left — it animates into the archive;
- **Auto-cleanup**: finished items stay in the archive for 7 days, then are deleted automatically;
- **Rename**: double-click a row to edit inline;
- **Details / notes**: click a row to open the detail panel; **rows with a note show a red dot**; keep a note under about **10,000 characters / 100 blocks** (beyond that is outside normal use and is not guaranteed to save); a paste that would exceed the limit fails silently (Ctrl+V does nothing and the original content is kept);
- **Rich-text details**: paragraphs and image blocks live together; press `Ctrl+V` in the detail panel to paste screenshots or images; drag image blocks to reorder; `Ctrl+Enter` saves and closes;
- **Delete**: the delete button on the right — click twice to confirm;
- **Reorder**: drag a row.

### Bubbles (Clipboard)

- **Text**: copy text and press the hotkey (default **Ctrl + Alt + C**); selecting text and pressing the hotkey also works;
- **Images**: screenshot to the clipboard, or copy an image file, then press the hotkey;
- **View**: click a bubble for the full text / large image; in an image bubble **double-click the image to open the original**;
- **Copy back**: double-click a bubble to copy it back — text as usual; images are faithful (transparency kept, original file name kept);
- Drag to reorder; a red warning appears when you reach the limit (default 5).

### Whiteboard

A scratch pad that saves as you type — just switch to its tab.

### Window

- **Move**: drag the title bar;
- **Double-click the title**: hide to tray (see "Exit" to quit fully);
- **Snap / Always-on-top**: off by default; enable in settings or the tray menu.

### Tray

- **Hover**: preview the list; **left-click**: show/focus; **right-click**: menu (focus / snap / on-top / exit).

## Settings

| Setting        | Description                                                                      |
| -------------- | -------------------------------------------------------------------------------- |
| Follow system  | Theme follows Windows light/dark automatically (default on); off = pick manually |
| Bubble limit   | How many bubbles trigger the cleanup reminder (1–20)                             |
| Capture hotkey | Global hotkey combo; click and press the combo you want (occupied = red warning) |

## Data & Backup

- `configs\config.json` — window position, bubble limit, hotkey, etc.;
- `data\todo.db` — todos, archive, bubbles and whiteboard;
- `data\images\` — image files for image bubbles and detail image blocks (one folder per image).

Back up = copy these two folders; restore = copy them back.

## Exit

Right-click the tray icon → **Exit** to quit (saves the window position); double-click the title / Alt+F4 only hides to tray.

## FAQ

**Hotkey not working?** The combo may be taken — change it in settings; otherwise check your IME or antivirus.

**"File missing" on an image bubble?** Its file was deleted from `data\images\` — click "Delete" in the detail panel to remove the bubble; other bubbles are unaffected.

**Antivirus warning?** The unsigned single exe triggers false positives; the project is open source (GitHub: pyapple12/CapsuleTODO) — verify the source and add an exception.

**Window gone?** Likely hidden in the tray — click the tray icon; or its saved position was on a disconnected monitor, so restarting returns it to the default spot.

**Requirements?** Windows 10 / 11 (64-bit).

## Changelog

### v0.2.3

**Added**

- 📝 **Rich-text details**: the detail panel is now a sequence of paragraph and image blocks, mixing text and images freely;
- 📋 **Paste images directly**: press `Ctrl+V` in the detail panel to paste a screenshot or image; files are stored under `data\images\` and image blocks can be dragged to reorder;
- 🌐 **Remote images**: pasting an image from a web page shows a "Downloading…" placeholder first, then replaces it when the background download succeeds; failures gracefully fall back to a link;
- 🧷 **Multi-part paste / copy**: HTML multi-image paste, mixed text + image copy, drag reordering and undo for image blocks;
- 🪟 **On-demand acrylic**: Windows Acrylic is attached only while a detail / floating board is open and focused, and removed on close or blur;
- 🎨 **New app icon**: refreshed release icon and sharper taskbar / Alt+Tab rendering.

**Improved**

- Detail editing: paragraph split/merge, cross-block selection, arrow-key navigation, IME composition, select-all replacement and empty-block normalization;
- Scrolling and dragging: sticky-bottom detail panel, top fade, floating-board clipping, image-block reorder and scroll locking;
- Image pipeline: content reuse, on-disk previews, corrupt-image fallback, remote download budgets and off-lock I/O;
- Tray and window: left/right-click and hover-preview mutual exclusion, on-demand backdrop, single-instance activation and position persistence.

**Fixed**

- Fixed detail paste losing or misplacing content in empty notes, when switching items, during selection replacement, and after undo;
- Fixed detail image-block delete/undo, image drag drop targets, detail scrolling jitter and bottom bounce;
- Fixed remote-image placeholders, stale saves and image-job races causing inconsistent content;
- Fixed image-bubble copy, detail large image, corrupt PNG, DIB alpha and clipboard multi-format writes;
- Fixed IME Enter, select-all, Backspace, arrow keys and cross-block selection conflicts in details;
- Fixed archive / bubble / settings / tray menu masking, sliders, click-through and refresh issues;
- Fixed theme switching / DWM backdrop / single-instance / drag focus flicker;
- Fixed HTML paste parsing boundaries, block/character limit checks and failure recovery;
- Fixed DEV mock, diagnostics builds, CI and frontend test-gate consistency.

### v0.2.2

**Added**

- 🖼️ Image bubbles: capture screenshots / image files with the hotkey, view large, copy back faithfully;
- 🔍 Open original: double-click the image in the detail panel;
- ⚡ Large-image preview: downscaled preview, instant detail open, original preserved;
- 🔴 Note indicator: rows with a note show a red dot;
- 💬 Clearer feedback: Captured / Copied / Duplicate, ignored!.

**Improved**

- Images now stored under `data\images\` (lighter DB, clearer backup) and kept in their original format (smaller, animations preserved).

**Fixed**

- An old clipboard image no longer overrides a new selection;
- A missing image bubble now shows a hint and one-click delete instead of getting stuck;
- Deleting an image bubble now refreshes the list immediately;
- Broken-image panel overflowing (stray scrollbar / glass slider);
- Capture button getting stuck after a duplicate;
- Detail-panel image slider/triangles and the slider lingering after closing;
- Image left over when switching from a bubble to a todo;
- Whiteboard error line overridden by the bubble style;
- Capture errors not auto-hiding (now fades out after 1s, without leaking system details);
- Extra horizontal slider on the image shell;
- Note dot not fading out on masked rows.

### v0.2.1

- Auto-cleanup of the archive (finished items kept for 7 days);
- Theme choice is remembered;
- Fixed inline-edit cursor jumping to line end, draft loss on cross-item editing, and masked rows entering edit mode.

---

CapsuleTODO V0.2.3 · For learning and exchange only
