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
- **查看详情 / 写笔记**：单击条目正文打开详情板；**有笔记的行左上角常显红点**；
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
- `data\images\`——图片气泡的图片文件（每个气泡一个小文件夹）。

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

CapsuleTODO V0.2.2 · 仅供学习交流使用

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
- **Details / notes**: click a row to open the detail panel; **rows with a note show a red dot**;
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
- `data\images\` — image files for image bubbles (one folder per bubble).

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

CapsuleTODO V0.2.2 · For learning and exchange only
