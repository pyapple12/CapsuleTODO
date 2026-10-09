//! 剪贴板图片读写（PL024；读链三级化 PL024.8b）：纯逻辑编解码段（跨平台 cargo test
//! 全测）+ Win32 FFI 薄壳段（cfg windows）。读 = PNG 注册格式主路径（截图工具原厂
//! 字节零转换）→ CF_DIB 回退（24/32bpp BI_RGB/BI_BITFIELDS → PNG 编码，罕见变体严格
//! 报不支持）→ CF_HDROP 文件引用回退（选中/复制的单个图像文件读盘转 PNG，PL024.8b）；
//! 写 = PNG 注册格式 + CF_DIB + CF_DIBV5 三格式（= Win+Shift+S 原厂写入集，像素
//! 同源仅头结构差异，alpha 全链无损；CF_BITMAP 由剪贴板自动合成不手写）。
//! base64 手写标准实现（详情板 data URL，零第二新依赖）。

use crate::bubble::BubbleError;

/// CF_HDROP 源文件字节上限（防呆闸口：读盘前按元数据长度拦截，PL024.8b，可调）
pub const MAX_SOURCE_FILE_BYTES: u64 = 20 * 1024 * 1024;

/// 认可的图像文件扩展名（CF_HDROP 文件分支；表驱动 + 小写比对，PL024.8b）
const IMAGE_FILE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp"];

/// 剪贴板图片读写失败错误族（写方向严格报错，经 CommandError::Clipboard 跨进程可见）
#[derive(Debug, thiserror::Error)]
pub enum ClipboardImageError {
    /// 剪贴板打开失败（占用竞争）
    #[error("剪贴板打开失败")]
    Open,
    /// 剪贴板清空失败
    #[error("剪贴板清空失败")]
    Empty,
    /// PNG 注册格式注册失败
    #[error("PNG 剪贴板格式注册失败")]
    RegisterFormat,
    /// 图片编码失败（CF_DIB/CF_DIBV5 构建）
    #[error("图片编码失败：{0}")]
    Encode(#[from] BubbleError),
    /// 某格式 SetClipboardData 失败（携带格式名定位）
    #[error("剪贴板写入失败（{0}）")]
    Set(&'static str),
    /// 当前平台不支持图片剪贴板（macOS/Linux 延后基线，PL024 定案）
    #[error("当前平台暂不支持图片剪贴板")]
    UnsupportedPlatform,
}

// —— 纯逻辑段（跨平台）——

/// BITMAPINFOHEADER 压缩类型：无压缩
const BI_RGB: u32 = 0;
/// BITMAPINFOHEADER 压缩类型：显式掩码位域
const BI_BITFIELDS: u32 = 3;
/// 标准 DIB 信息头尺寸（字节）
const BMP_INFO_HEADER: usize = 40;
/// V5 位图信息头尺寸（字节，PL024 写方向）
const BMP_V5_HEADER: usize = 124;
/// LCS_sRGB 颜色空间标记（'BGRs' 小端；DIBV5 头 bV5CSType 惯用值）
const LCS_SRGB: u32 = 0x7352_4742;
/// LCS_GM_IMAGES 颜色匹配意图（DIBV5 头 bV5Intent 惯用值）
const LCS_GM_IMAGES: u32 = 4;

fn rd_u16(d: &[u8], at: usize) -> Option<u16> {
    if d.len() < at + 2 {
        return None;
    }
    Some(u16::from_le_bytes([d[at], d[at + 1]]))
}

fn rd_u32(d: &[u8], at: usize) -> Option<u32> {
    if d.len() < at + 4 {
        return None;
    }
    Some(u32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]]))
}

fn wr_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn wr_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// DIB 解析结果（受支持形态的规范化参数）
struct DibLayout {
    width: u32,
    height: u32,
    /// true = 自顶向下（负高）；false = 自下而上（经典正高，行序需翻转）
    top_down: bool,
    bpp: u16,
    /// 像素数据起始偏移（头 + BITFIELDS 掩码段之后）
    px_off: usize,
}

