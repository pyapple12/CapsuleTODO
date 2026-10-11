# Future Plan 002 — 对外站点与协作文档：GitHub Pages（Hugo）/ GitHub Wiki

> 状态：**方案记录**（2026-10-10 记录，尚未立项）。技术选型已讨论成形（Hugo + GitHub Actions 部署 Pages；Wiki 作为可选协作文档），**用户后续实现**。实施时按体量拆 PL 取号：方案细化入 `z.plan.md` 附录，执行条目入 `x.progress.md`，本文件随即归档。
> 体量排序：功能一（Pages，中等）→ 功能二（Wiki，最小，可选）。

---

## 背景与目标

项目当前对外信息全部压在 `README.md` + `release/README.md` 上，随功能增长会越来越挤。目标 = 把"介绍 / 使用手册 / FAQ / 开发文档"从 README 拆出来，做成**可浏览的站点**，README 只留门面。

两条路线（**二选一或组合**）：

- **GitHub Pages**：静态站点托管，适合落地页 / 文档站（推荐先做）。
- **GitHub Wiki**：仓库自带的协作文档，适合多人随手改的 FAQ / 手册（可选）。

---

## 功能一：GitHub Pages —— Hugo 站点（推荐先做）

### 定位

- **产品落地页**：一句话介绍 + 三屏预览图（`images/Github-Preview-Image.jpg`）+ 下载按钮（指向 Releases）；
- **文档站**：使用手册（复用 `release/README.md`）/ FAQ / 开发文档。

> ⚠️ **不能托管应用本体**：`ui/` 是 Tauri 前端，依赖 Tauri IPC 命令，**在浏览器里跑不起来**——不可把 `dist/` 部署成"在线版"。`design/` 是纯静态原型，理论上可部署成在线可点 Demo（非成品，暂列备选）。

### 站点位置

仓库根已是 Tauri 工程（`core/` / `ui/` / `release/`…），Hugo 站放**子目录 `site/`**：

```
CapsuleTODO/
├── core/ ui/ release/ …        # 现有
├── site/                       # 🆕 Hugo 站
│   ├── hugo.toml
│   ├── content/
│   ├── layouts/
│   ├── static/
│   └── themes/                 # 主题（git 子模块）
└── .github/workflows/hugo.yml
```

### 开关

**Settings → Pages → Build and deployment → Source = `GitHub Actions`**（不是 Deploy from a branch）。

### 工作流 `.github/workflows/hugo.yml`

```yaml
name: Deploy Hugo site to Pages

on:
  push:
    branches: [main]
    paths: ["site/**", ".github/workflows/hugo.yml"]
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: pages
  cancel-in-progress: false

jobs:
  build:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: site # 在 site/ 里跑
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: recursive # 主题是 git 子模块时必须
          fetch-depth: 0

      - name: Setup Hugo
        uses: peaceiris/actions-hugo@v3
        with:
          hugo-version: "0.140.0"
          extended: true # 用 SCSS 主题时必须 extended

      - name: Setup Pages
        id: pages
        uses: actions/configure-pages@v5

      - name: Build
        env:
          HUGO_ENVIRONMENT: production
          HUGO_CACHEDIR: ${{ runner.temp }}/hugo_cache
        run: hugo --minify --baseURL "${{ steps.pages.outputs.base_url }}/"

      - name: Upload artifact
        uses: actions/upload-pages-artifact@v3
        with:
          path: site/public # 产物路径

  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - id: deployment
        uses: actions/deploy-pages@v4
```

> 备选安装法（不依赖第三方 action）：build 步骤里 `wget` 官方 Hugo `.deb` + `dpkg -i`（GitHub 官方 Hugo starter 写法）；`peaceiris/actions-hugo` 更省事。

### 关键点（坑）

1. **`baseURL` 必须带子路径**：项目站网址是 `https://pyapple12.github.io/CapsuleTODO/`（**不是根**）。用 `actions/configure-pages@v5` 的 `base_url` 输出自动拼好，别手写死；本地 `hugo.toml` 也建议写同值。
2. **主题是 git 子模块** → `checkout` 必须 `submodules: recursive`，否则构建时主题目录是空的。
3. **Hugo extended**：主题用 SCSS 就必须 `extended: true`。
4. **不需要 `.nojekyll`**：走 Actions 部署不经过 Jekyll。
5. **项目站子路径**：资源用相对路径（自建前端站要设 `base`，Hugo 由 `baseURL` 处理）。

### 本地开发

```bash
cd site
hugo new site . --force   # 首次（若还没建）
hugo server -D            # 本地预览 http://localhost:1313/
```

### 内容规划（拟）

