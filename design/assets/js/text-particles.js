// ===== 标题粒子化（参考 Da0Mine/text-particleization，MIT；CT 版改造：局部画布 / accent 色 /
// clearRect 无拖尾 / 2x 采样密度 / reduced-motion 守卫 / 静止自动停帧省 GPU） =====
function initTitleParticles() {
  const h1 = document.querySelector(".title");
  if (!h1) return null;
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return null;
  const canvas = h1.querySelector(".title-canvas");
  const ctx = canvas.getContext("2d");
  const card = h1.closest(".glass-card");
  const tabs = document.querySelector(".tabs");
  h1.classList.add("title--particles"); // 仅粒子形态：原文字透明让位（用户定案）
  const TEXT = "CapsuleTODO";
  const GAP = 2; // 采样步长（离屏像素）
  const REPEL_RADIUS = 55; // 排斥半径（收敛方案，用户定案）
  const REPEL_FORCE = 0.45;
  const SPRING = 0.06; // 回弹比排斥更快耗尽，扰动不拖尾（收敛方案）
  const FRICTION = 0.85;
  const MAX_OFFSET = 15; // 位移软钳制半径：离 home 超限即投影回球面（用户定案）
  const SETTLE_DIST = 0.5; // 最远粒子距 home 低于此且鼠标远离 → 吸附归位并停帧
  const FRAME_CAP = 240; // 硬上限：超时强制收尾（鼠标远离前提下）
  const MARGIN_X = 30;
  const MARGIN_TOP = 20; // 上余量：辉光上摆（模糊 15）+ 粒子散开；与 .title-canvas top:-20px 联动
  const MARGIN_BOTTOM = 30; /* 下余量：辉光下摆（偏移10+模糊15）+ 粒子散开不侵页签 */
  let particles = [];
  let dpr = 1;
  let raf = 0;
  let rect = null; // 标题视口坐标缓存（resize/scroll 刷新）
  let mouse = { x: -9999, y: -9999 }; // 画布坐标系，-9999 = 远离
  let frames = 0; // 周期帧计数：每轮动画独立起算（build 重建 / 出区弹回各归零），240 帧兜底只对本轮生效
  let zone = null; // 唤醒区（页面坐标矩形）：卡左右缘 × 卡顶→页签顶，随 build/scroll 刷新
  let shadow = { x: 1.7, y: 1.7, blur: 8, alpha: 0.35 }; // 落影参数：build 时从 --panel-shadow 令牌解析
  let glowColor = "rgba(192, 176, 253, 0.35)"; // 辉光色：build 时从 --input-focus-glow 令牌解析（回退 accent 35%）
  const GLOWS = [
    { x: 0, y: 10, blur: 15, spread: 3 },
    { x: 0, y: 4, blur: 6, spread: 4 },
  ]; /* 输入框聚焦辉光同款两层（外晕+近光）：spread = 配方负收缩，只采距缘足够深的点 */
  const SHADOW_THICK = 1.5; // 影子点在粒子半径上加粗的量：模糊后更厚实（用户定案）
  const sCan = document.createElement("canvas"); // 影层离屏：同坐标暗点整层模糊后垫底，随粒子位移
  const sctx = sCan.getContext("2d");

  const accent = () =>
    (getComputedStyle(document.documentElement).getPropertyValue("--accent") || "#7c3aed").trim();

  // 离屏 2x 采样：30px 绘制文字 → alpha>128 网格扫描 → 粒子（home = 字形坐标）
  function build() {
    frames = 0; // 重建即新周期：重聚动画重新获得完整兜底预算，不继承上一轮累计
    rect = h1.getBoundingClientRect();
    const cr = card.getBoundingClientRect();
    const tr = tabs.getBoundingClientRect();
    zone = { left: cr.left, right: cr.right, top: cr.top, bottom: tr.top };
    dpr = Math.min(window.devicePixelRatio || 1, 2);
    const cw = rect.width + MARGIN_X * 2;
    const ch = rect.height + MARGIN_TOP + MARGIN_BOTTOM;
    canvas.width = Math.round(cw * dpr);
    canvas.height = Math.round(ch * dpr);
    canvas.style.width = `${cw}px`;
    canvas.style.height = `${ch}px`;
    sCan.width = canvas.width;
    sCan.height = canvas.height;
    // 落影/辉光参数取自令牌（单一来源，随主题换色自动跟随）；解析失败回退基准值
    const cs = getComputedStyle(document.documentElement);
    const raw = cs.getPropertyValue("--panel-shadow");
    const dims = raw.match(/([\d.]+)px\s+([\d.]+)px\s+([\d.]+)px/);
    const a = raw.match(/,\s*([\d.]+)\)/);
    shadow = dims
      ? {
          x: Number(dims[1]),
          y: Number(dims[2]),
          blur: Number(dims[3]),
          alpha: a ? Number(a[1]) : 0.35,
        }
      : { x: 1.7, y: 1.7, blur: 8, alpha: 0.35 };
    glowColor = cs.getPropertyValue("--input-focus-glow").trim() || glowColor;
    // 采样参数取自标题实际样式：加粗/放大自动落到粒子上（用户定案）
    const style = getComputedStyle(h1);
    const cssFont = parseFloat(style.fontSize) || 15;
    const cssWeight = style.fontWeight || "700";
    const sampleFont = cssFont * 2; // 2 倍采样：渲染减半呈现，密度翻倍
    const font = `${cssWeight} ${sampleFont}px "SF Pro Display", "Segoe UI Variable Display", "Segoe UI", sans-serif`;
    const off = document.createElement("canvas");
    const octx = off.getContext("2d", { willReadFrequently: true });
    octx.font = font;
    const tw = octx.measureText(TEXT).width;
    off.width = Math.ceil(tw + 8);
    off.height = Math.ceil(sampleFont * 1.5);
    octx.font = font; // 重设尺寸会重置上下文，需再赋一次
    octx.textBaseline = "middle";
    octx.fillStyle = "#fff";
    octx.fillText(TEXT, 4, off.height / 2);
    const data = octx.getImageData(0, 0, off.width, off.height).data;
    const scale = rect.width / tw; // 粒子字形精确铺满标题宽（≈0.5，即 2x 采样）
    const midY =
      MARGIN_TOP + rect.height / 2; /* 字形中心随盒：line-height 1 后墨迹顶≈盒顶=18（用户定案） */
    // 格点全集先收集：逐环外扩查空格可得每点到字形边缘的距离（1 格 ≈ 1 CSS px）
    const dots = new Set();
    for (let y = 0; y < off.height; y += GAP)
      for (let x = 0; x < off.width; x += GAP)
        if (data[(y * off.width + x) * 4 + 3] > 128) dots.add(`${x},${y}`);
    particles = [];
    for (let y = 0; y < off.height; y += GAP) {
      for (let x = 0; x < off.width; x += GAP) {
        if (!dots.has(`${x},${y}`)) continue;
        // 到字形边缘距离（CSS px）：辉光负 spread = 轮廓内缩，采 ed ≥ spread 的点近似
        let ed = 5 * GAP * scale;
        outer: for (let ring = 1; ring <= 5; ring++) {
          for (let ny = y - ring * GAP; ny <= y + ring * GAP; ny += GAP) {
            for (let nx = x - ring * GAP; nx <= x + ring * GAP; nx += GAP) {
              const onRing = Math.abs(nx - x) === ring * GAP || Math.abs(ny - y) === ring * GAP;
              if (onRing && !dots.has(`${nx},${ny}`)) {
                ed = ring * GAP * scale;
                break outer;
              }
            }
          }
        }
        const hx = MARGIN_X + (x - 4) * scale;
        const hy = midY + (y - off.height / 2) * scale;
        // 开场：随机撒在标题周边，靠弹簧汇聚成字
        particles.push({
          hx,
          hy,
          x: hx + (Math.random() - 0.5) * 80,
          y: hy + (Math.random() - 0.5) * 40,
          vx: 0,
          vy: 0,
          ed,
        });
      }
    }
  }

  function frame() {
    frames++;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, canvas.width, canvas.height); // 真清屏：玻璃上不留拖尾
    let maxHome = 0; // 本帧所有粒子离 home 的最大距离：聚拢判定用最远者（用户定案）
    for (const p of particles) {
      const dx = p.x - mouse.x;
      const dy = p.y - mouse.y;
      const dist = Math.hypot(dx, dy);
      if (dist < REPEL_RADIUS && dist > 0.01) {
        const f = ((REPEL_RADIUS - dist) / REPEL_RADIUS) * REPEL_FORCE;
        p.vx += (dx / dist) * f;
        p.vy += (dy / dist) * f;
      }
      p.vx = (p.vx + (p.hx - p.x) * SPRING) * FRICTION;
      p.vy = (p.vy + (p.hy - p.y) * SPRING) * FRICTION;
      p.x += p.vx;
      p.y += p.vy;
      // 软钳制：开场 120 帧后启用，粒子无论被扫多狠，离 home 不超过 MAX_OFFSET（用户定案）
      const ddx = p.x - p.hx;
      const ddy = p.y - p.hy;
      const homeDist = Math.hypot(ddx, ddy);
      if (frames > 120 && homeDist > MAX_OFFSET) {
        p.x = p.hx + (ddx / homeDist) * MAX_OFFSET;
        p.y = p.hy + (ddy / homeDist) * MAX_OFFSET;
      }
      if (homeDist > maxHome) maxHome = homeDist;
    }
    paint(); // 影子层 + 粒子本体（影子随粒子位移同步，用户定案）
    // 静止自动停帧：最远粒子距 home < 0.5px 视作聚拢完成 → 吸附归位画出零残差终帧再休眠
    //（按最远而非均值：出区时仅光标附近一小簇散开，均值被未动粒子稀释后会误判"已聚拢"
    // 而首帧瞬移归位——2026-09-18 实测）；鼠标远离前提下本周期超 240 帧强制收尾。靠近/重建时唤醒
    const far = mouse.x < -999;
    if (far && (maxHome < SETTLE_DIST || frames > FRAME_CAP)) {
      for (const p of particles) {
        p.x = p.hx;
        p.y = p.hy;
        p.vx = 0; // 吸附同时清零速度：防唤醒首帧旧速度回放抖动（用户定案）
        p.vy = 0;
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, canvas.width, canvas.height); // 清掉旧帧残影再画终帧（用户定案）
      paint(); // 终帧同款绘制：影子随粒子归位（用户定案）
      raf = 0;
      return;
    }
    raf = requestAnimationFrame(frame);
  }

  // 影层统一管线：同坐标暗点（半径 = 粒子半径 + thick）落进离屏 → 整层一次模糊 → 叠到主画布
  //（设备像素空间整层模糊，半径按 dpr 放大，观感即 CSS px）
  function paintLayer(color, ox, oy, thick, blur, alpha, spread) {
    sctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    sctx.clearRect(0, 0, canvas.width, canvas.height);
    sctx.fillStyle = color;
    for (const p of particles) {
      if (p.ed < spread) continue; // 负 spread：贴缘点不进辉光，等效轮廓内缩（box-shadow 语义）
      const r = 0.5 + Math.min(1, Math.hypot(p.vx, p.vy) / 3) * 0.4 + thick;
      sctx.beginPath();
      sctx.arc(p.x + ox, p.y + oy, r, 0, 6.2832);
      sctx.fill();
    }
    ctx.save();
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.filter = `blur(${blur * dpr}px)`;
    ctx.globalAlpha = alpha;
    ctx.drawImage(sCan, 0, 0);
    ctx.restore(); // 恢复变换/滤镜/透明度
  }

  // 影层 + 粒子本体一次画齐，叠序 = 旧 CSS 滤镜链：辉光两层 → 玻璃落影 → 粒子；
  // 全部同坐标暗点经离屏模糊，散开/弹回全程与粒子同步（用户定案）
  function paint() {
    for (const g of GLOWS) paintLayer(glowColor, g.x, g.y, 0, g.blur, 1, g.spread);
    paintLayer("#000", shadow.x, shadow.y, SHADOW_THICK, shadow.blur, shadow.alpha, 0);
    ctx.fillStyle = accent();
    for (const p of particles) {
      const disp = Math.min(1, Math.hypot(p.vx, p.vy) / 3); // 位移越大越大（不透明恒定，用户定案）
      ctx.globalAlpha = 1;
      ctx.beginPath();
      ctx.arc(p.x, p.y, 0.5 + disp * 0.4, 0, 6.2832); // 粒子基础半径 0.5（用户定案）
      ctx.fill();
    }
  }

  function wake() {
    if (!raf) raf = requestAnimationFrame(frame);
  }

  // 交互监听在 window：光标落在"卡宽 × 卡顶→页签顶"矩形（zone）内即唤醒并施加排斥，
  // 越界即视作远离（粒子弹回归位）。边界取自卡片/页签实测矩形，布局改动自动跟随
  const track = (clientX, clientY) => {
    if (!rect || !zone) return;
    const cx = clientX - rect.left + MARGIN_X;
    const cy = clientY - rect.top + MARGIN_TOP;
    const near =
      clientX >= zone.left &&
      clientX <= zone.right &&
      clientY >= zone.top &&
      clientY <= zone.bottom;
    if (!near) {
      mouse.x = -9999;
      mouse.y = -9999;
      frames = 0; // 出区弹回按新周期计帧：长时把玩后累计帧数超限，不清零则弹回首帧即被强制归位
      wake(); // 出区也唤醒：弹回归位动画必然播完，不残留散开态（用户定案）
      return;
    }
    mouse.x = cx;
    mouse.y = cy;
    wake();
  };
  window.addEventListener("mousemove", (e) => track(e.clientX, e.clientY), { passive: true });
  window.addEventListener(
    "touchmove",
    (e) => {
      if (e.touches.length) track(e.touches[0].clientX, e.touches[0].clientY);
    },
    { passive: true },
  );
  window.addEventListener("resize", () => {
    build();
    wake();
  });
  window.addEventListener(
    "scroll",
    () => {
      if (!rect) return;
      rect = h1.getBoundingClientRect(); // 视口坐标随滚动失效，仅刷新缓存
      const cr = card.getBoundingClientRect();
      const tr = tabs.getBoundingClientRect();
      zone = { left: cr.left, right: cr.right, top: cr.top, bottom: tr.top };
    },
    { passive: true },
  );

  build();
  wake(); // 开场汇聚动画
  return {
    refresh() {
      // 主题换色后重建（accent 随主题）
      build();
      wake();
    },
  };
}

const titleFX = initTitleParticles();