/// 解析 CF_DIB 头并校验受支持形态：仅 40B 标准头、24/32bpp、BI_RGB/BI_BITFIELDS
/// 且掩码为规范 BGRA 组合；其余（16bpp/调色板/RLE/非规范掩码）一律 None → 上层报
/// UnsupportedFormat（严格抛错主线，PL024 定案）
fn parse_dib(dib: &[u8]) -> Option<DibLayout> {
    if rd_u32(dib, 0)? != BMP_INFO_HEADER as u32 {
        return None;
    }
    let width = rd_u32(dib, 4)? as i32;
    if width <= 0 {
        return None;
    }
    let height_raw = rd_u32(dib, 8)? as i32;
    if height_raw == 0 {
        return None;
    }
    let top_down = height_raw < 0;
    let bpp = rd_u16(dib, 14)?;
    let compression = rd_u32(dib, 16)?;
    if !matches!(bpp, 24 | 32) {
        return None;
    }
    let canonical = (0x00FF_0000, 0x0000_FF00, 0x0000_00FF);
    let px_off = if compression == BI_BITFIELDS {
        let (r, g, b) = (rd_u32(dib, 40)?, rd_u32(dib, 44)?, rd_u32(dib, 48)?);
        // 40B 标准头只有三个掩码位（alpha 无槽位——那是 DIBV5 的存在理由），
        // 像素起于 52；32bpp alpha 固定按 0xFF000000 直通
        if bpp == 24 || (r, g, b) != canonical {
            return None;
        }
        52
    } else if compression == BI_RGB {
        40
    } else {
        return None;
    };
    Some(DibLayout {
        width: width as u32,
        height: height_raw.unsigned_abs(),
        top_down,
        bpp,
        px_off,
    })
}

/// CF_DIB 字节 → PNG 编码（读方向回退路径，PL024）。像素按行序归位 BGR(A)→RGBA，
/// 截断/溢出一律 UnsupportedFormat
pub fn encode_png_from_dib(dib: &[u8]) -> Result<Vec<u8>, BubbleError> {
    let bad = || BubbleError::UnsupportedFormat;
    let layout = parse_dib(dib).ok_or_else(bad)?;
    let stride = (layout.width * layout.bpp as u32).div_ceil(32) as usize * 4;
    let total = layout
        .px_off
        .checked_add(stride.checked_mul(layout.height as usize).ok_or_else(bad)?)
        .ok_or_else(bad)?;
    if dib.len() < total {
        return Err(bad());
    }
    let bytes_pp = (layout.bpp / 8) as usize;
    let (w, h) = (layout.width as usize, layout.height as usize);
    let mut rgba = vec![0u8; w * h * 4];
    for y in 0..h {
        // 正高 = 自下而上：源行 h-1-y 对齐图像行 y
        let src_row = if layout.top_down { y } else { h - 1 - y };
        for x in 0..w {
            let s = layout.px_off + src_row * stride + x * bytes_pp;
            let d = (y * w + x) * 4;
            rgba[d] = dib[s + 2]; // R
            rgba[d + 1] = dib[s + 1]; // G
            rgba[d + 2] = dib[s]; // B
            rgba[d + 3] = if bytes_pp == 4 { dib[s + 3] } else { 255 };
        }
    }
    encode_rgba_to_png(w as u32, h as u32, rgba)
}

/// 原始 RGBA 像素 → PNG 编码（读/文件路径与写路径共用取数口，PL024.8b 抽取）
fn encode_rgba_to_png(w: u32, h: u32, rgba: Vec<u8>) -> Result<Vec<u8>, BubbleError> {
    let bad = || BubbleError::UnsupportedFormat;
    let img =
        image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_raw(w, h, rgba).ok_or_else(bad)?;
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|_| bad())?;
    Ok(png)
}

/// 图像字节 → 原始 RGBA（通用解码入口：image crate 按魔数自适应，PL024.8b 由
/// decode_png_to_rgba 泛化）；解码失败严格报 UnsupportedFormat
pub fn decode_to_rgba(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), BubbleError> {
    let img = image::load_from_memory(bytes).map_err(|_| BubbleError::UnsupportedFormat)?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok((w, h, rgba.into_raw()))
}