| 页面     | 内容来源                                         |
| -------- | ------------------------------------------------ |
| 首页     | 一句话介绍 + 三屏预览图 + 下载按钮（→ Releases） |
| 使用手册 | 复用 `release/README.md`                         |
| FAQ      | 复用 `release/README.md` 常见问题段              |
| 开发文档 | 复用 `README.md` 开发段 + 目录结构               |

### 自定义域（可选）

默认地址 `https://pyapple12.github.io/CapsuleTODO/` 够用；有自己的域名才配。

- **设置**（二选一，别冲突）：① Settings → Pages → **Custom domain** 填域名；② `site/static/CNAME` 写一行域名（Hugo 把 `static/` 原样拷进 `public/`，产物根即有 `CNAME`）。
- **DNS 必配**（在域名服务商）：

| 域名形式                                    | 记录        | 指向                                                     |
| ------------------------------------------- | ----------- | -------------------------------------------------------- |
| 裸域 `example.com`                          | **A** ×4    | `185.199.108.153` / `.109.153` / `.110.153` / `.111.153` |
| 裸域（IPv6，可选）                          | **AAAA** ×4 | `2606:50c0:8000::153` / `8001` / `8002` / `8003`         |
| 子域 `www.example.com` / `todo.example.com` | **CNAME**   | `pyapple12.github.io`                                    |

- **HTTPS 自动**（GitHub 免费签 Let's Encrypt，生效后勾 Enforce HTTPS）；
- **配了域名要改** `hugo.toml` 的 `baseURL`（否则站内链接/资源指向 `github.io` 子路径会 404）。

---

## 功能二：GitHub Wiki —— 协作文档（可选）

### 定位

面向用户、会频繁增补的内容（使用手册 / FAQ）；**多人随手改**时比仓库 `docs/` 方便。

### 结构与语法

| 元素          | 说明                                        |
| ------------- | ------------------------------------------- |
| `Home.md`     | 首页（必须）                                |
| `_Sidebar.md` | **侧边栏**（每页都显示，做导航目录）        |
| `_Footer.md`  | **页脚**（每页底部）                        |
| 普通页        | 任意 `.md`，文件名 = 页面标题（空格用 `-`） |
| 页面间链接    | `[[Usage]]` 或 `[[Usage\|使用说明]]`        |

### 建页方式

- **网页 UI**：Wiki 标签 → `New Page` → 标题 + Markdown → 保存（一页一页建）；
- **批量（捷径）**：先把 Wiki 当 git 仓库克隆，一次推多页——
  ```bash
  # 前提：先在网页建一个 Home 页（初始化 wiki 仓库）
  git clone https://github.com/pyapple12/CapsuleTODO.wiki.git
  cd CapsuleTODO.wiki
  # 写一堆 .md（Home.md / Usage.md / FAQ.md …）
  git add . && git commit -m "init wiki" && git push
  ```

### 权限

- 开关：**Settings → Features → Wikis**；
- 编辑权限：默认仅**有写权限的协作者**可改；取消勾选「Restrict editing to collaborators only」后**公开仓库任何人都能编辑**（一般保持限制）。

### 坑与取舍

- Wiki 是**独立仓库**（`repo.wiki.git`）：**不随主仓库 clone**、**不受 PR/Code Review 管**、**不版本化进代码历史**、不进主仓库搜索/打包；
- 因此**极易与版本脱节**（本项目已吃过"README 版本口径落后"的亏）；
- 详见下方取舍表。

---

## 取舍对照表

| 方式             | 版本化 / PR                  | 位置              | 适合                            |
| ---------------- | ---------------------------- | ----------------- | ------------------------------- |
| **GitHub Pages** | ✅（Actions 部署，源在仓库） | `xxx.github.io/…` | 落地页、文档站（推荐）          |
| **仓库 `docs/`** | ✅ 跟代码                    | 主仓库            | 开发文档、设计说明（要 review） |
| **GitHub Wiki**  | ❌ 独立仓库                  | 网页 Wiki 标签    | 社区协作 FAQ、多人随手改        |

**倾向**：**Pages（Hugo）优先**；Wiki 可选——若文档要跟版本走，用仓库 `docs/` 或 Pages 更稳。

---

## 待定项与推进建议

**推进顺序**：功能一（Pages）→ 功能二（Wiki，看是否需要）。

**待定项（定案后即可 PL 化）**：

1. **主题选型**：PaperMod / Blowfish / 自写？（未定）
2. **站点内容范围**：只做落地页，还是含完整文档站？
3. **Wiki 要不要做**：若文档要跟版本走，建议改用 `docs/` 或 Pages 承载。
4. **自定义域**：是否已有域名？有则一并配 CNAME + DNS + `baseURL`。
5. **`design/` 是否部署成在线 Demo**：纯静态可部署（非成品，看是否需要）。
