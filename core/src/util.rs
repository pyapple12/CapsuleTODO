//! 通用小工具（PL027）：跨域共用的纯工具，从**域专属文件**（`commands/bubble.rs`）
//! 抽出——避免其它域去依赖"气泡的文件"。本模块不含业务语义：本地时间戳 / 日期换算 /
//! 系统默认程序打开文件。
//!
//! 形态：`open_path_with_default` 含平台分叉（Windows `ShellExecuteW` + 非 Windows
//! 空桩），**平台知识收在本文件内 → 调用点零 `#[cfg]`**（沿 PL026 形态）；其余为纯逻辑。
//! 边界（PL027 用户定案）：**独立文件**（`storage.rs` / `clipboard_image.rs`）被两域调用
//! 即正确形态、一行不动；只有**域专属文件**里的通用件才抽到这里。

use std::path::Path;

/// 本地时间戳 `yyMMddHHmmss`（PL025 命名与标签共用）：Windows GetLocalTime 直连取本地
/// 时区（零依赖，kernel32 薄壳同 hotkey/capture FFI 先例）；非 Windows 退 UTC civil 推算
#[cfg(target_os = "windows")]
pub fn local_stamp() -> String {
    #[repr(C)]
    struct SystemTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        millis: u16,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetLocalTime(out: *mut SystemTime);
    }
    let mut t = SystemTime {
        year: 0,
        month: 0,
        day_of_week: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
        millis: 0,
    };
    unsafe { GetLocalTime(&mut t) };
    format!(
        "{:02}{:02}{:02}{:02}{:02}{:02}",
        t.year % 100,
        t.month,
        t.day,
        t.hour,
        t.minute,
        t.second
    )
}

/// epoch 天数 → (年, 月, 日)（Howard Hinnant civil 算法，延后平台 UTC 标签用）
#[cfg(not(target_os = "windows"))]
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 本地时间戳（非 Windows：UTC 推算，延后平台占位实现）
#[cfg(not(target_os = "windows"))]
pub fn local_stamp() -> String {
    let secs = crate::storage::system_now() / 1000;
    let sod = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(secs.div_euclid(86_400));
    format!(
        "{:02}{:02}{:02}{:02}{:02}{:02}",
        y % 100,
        m,
        d,
        sod / 3600,
        (sod % 3600) / 60,
        sod % 60
    )
}

/// 用系统默认程序打开文件（PL025）：Windows `ShellExecuteW`（返回 >32 = 成功约定）；
/// 非 Windows 报暂不支持（延后基线）
#[cfg(target_os = "windows")]
pub fn open_path_with_default(path: &Path) -> Result<(), String> {
    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteW(
            hwnd: isize,
            op: *const u16,
            file: *const u16,
            params: *const u16,
            dir: *const u16,
            show: i32,
        ) -> isize;
    }
    let wide = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };
    let op = wide("open");
    let file = wide(&path.to_string_lossy());
    const SW_SHOWNORMAL: i32 = 1;
    let ret = unsafe {
        ShellExecuteW(
            0,
            op.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if ret <= 32 {
        return Err(format!("系统打开失败（ShellExecute 返回 {ret}）"));
    }
    Ok(())
}

/// 非 Windows：打开原图暂不支持
#[cfg(not(target_os = "windows"))]
pub fn open_path_with_default(_path: &Path) -> Result<(), String> {
    Err("当前平台暂不支持".to_string())
}