/// 行像素 RGBA → BGRA（直通 alpha）自下而上序：DIB/DIBV5 两构建器共用像素段
fn bottom_up_bgra(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    let stride = (w as usize) * 4;
    let mut px = vec![0u8; stride * h as usize];
    for y in 0..h as usize {
        let src = &rgba[y * stride..(y + 1) * stride];
        // 自下而上：图像底行写入最前
        let dst = &mut px[(h as usize - 1 - y) * stride..(h as usize - y) * stride];
        for x in 0..w as usize {
            dst[x * 4] = src[x * 4 + 2]; // B
            dst[x * 4 + 1] = src[x * 4 + 1]; // G
            dst[x * 4 + 2] = src[x * 4]; // R
            dst[x * 4 + 3] = src[x * 4 + 3]; // A 直通
        }
    }
    px
}

/// PNG → CF_DIB（32bpp BI_BITFIELDS 正高自下而上 + 规范 BGRA 掩码 + 直通 alpha）
pub fn build_dib_from_png(png: &[u8]) -> Result<Vec<u8>, BubbleError> {
    let (w, h, rgba) = decode_to_rgba(png)?;
    let px = bottom_up_bgra(w, h, &rgba);
    let mut out = Vec::with_capacity(52 + px.len());
    wr_u32(&mut out, BMP_INFO_HEADER as u32);
    wr_u32(&mut out, w);
    wr_u32(&mut out, h); // 正高 = 自下而上
    wr_u16(&mut out, 1); // planes
    wr_u16(&mut out, 32); // bpp
    wr_u32(&mut out, BI_BITFIELDS);
    wr_u32(&mut out, px.len() as u32); // sizeimage
    wr_u32(&mut out, 0); // x ppm
    wr_u32(&mut out, 0); // y ppm
    wr_u32(&mut out, 0); // clrused
    wr_u32(&mut out, 0); // clrimportant
    wr_u32(&mut out, 0x00FF_0000); // R
    wr_u32(&mut out, 0x0000_FF00); // G
    wr_u32(&mut out, 0x0000_00FF); // B
    out.extend_from_slice(&px);
    Ok(out)
}

/// PNG → CF_DIBV5（124B V5 头带显式 alpha 掩码，像素与 CF_DIB 同源；读 alpha 的
/// 现代应用据此正确呈现透明——PL024"一期做全"定案）
pub fn build_dibv5_from_png(png: &[u8]) -> Result<Vec<u8>, BubbleError> {
    let (w, h, rgba) = decode_to_rgba(png)?;
    let px = bottom_up_bgra(w, h, &rgba);
    let mut out = Vec::with_capacity(BMP_V5_HEADER + px.len());
    wr_u32(&mut out, BMP_V5_HEADER as u32);
    wr_u32(&mut out, w);
    wr_u32(&mut out, h);
    wr_u16(&mut out, 1); // planes
    wr_u16(&mut out, 32); // bpp
    wr_u32(&mut out, BI_BITFIELDS);
    wr_u32(&mut out, px.len() as u32);
    wr_u32(&mut out, 0); // x ppm
    wr_u32(&mut out, 0); // y ppm
    wr_u32(&mut out, 0); // clrused
    wr_u32(&mut out, 0); // clrimportant
    wr_u32(&mut out, 0x00FF_0000); // bV5RedMask
    wr_u32(&mut out, 0x0000_FF00); // bV5GreenMask
    wr_u32(&mut out, 0x0000_00FF); // bV5BlueMask
    wr_u32(&mut out, 0xFF00_0000); // bV5AlphaMask
    wr_u32(&mut out, LCS_SRGB); // bV5CSType
    out.extend_from_slice(&[0u8; 48]); // bV5Endpoints（CIEXYZTRIPLE 36B）+ 三 gamma（12B），全零合法
    wr_u32(&mut out, LCS_GM_IMAGES); // bV5Intent
    wr_u32(&mut out, 0); // bV5ProfileData
    wr_u32(&mut out, 0); // bV5ProfileSize
    wr_u32(&mut out, 0); // bV5Reserved
    out.extend_from_slice(&px);
    Ok(out)
}

