# 从设计源派生五个 PNG（32/64/128/128@2x/icon.png）+ 多尺寸 ICO（共六个文件）+
# 预览图（PATCH002.59，来源 = 系列项目 CapsulePulse `core/icons/mk-icons.ps1`）。
#
# ⚠️ **先读这条再跑**（2026-10-19 实测）：本仓 `source.png` 重采样出的画与**当前入库的
# 六个图标文件不一致**（32/64/128 逐像素最大通道差 194/255/255）⇒ **本脚本不能复现现有图标**。
# 故 PATCH002.59 **只重排了既有 `icon.ico` 的目录帧序**（64 首帧，零画变，见
# `.temp/ico-reorder.ps1` 一次性脚本），**未跑本脚本**。将来要重生成图标时：
# ① 先目测确认 `source.png` 就是目标画（或换成正确设计源）；② 再跑本脚本；③ 跑完核对
# 第 5 步断言与预览图。
#
# 为什么要管"帧序"（PATCH013/PATCH002.59 根因）：Tauri 编译期只取 `icon.ico` 的
# **目录首帧**（`tauri-codegen` 的 `entries()[0]`）当默认窗口图标，且是**单张**位图
# ⇒ 首帧是 16/32 时，任务栏 24px / Alt+Tab 32px、高 DPI 的 40/48px 只能**放大**它 ⇒ 糊；
# 首帧取 64（≥ 任何界面所需尺寸）则全程只缩不放。Windows 走 exe 资源时按尺寸挑帧，帧序无关。
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

# 脚本住在 core/icons/ ⇒ 图标目录 = 脚本自身所在；项目根 = 上两级
$icons = $PSScriptRoot
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$srcPath = Join-Path $icons "source.png"
$src = [System.Drawing.Image]::FromFile($srcPath)
"设计源 = $srcPath（$($src.Width)x$($src.Height)）"

# —— 1. 派生各尺寸 PNG（高质量重采样；不加内边距 = 满幅缩放）——
function New-Scaled([System.Drawing.Image]$image, [int]$size, [string]$out) {
  $bmp = New-Object System.Drawing.Bitmap $size, $size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
  $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
  $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
  $g.Clear([System.Drawing.Color]::Transparent)
  $g.DrawImage($image, (New-Object System.Drawing.Rectangle 0, 0, $size, $size))
  $g.Dispose()
  $bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
}

$targets = @{ "32x32.png" = 32; "64x64.png" = 64; "128x128.png" = 128; "128x128@2x.png" = 256; "icon.png" = 512 }
foreach ($k in $targets.Keys) {
  New-Scaled $src $targets[$k] (Join-Path $icons $k)
  "{0} = {1}x{1}" -f $k, $targets[$k]
}

# —— 2. 多尺寸 ICO（PNG 负载，Vista+ 原生支持）——
# **帧序有语义、勿改回升序**：首帧必须 ≥48（本脚本取 64）
$icoSizes = @(64, 48, 32, 24, 16, 128, 256)
$tmpDir = Join-Path $root ".temp\ico-tmp"
New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
$blobs = @()
foreach ($s in $icoSizes) {
  $p = Join-Path $tmpDir "icon-$s.png"
  New-Scaled $src $s $p
  $blobs += , @{ Size = $s; Bytes = [System.IO.File]::ReadAllBytes($p) }
}

$icoPath = Join-Path $icons "icon.ico"
$fs = [System.IO.File]::Create($icoPath)
$bw = New-Object System.IO.BinaryWriter($fs)
$bw.Write([UInt16]0); $bw.Write([UInt16]1); $bw.Write([UInt16]$blobs.Count)
$offset = 6 + 16 * $blobs.Count
foreach ($b in $blobs) {
  $dim = if ($b.Size -ge 256) { 0 } else { $b.Size }
  $bw.Write([Byte]$dim); $bw.Write([Byte]$dim); $bw.Write([Byte]0); $bw.Write([Byte]0)
  $bw.Write([UInt16]1); $bw.Write([UInt16]32)
  $bw.Write([UInt32]$b.Bytes.Length); $bw.Write([UInt32]$offset)
  $offset += $b.Bytes.Length
}
foreach ($b in $blobs) { $bw.Write($b.Bytes) }
$bw.Flush(); $bw.Dispose(); $fs.Dispose()
Remove-Item $tmpDir -Recurse -Force
"icon.ico 已写入（{0} 帧：{1}）" -f $blobs.Count, ($icoSizes -join "/")

# —— 3. 多尺寸预览图（浅底 / 深底实际尺寸 + 16/24/32 放大 8 倍）——
$bgLight = [System.Drawing.Color]::FromArgb(255, 245, 245, 247)
$bgDark = [System.Drawing.Color]::FromArgb(255, 29, 29, 31)
$w = 980; $h = 430
$pv = New-Object System.Drawing.Bitmap $w, $h, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$g = [System.Drawing.Graphics]::FromImage($pv)
$g.Clear([System.Drawing.Color]::White)
$font = New-Object System.Drawing.Font "Consolas", 11
$showSizes = @(16, 24, 32, 48, 64, 128)
$g.FillRectangle((New-Object System.Drawing.SolidBrush $bgLight), 0, 0, $w, 130)
$x = 20
foreach ($s in $showSizes) {
  $img = New-Object System.Drawing.Bitmap $srcPath
  $g.DrawImage($img, (New-Object System.Drawing.Rectangle $x, (100 - $s), $s, $s))
  $g.DrawString("$s", $font, [System.Drawing.Brushes]::Black, $x, 4)
  $x += [Math]::Max($s, 40) + 24
  $img.Dispose()
}
$g.FillRectangle((New-Object System.Drawing.SolidBrush $bgDark), 0, 140, $w, 130)
$x = 20
foreach ($s in $showSizes) {
  $img = New-Object System.Drawing.Bitmap $srcPath
  $g.DrawImage($img, (New-Object System.Drawing.Rectangle $x, (240 - $s), $s, $s))
  $g.DrawString("$s", $font, [System.Drawing.Brushes]::White, $x, 144)
  $x += [Math]::Max($s, 40) + 24
  $img.Dispose()
}
$g.Dispose()
$pvPath = Join-Path $root ".temp\icon-preview.png"
$pv.Save($pvPath, [System.Drawing.Imaging.ImageFormat]::Png)
$pv.Dispose()
$src.Dispose()
"预览图 = $pvPath"

# —— 4. 校验：**必须校验"目录首帧"**（= Tauri 取作窗口图标的那一帧），
# 不要用 .NET 的 Icon 默认帧——它按自己的默认尺寸挑帧，曾致"默认帧 16x16 无害"误判 ——
$bytes = [System.IO.File]::ReadAllBytes($icoPath)
$frameCount = [BitConverter]::ToUInt16($bytes, 4)
$head = $bytes[6]; if ($head -eq 0) { $head = 256 }
$order = @()
for ($i = 0; $i -lt $frameCount; $i++) {
  $o = 6 + $i * 16; $wd = $bytes[$o]; if ($wd -eq 0) { $wd = 256 }; $order += $wd
}
"=== core/icons 产物 ==="
Get-ChildItem $icons -File | Select-Object Name, Length | Format-Table -AutoSize
"icon.ico 帧序 = {0}；目录首帧 = {1}x{1}（须 ≥ 48，否则任务栏会放大它）" -f ($order -join "/"), $head
if ($head -lt 48) { throw "目录首帧 < 48（$head），窗口图标会发糊 ✗" }
"OK：目录首帧 ≥ 48 ✓"