/// 标准 base64 编码（RFC 4648 字母表 + padding；手写 ~20 行避免第二新依赖，
/// 详情板 data URL 专用）
pub fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// 文件名是否为认可的图像文件（扩展名表驱动、大小写不敏感；Path 取末段扩展名，
/// 免疫目录名带点，PL024.8b）
pub fn is_image_extension(name: &str) -> bool {
    match std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
    {
        Some(ext) => {
            let lower = ext.to_ascii_lowercase();
            IMAGE_FILE_EXTENSIONS.contains(&lower.as_str())
        }
        None => false,
    }
}

/// 源文件字节数是否在上限内（CF_HDROP 分支读盘前闸口，PL024.8b）
pub fn source_file_within_limit(len: u64) -> bool {
    len <= MAX_SOURCE_FILE_BYTES
}

/// 图像文件字节 → PNG（可入库形态）：已是 PNG 直通原字节（零转换）；其余格式经通用
/// 解码转 PNG；解码失败严格报 UnsupportedFormat（PL024.8b）
pub fn image_file_to_png(bytes: &[u8]) -> Result<Vec<u8>, BubbleError> {
    if bytes.starts_with(&crate::bubble::PNG_MAGIC) {
        return Ok(bytes.to_vec());
    }
    let (w, h, rgba) = decode_to_rgba(bytes)?;
    encode_rgba_to_png(w, h, rgba)
}

// —— Win32 FFI 薄壳段（仅 Windows）——

#[cfg(target_os = "windows")]
mod win {
    use super::{build_dib_from_png, build_dibv5_from_png, ClipboardImageError};

    // FFI 文件级单源声明（FIX013.8 先例：集中于本模块头部，调用点零散落）
    #[link(name = "user32")]
    extern "system" {
        fn OpenClipboard(hwnd_new_owner: isize) -> i32;
        fn CloseClipboard() -> i32;
        fn EmptyClipboard() -> i32;
        fn IsClipboardFormatAvailable(format: u32) -> i32;
        fn GetClipboardData(format: u32) -> isize;
        fn SetClipboardData(format: u32, mem: isize) -> isize;
        fn RegisterClipboardFormatW(lpsz_format: *const u16) -> u32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalAlloc(flags: u32, bytes: usize) -> isize;
        fn GlobalLock(mem: isize) -> *mut u8;
        fn GlobalUnlock(mem: isize) -> i32;
        fn GlobalSize(mem: isize) -> usize;
        fn GlobalFree(mem: isize) -> isize;
    }
    #[link(name = "shell32")]
    extern "system" {
        /// 查询拖放文件列表：iFile=0xFFFFFFFF 取文件个数；具体项取路径（cch=0 返回所需字符数）
        fn DragQueryFileW(hdrop: isize, ifile: u32, lpszfile: *mut u16, cch: u32) -> u32;
    }

    const GMEM_MOVEABLE: u32 = 0x0002;
    /// 标准剪贴板格式：设备无关位图
    const CF_DIB: u32 = 8;
    /// 标准剪贴板格式：V5 位图（带 alpha 通道语义）
    const CF_DIBV5: u32 = 17;
    /// 标准剪贴板格式：文件引用（拖放）列表（PL024.8b）
    const CF_HDROP: u32 = 15;

    /// "PNG" 注册格式 ID（同一会话注册幂等同值；0 = 失败）
    fn png_format() -> u32 {
        let name: Vec<u16> = "PNG\0".encode_utf16().collect();
        unsafe { RegisterClipboardFormatW(name.as_ptr()) }
    }

    /// 拷贝剪贴板某格式数据为自有字节（须已 OpenClipboard）；None = 格式缺席/取数失败
    fn copy_format(format: u32) -> Option<Vec<u8>> {
        unsafe {
            if IsClipboardFormatAvailable(format) == 0 {
                return None;
            }
            let handle = GetClipboardData(format);
            if handle == 0 {
                return None;
            }
            let size = GlobalSize(handle);
            let ptr = GlobalLock(handle);
            if ptr.is_null() {
                return None;
            }
            let data = std::slice::from_raw_parts(ptr, size).to_vec();
            GlobalUnlock(handle);
            Some(data)
        }
    }

    /// 分配全局内存并写入剪贴板（成功后系统接管句柄；失败路径自行 GlobalFree 防泄漏）
    fn set_bytes(format: u32, bytes: &[u8]) -> Result<(), ()> {
        unsafe {
            let handle = GlobalAlloc(GMEM_MOVEABLE, bytes.len());
            if handle == 0 {
                return Err(());
            }
            let ptr = GlobalLock(handle);
            if ptr.is_null() {
                GlobalFree(handle);
                return Err(());
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
            GlobalUnlock(handle);
            if SetClipboardData(format, handle) == 0 {
                GlobalFree(handle);
                return Err(());
            }
            Ok(())
        }
    }

    /// 读剪贴板文件引用路径（须已 OpenClipboard）：仅当恰好一个文件时返回其路径
    /// （多选忽略防误批量，PL024.8b）
    fn read_drop_path() -> Option<String> {
        unsafe {
            if IsClipboardFormatAvailable(CF_HDROP) == 0 {
                return None;
            }
            let handle = GetClipboardData(CF_HDROP);
            if handle == 0 {
                return None;
            }
            // 0xFFFFFFFF = 查询文件个数；只认恰好一个
            if DragQueryFileW(handle, 0xFFFF_FFFF, std::ptr::null_mut(), 0) != 1 {
                return None;
            }
            let len = DragQueryFileW(handle, 0, std::ptr::null_mut(), 0);
            if len == 0 {
                return None;
            }
            let mut buf = vec![0u16; len as usize + 1];
            let got = DragQueryFileW(handle, 0, buf.as_mut_ptr(), buf.len() as u32);
            if got == 0 {
                return None;
            }
            Some(String::from_utf16_lossy(&buf[..got as usize]))
        }
    }

    /// 读图像文件转 PNG（须已 CloseClipboard）：扩展名判定 + 元数据长度闸口 + 读盘 +
    /// 转码；IO/解码失败落日志返 None（热键静默定案对齐，PL024.8b）
    fn read_image_file(path: &str) -> Option<Vec<u8>> {
        if !super::is_image_extension(path) {
            return None;
        }
        let meta = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(err) => {
                eprintln!("剪贴板图片读取：文件元数据失败（{err}）");
                return None;
            }
        };
        if !super::source_file_within_limit(meta.len()) {
            eprintln!("剪贴板图片读取：文件超上限（{} 字节）", meta.len());
            return None;
        }
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(err) => {
                eprintln!("剪贴板图片读取：文件读取失败（{err}）");
                return None;
            }
        };
        match super::image_file_to_png(&bytes) {
            Ok(png) => Some(png),
            Err(err) => {
                eprintln!("剪贴板图片读取：文件转 PNG 失败（{err}）");
                None
            }
        }
    }

    /// 读剪贴板图片（三级链：PNG 注册格式原字节直取 → CF_DIB 编码转 PNG → CF_HDROP
    /// 文件引用读盘转 PNG）。打开失败/格式缺席返回 None——热键路径静默、手动路径可见
    /// 报错由调用层分派
    pub fn read_clipboard_image() -> Option<Vec<u8>> {
        if unsafe { OpenClipboard(0) } == 0 {
            eprintln!("剪贴板图片读取：打开失败（占用竞争，下次捕获自愈）");
            return None;
        }
        let png_fmt = png_format();
        let has_png = png_fmt != 0 && unsafe { IsClipboardFormatAvailable(png_fmt) } != 0;
        // 短临界区：只拷原始字节/取文件路径，编码与读盘挪到 CloseClipboard 之后
        let raw = if has_png {
            copy_format(png_fmt)
        } else {
            copy_format(CF_DIB)
        };
        // 三级兜底：PNG/CF_DIB 皆缺席时才探文件引用（截图与文件复制互斥，PL024.8b）
        let drop_path = if has_png || raw.is_some() {
            None
        } else {
            read_drop_path()
        };
        unsafe { CloseClipboard() };
        if has_png {
            return raw;
        }
        if let Some(dib) = raw {
            return match super::encode_png_from_dib(&dib) {
                Ok(png) => Some(png),
                Err(err) => {
                    eprintln!("剪贴板图片读取：CF_DIB 编码失败（{err}）");
                    None
                }
            };
        }
        drop_path.and_then(|path| read_image_file(&path))
    }

    /// 写剪贴板图片三格式（PNG 注册格式 + CF_DIB + CF_DIBV5 = 截图工具原厂写入集；
    /// 各格式独立全局内存分配，系统接管后不需释放）。任一步失败严格报错，不静默
    pub fn write_clipboard_image(png: &[u8]) -> Result<(), ClipboardImageError> {
        if unsafe { OpenClipboard(0) } == 0 {
            return Err(ClipboardImageError::Open);
        }
        let result = (|| {
            if unsafe { EmptyClipboard() } == 0 {
                return Err(ClipboardImageError::Empty);
            }
            let png_fmt = png_format();
            if png_fmt == 0 {
                return Err(ClipboardImageError::RegisterFormat);
            }
            set_bytes(png_fmt, png).map_err(|_| ClipboardImageError::Set("PNG"))?;
            let dib = build_dib_from_png(png)?;
            set_bytes(CF_DIB, &dib).map_err(|_| ClipboardImageError::Set("CF_DIB"))?;
            let v5 = build_dibv5_from_png(png)?;
            set_bytes(CF_DIBV5, &v5).map_err(|_| ClipboardImageError::Set("CF_DIBV5"))?;
            Ok(())
        })();
        unsafe { CloseClipboard() };
        result
    }
}

/// 读剪贴板图片（Windows 实装；其余平台恒 None 回落文本链路）
#[cfg(target_os = "windows")]
pub fn read_clipboard_image() -> Option<Vec<u8>> {
    win::read_clipboard_image()
}

/// 读剪贴板图片（非 Windows 占位：图捕获不可用，热键/手动均回落文本）
#[cfg(not(target_os = "windows"))]
pub fn read_clipboard_image() -> Option<Vec<u8>> {
    None
}

/// 写剪贴板图片（Windows 三格式实装）
#[cfg(target_os = "windows")]
pub fn write_clipboard_image(png: &[u8]) -> Result<(), ClipboardImageError> {
    win::write_clipboard_image(png)
}

/// 写剪贴板图片（非 Windows 明确报暂不支持——延后基线，严格报错非降级）
#[cfg(not(target_os = "windows"))]
pub fn write_clipboard_image(_png: &[u8]) -> Result<(), ClipboardImageError> {
    Err(ClipboardImageError::UnsupportedPlatform)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 独立证据夹具：用 image crate 直接编码一张 2×2 已知像素 PNG（不经本模块
    /// encode 路径，防同源自证——progress-task 验证样例独立化纪律）
    fn fixture_png() -> Vec<u8> {
        let mut img = image::RgbaImage::new(2, 2);
        img.put_pixel(0, 0, image::Rgba([10, 20, 30, 40]));
        img.put_pixel(1, 0, image::Rgba([50, 60, 70, 80]));
        img.put_pixel(0, 1, image::Rgba([90, 100, 110, 120]));
        img.put_pixel(1, 1, image::Rgba([130, 140, 150, 160]));
        let mut out = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .expect("夹具编码必须成功");
        out
    }

    #[test]
    fn base64_rfc4648_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn dib_bgr_bottom_up_to_png_roundtrip() {
        // 2×2 32bpp BI_RGB 自下而上：行 0 = 图像底行；编码后解码比对像素归位
        let px = |b: u8, g: u8, r: u8, a: u8| [b, g, r, a];
        let mut dib = Vec::new();
        wr_u32(&mut dib, 40);
        wr_u32(&mut dib, 2); // width
        wr_u32(&mut dib, 2); // height（正 = 自下而上）
        wr_u16(&mut dib, 1); // planes
        wr_u16(&mut dib, 32); // bpp
        wr_u32(&mut dib, BI_RGB);
        wr_u32(&mut dib, 0); // sizeimage
        wr_u32(&mut dib, 0);
        wr_u32(&mut dib, 0);
        wr_u32(&mut dib, 0);
        wr_u32(&mut dib, 0);
        // 自下而上：先写图像底行
        dib.extend_from_slice(&px(2, 3, 4, 255));
        dib.extend_from_slice(&px(6, 7, 8, 255));
        dib.extend_from_slice(&px(1, 2, 3, 255));
        dib.extend_from_slice(&px(5, 6, 7, 255));
        let png = encode_png_from_dib(&dib).expect("编码必须成功");
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        let (w, h, rgba) = decode_to_rgba(&png).expect("自解码必须成功");
        assert_eq!((w, h), (2, 2));
        // 图像顶行 = 源第二写入行
        assert_eq!(&rgba[0..4], &[3, 2, 1, 255]);
        assert_eq!(&rgba[4..8], &[7, 6, 5, 255]);
        assert_eq!(&rgba[8..12], &[4, 3, 2, 255]);
        assert_eq!(&rgba[12..16], &[8, 7, 6, 255]);
    }

    #[test]
    fn fixture_png_to_dib_and_back() {
        // 独立夹具 PNG → DIB 头字段与像素布局断言 → 回编 PNG 解码像素一致
        let png = fixture_png();
        let dib = build_dib_from_png(&png).expect("构建必须成功");
        assert_eq!(&dib[0..4], &40u32.to_le_bytes(), "biSize = 40");
        assert_eq!(&dib[4..8], &2u32.to_le_bytes());
        assert_eq!(&dib[8..12], &2u32.to_le_bytes(), "正高 = 自下而上");
        assert_eq!(&dib[14..16], &32u16.to_le_bytes());
        assert_eq!(&dib[16..20], &BI_BITFIELDS.to_le_bytes());
        assert_eq!(&dib[40..44], &0x00FF_0000u32.to_le_bytes(), "R 掩码");
        assert_eq!(&dib[44..48], &0x0000_FF00u32.to_le_bytes(), "G 掩码");
        assert_eq!(&dib[48..52], &0x0000_00FFu32.to_le_bytes(), "B 掩码");
        // 像素区自下而上：底行（源 y=1）在前——BGRA 直通 alpha
        assert_eq!(&dib[52..56], &[110, 100, 90, 120]);
        assert_eq!(&dib[60..64], &[30, 20, 10, 40]);
        // 往返闭环：DIB → PNG → 解码像素与夹具一致
        let back = encode_png_from_dib(&dib).expect("回编必须成功");
        let (w, h, rgba) = decode_to_rgba(&back).expect("解码必须成功");
        assert_eq!((w, h), (2, 2));
        assert_eq!(&rgba[0..4], &[10, 20, 30, 40]);
        assert_eq!(&rgba[12..16], &[130, 140, 150, 160]);
    }

    #[test]
    fn build_dibv5_header_and_alpha() {
        let png = fixture_png();
        let v5 = build_dibv5_from_png(&png).expect("构建必须成功");
        assert_eq!(&v5[0..4], &124u32.to_le_bytes(), "biSize = 124");
        assert_eq!(&v5[16..20], &BI_BITFIELDS.to_le_bytes());
        assert_eq!(&v5[40..44], &0x00FF_0000u32.to_le_bytes(), "R 掩码");
        assert_eq!(&v5[52..56], &0xFF00_0000u32.to_le_bytes(), "alpha 掩码");
        assert_eq!(&v5[56..60], &LCS_SRGB.to_le_bytes(), "CSType = sRGB");
        assert_eq!(&v5[108..112], &LCS_GM_IMAGES.to_le_bytes(), "Intent");
        // 像素与 DIB 版同源（V5 头 124B 后起）
        let dib = build_dib_from_png(&png).expect("构建必须成功");
        assert_eq!(v5[124..], dib[52..], "像素数据同源");
    }

    #[test]
    fn unsupported_dib_variants_rejected() {
        let header = |bpp: u16, compression: u32| {
            let mut d = Vec::new();
            wr_u32(&mut d, 40);
            wr_u32(&mut d, 2);
            wr_u32(&mut d, 2);
            wr_u16(&mut d, 1);
            wr_u16(&mut d, bpp);
            wr_u32(&mut d, compression);
            d.resize(64, 0);
            d
        };
        // 16bpp / RLE 压缩 / 非规范掩码 / 零宽 —— 全部 UnsupportedFormat
        assert!(encode_png_from_dib(&header(16, BI_RGB)).is_err());
        assert!(encode_png_from_dib(&header(32, 1)).is_err());
        assert!(encode_png_from_dib(&header(8, BI_RGB)).is_err());
        let mut bad_mask = header(32, BI_BITFIELDS);
        bad_mask[40] = 0x0F; // 非规范 R 掩码
        assert!(encode_png_from_dib(&bad_mask).is_err());
    }

    // —— PL024.8b 文件分支（HDROP 纯逻辑面）——

    /// 独立证据夹具：image crate 直接编码 2×2 无损 BMP（像素精确，非 PNG 分支用）
    fn fixture_bmp() -> Vec<u8> {
        let mut img = image::RgbImage::new(2, 2);
        img.put_pixel(0, 0, image::Rgb([11, 22, 33]));
        img.put_pixel(1, 0, image::Rgb([44, 55, 66]));
        img.put_pixel(0, 1, image::Rgb([77, 88, 99]));
        img.put_pixel(1, 1, image::Rgb([111, 122, 133]));
        let mut out = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Bmp)
            .expect("BMP 夹具编码必须成功");
        out
    }

    #[test]
    fn is_image_extension_truth_table() {
        for name in [
            "a.png",
            "B.JPG",
            "x.jpeg",
            "y.gif",
            "z.webp",
            "w.bmp",
            r"C:\dir.name\pic.PNG",
        ] {
            assert!(is_image_extension(name), "应识别为图像：{name}");
        }
        for name in ["a.txt", "noext", "a.", "", r"C:\dir.png\file"] {
            assert!(!is_image_extension(name), "不应识别为图像：{name}");
        }
    }

    #[test]
    fn source_file_limit_boundary() {
        assert!(source_file_within_limit(MAX_SOURCE_FILE_BYTES));
        assert!(!source_file_within_limit(MAX_SOURCE_FILE_BYTES + 1));
    }

    #[test]
    fn png_file_passes_through_unchanged() {
        let png = fixture_png();
        assert_eq!(image_file_to_png(&png).expect("直通必须成功"), png);
    }

    #[test]
    fn bmp_file_decodes_to_png_pixels_preserved() {
        // 无损 BMP → PNG：解码像素与夹具逐像素一致（JPEG 有损故另测）
        let bmp = fixture_bmp();
        let png = image_file_to_png(&bmp).expect("转 PNG 必须成功");
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        let (w, h, rgba) = decode_to_rgba(&png).expect("解码必须成功");
        assert_eq!((w, h), (2, 2));
        assert_eq!(&rgba[0..4], &[11, 22, 33, 255]);
        assert_eq!(&rgba[12..16], &[111, 122, 133, 255]);
    }

    #[test]
    fn jpeg_file_decodes_to_png() {
        // 有损 JPEG → PNG：只断言可解码且尺寸一致（像素不逐点比对，有损不可精确）
        let mut img = image::RgbImage::new(2, 2);
        for (x, y, p) in img.enumerate_pixels_mut() {
            *p = image::Rgb([(x * 100) as u8, (y * 100) as u8, 128]);
        }
        let mut jpg = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(
                &mut std::io::Cursor::new(&mut jpg),
                image::ImageFormat::Jpeg,
            )
            .expect("JPEG 夹具编码必须成功");
        let png = image_file_to_png(&jpg).expect("JPEG 转 PNG 必须成功");
        let (w, h, _) = decode_to_rgba(&png).expect("解码必须成功");
        assert_eq!((w, h), (2, 2));
    }

    #[test]
    fn garbage_bytes_rejected() {
        assert!(image_file_to_png(b"definitely not an image").is_err());
    }
}
