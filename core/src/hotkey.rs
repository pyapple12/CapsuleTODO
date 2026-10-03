//! 全局气泡热键：组合键字符串解析/规范化 + Win32 注册运行时（PL015）。
//! 纯逻辑段（parse/to_display）禁 tauri 依赖，cargo test 直测；运行时段
//! （cfg windows）RegisterHotKey + GetMessageW 循环线程，WM_HOTKEY 在热键线程
//! 直调捕获回调（未走 run_on_main_thread，行为等价——见运行时段实现注记）。
//! 设置变更经 reregister 热替换。

use thiserror::Error;

/// Win32 RegisterHotKey 修饰键位（对齐 winuser.h）
pub const MOD_ALT: u32 = 0x1;
pub const MOD_CONTROL: u32 = 0x2;
pub const MOD_SHIFT: u32 = 0x4;
pub const MOD_WIN: u32 = 0x8;

/// 热键错误（跨 IPC：经 `From<HotkeyError>` 转 CommandError 承载可读串，本类型不直接序列化）
#[derive(Debug, Error)]
pub enum HotkeyError {
    #[error("热键组合为空")]
    Empty,
    #[error("缺少修饰键（Ctrl/Alt/Shift/Win 至少其一）")]
    NoModifier,
    #[error("缺少主键（字母/数字/F1-F12）")]
    MissingMainKey,
    #[error("未知按键：{0}")]
    UnknownKey(String),
    #[error("重复段：{0}")]
    Duplicate(String),
}

/// 组合键：Win32 修饰位掩码 + 主键虚拟键码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotkeyCombo {
    /// 修饰键位掩码（MOD_* 位或）
    pub mods: u32,
    /// 主键虚拟键码（'A'-'Z' / '0'-'9' / VK_F1-F12 = 0x70-0x7B）
    pub vk: u32,
}

/// 解析组合键字符串："Ctrl+Alt+C" 形态——"+"分段，修饰键（Ctrl/Control/Alt/Shift/
/// Win/Super）任意组合但必须非空、顺序无关、大小写无关；主键单字符 A-Z/0-9 或
/// F1-F12，且必须恰有一个。空串/空段/无修饰/无主键/未知键/重复段各自报错
pub fn parse(s: &str) -> Result<HotkeyCombo, HotkeyError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(HotkeyError::Empty);
    }
    let mut mods = 0u32;
    let mut vk: Option<u32> = None;
    for part in trimmed.split('+') {
        let key = part.trim().to_ascii_uppercase();
        if key.is_empty() {
            return Err(HotkeyError::Empty);
        }
        let mod_bit = match key.as_str() {
            "CTRL" | "CONTROL" => Some(MOD_CONTROL),
            "ALT" => Some(MOD_ALT),
            "SHIFT" => Some(MOD_SHIFT),
            "WIN" | "SUPER" => Some(MOD_WIN),
            _ => None,
        };
        if let Some(bit) = mod_bit {
            if mods & bit != 0 {
                return Err(HotkeyError::Duplicate(part.trim().to_string()));
            }
            mods |= bit;
        } else {
            if vk.is_some() {
                return Err(HotkeyError::Duplicate(part.trim().to_string()));
            }
            vk = Some(
                parse_vk(&key).ok_or_else(|| HotkeyError::UnknownKey(part.trim().to_string()))?,
            );
        }
    }
    let vk = vk.ok_or(HotkeyError::MissingMainKey)?;
    if mods == 0 {
        return Err(HotkeyError::NoModifier);
    }
    Ok(HotkeyCombo { mods, vk })
}

/// 单键名 → 虚拟键码（A-Z / 0-9 单字符；F1-F12 = VK_F1 0x70 起）
fn parse_vk(key: &str) -> Option<u32> {
    let bytes = key.as_bytes();
    if bytes.len() == 1 {
        let b = bytes[0];
        if b.is_ascii_uppercase() || b.is_ascii_digit() {
            return Some(b as u32);
        }
        return None;
    }
    if let Some(n) = key.strip_prefix('F') {
        if let Ok(n) = n.parse::<u32>() {
            if (1..=12).contains(&n) {
                return Some(0x70 + n - 1);
            }
        }
    }
    None
}

/// 规范化显示串：修饰键固定序 Ctrl+Alt+Shift+Win，主键按虚拟键码还原
/// （A-Z/0-9 还原字符，0x70-0x7B 还原 F1-F12）
pub fn to_display(combo: &HotkeyCombo) -> String {
    let mut parts: Vec<String> = Vec::new();
    if combo.mods & MOD_CONTROL != 0 {
        parts.push("Ctrl".to_string());
    }
    if combo.mods & MOD_ALT != 0 {
        parts.push("Alt".to_string());
    }
    if combo.mods & MOD_SHIFT != 0 {
        parts.push("Shift".to_string());
    }
    if combo.mods & MOD_WIN != 0 {
        parts.push("Win".to_string());
    }
    parts.push(vk_name(combo.vk));
    parts.join("+")
}

/// 虚拟键码 → 键名（F1-F12 / 单字符）
fn vk_name(vk: u32) -> String {
    if (0x70..=0x7B).contains(&vk) {
        return format!("F{}", vk - 0x70 + 1);
    }
    (vk as u8 as char).to_string()
}

// —— Win32 注册运行时（PL015.4，仅 Windows）——
// 线程模型：专用线程 RegisterHotKey（hwnd=NULL=线程热键）+ GetMessageW 循环；
// WM_HOTKEY → 存储的回调在热键线程直调执行捕获（实现注记：未走 run_on_main_thread
// ——捕获流程仅读剪贴板 + 入库，AppContext 有锁保护、剪贴板插件跨线程安全，行为
// 等价，沿 PL003.3 先例）；换档 = PostThreadMessage
// WM_QUIT 杀旧线程 + JoinHandle 同步等待退出 → 新线程注册（mpsc 回传注册结果，
// 命令层同步得知成功/失败——失败由命令层回滚落库旧值）。

/// 全局热键 ID（RegisterHotKey 的 id 参数；单热键应用固定 1）
const HOTKEY_ID: i32 = 1;
const WM_HOTKEY: u32 = 0x0312;
const WM_QUIT: u32 = 0x0012;

/// 热键线程 id（0 = 未运行）；reregister 据此投递 WM_QUIT 杀旧线程
#[cfg(target_os = "windows")]
static THREAD_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// 注册代际计数（FIX009.1 替代 FIX008.13 的 bool 放弃标志）：reregister 进入即
/// fetch_add 捕获本轮代际，线程双查比对"世界是否还是我那一轮"——不等 = 父侧已
/// 超时放弃或已被后续轮（含回滚轮）取代，自行 Unregister 退出。代际只增不减，
/// "复位"语义消失：bool 版的复位会被紧随超时的回滚轮抹掉，令 stall 线程错过
/// 放弃信号 → 与回滚轮新线程双热键并存（A009 P3-1）；代际比对天然隔离各轮
#[cfg(target_os = "windows")]
static REREGISTER_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 热键触发回调（捕获流程，热键线程直调——线程模型见运行时段实现注记）；spawn 时设置，reregister 复用
#[cfg(target_os = "windows")]
static ON_HOTKEY: std::sync::Mutex<Option<Box<dyn Fn() + Send>>> = std::sync::Mutex::new(None);

#[cfg(target_os = "windows")]
mod win {
    /// MSG 最小字段集（GetMessageW 出参；pt 坐标合并为两个 i32）
    #[repr(C)]
    pub struct Msg {
        pub hwnd: isize,
        pub message: u32,
        pub w_param: usize,
        pub l_param: isize,
        pub time: u32,
        pub pt_x: i32,
        pub pt_y: i32,
    }

    impl Msg {
        pub fn zeroed() -> Self {
            Self {
                hwnd: 0,
                message: 0,
                w_param: 0,
                l_param: 0,
                time: 0,
                pt_x: 0,
                pt_y: 0,
            }
        }
    }

    #[link(name = "user32")]
    extern "system" {
        pub fn RegisterHotKey(hwnd: isize, id: i32, mods: u32, vk: u32) -> i32;
        pub fn UnregisterHotKey(hwnd: isize, id: i32) -> i32;
        pub fn GetMessageW(msg: *mut Msg, hwnd: isize, min: u32, max: u32) -> i32;
        pub fn PostThreadMessageW(tid: u32, msg: u32, w_param: usize, l_param: isize) -> i32;
        pub fn GetCurrentThreadId() -> u32;
    }
}

/// 设置热键触发回调（spawn 前调用一次；后续 reregister 复用）
#[cfg(target_os = "windows")]
pub fn set_on_hotkey(callback: Box<dyn Fn() + Send>) {
    let mut slot = ON_HOTKEY.lock().unwrap_or_else(|e| e.into_inner());
    *slot = Some(callback);
}

/// 启动（或替换）全局热键线程：注册结果经 mpsc 同步回传（命令层可同步失败回滚）。
/// 旧线程经 WM_QUIT 优雅退出并轮询等待；新线程注册失败立即报错（热键被占等）。
/// FIX004.7 四点加固：tid 先落位再回传 / PostThreadMessageW 检查返回值 / 超时报错
/// 不带病推进 / 退出清零用 CAS 只在自己仍是登记者时清
#[cfg(target_os = "windows")]
pub fn reregister(combo: HotkeyCombo) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    use std::sync::mpsc;
    use std::time::Duration;

    // 1. 杀旧线程（无旧线程则跳过）并轮询等待退出——旧线程退出循环时 Unregister
    //    热键并自行 CAS 清零 tid（消息循环退出路径配套，FIX009.10 改语义描述免
    //    行号随插行漂移），父侧只投递 WM_QUIT 不代清
    //    （FIX007.3：原 swap(0) 先行清零令等待条件恒假 = 死等待，且剥夺旧线程
    //    CAS 成功权致 Unregister/Register 竞窗）
    // 0. 捕获本轮代际（FIX009.1）：fetch_add 令旧轮线程的双查比对立即失配（含
    //    上一轮超时未死的 stall 线程——bool 版复位会被紧随的回滚轮抹掉信号），
    //    无需独立复位步
    let my_gen = REREGISTER_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let old = THREAD_ID.load(Ordering::SeqCst);
    if old != 0 {
        // 投递失败短重试（目标线程消息队列未建时投递会失败，FIX004.7-②）
        for _ in 0..10 {
            let sent = unsafe { win::PostThreadMessageW(old, WM_QUIT, 0, 0) };
            if sent != 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let deadline = std::time::Instant::now() + Duration::from_millis(500);
        while THREAD_ID.load(Ordering::SeqCst) == old && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        if THREAD_ID.load(Ordering::SeqCst) == old {
            return Err("旧热键线程退出等待超时，放弃替换以防热键残留".to_string());
        }
    }

    // 2. 新线程：注册 → tid 落位 → 回传结果（FIX004.7-①：tid 先登记再回传，防
    //    "成功已返回而 tid 未登记"窗口被并发 reregister 击穿）→ 消息循环
    let (tx, rx) = mpsc::channel::<Result<(), String>>();
    std::thread::Builder::new()
        .name("bubble-hotkey".to_string())
        .spawn(move || {
            // my_gen 已被 move 闭包捕获（发起轮代际副本），双查比对全局计数
            unsafe {
                if win::RegisterHotKey(0, HOTKEY_ID, combo.mods, combo.vk) == 0 {
                    let _ = tx.send(Err(
                        "RegisterHotKey 失败（热键可能被其它程序占用）".to_string()
                    ));
                    return;
                }
                // FIX009.1 双查之一：世界已不是发起轮（父超时放弃或后续轮接管）
                // ——自行注销退出，不登记 tid 不进循环（"父已回退"与"新键生效"互斥）
                if REREGISTER_GEN.load(Ordering::SeqCst) != my_gen {
                    let _ = win::UnregisterHotKey(0, HOTKEY_ID);
                    let _ = tx.send(Err("父侧等待超时已放弃，注册线程自行注销".to_string()));
                    return;
                }
                // tid 落位改 CAS（FIX009.1）：预期 0 写自己——并发登记失败 = 已有
                // 新轮接管，按放弃路径自清退出（plain store 会覆盖新轮登记）
                if THREAD_ID
                    .compare_exchange(
                        0,
                        win::GetCurrentThreadId(),
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    )
                    .is_err()
                {
                    let _ = win::UnregisterHotKey(0, HOTKEY_ID);
                    let _ = tx.send(Err("并发登记冲突，注册线程自行注销".to_string()));
                    return;
                }
                let _ = tx.send(Ok(()));
                // FIX009.1 双查之二：覆盖"send 与父超时交错"的微秒窗——tid 已
                // 落位须自清（对齐消息循环退出路径的 CAS 形态）
                if REREGISTER_GEN.load(Ordering::SeqCst) != my_gen {
                    let _ = win::UnregisterHotKey(0, HOTKEY_ID);
                    let _ = THREAD_ID.compare_exchange(
                        win::GetCurrentThreadId(),
                        0,
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    );
                    return;
                }
                let mut msg = win::Msg::zeroed();
                // >0 = 收到消息；0 = WM_QUIT；<0 = 错误（错误时退出兜底防死循环）
                while win::GetMessageW(&mut msg, 0, 0, 0) > 0 {
                    if msg.message == WM_HOTKEY && (msg.w_param & 0xFFFF) as i32 == HOTKEY_ID {
                        let slot = ON_HOTKEY.lock().unwrap_or_else(|e| e.into_inner());
                        if let Some(cb) = slot.as_ref() {
                            cb();
                        }
                    }
                }
                let _ = win::UnregisterHotKey(0, HOTKEY_ID);
                // FIX004.7-③：CAS 清零——只在登记仍是自己时清，防迟到退出覆盖新线程登记
                let _ = THREAD_ID.compare_exchange(
                    win::GetCurrentThreadId(),
                    0,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                );
            }
        })
        .map_err(|err| format!("热键线程启动失败：{err}"))?;

    // 3. 同步等注册结果（FIX004.7-④：超时返回 Err，不带病推进）。超时后无需
    //    显式通知线程（FIX009.1）：回滚路径的下一轮 reregister fetch_add 会令
    //    本轮代际失配，stall 线程双查命中自行注销——bool 版"置放弃标志"在此
    //    被回滚轮复位抹掉的缺陷（A009 P3-1）随代际计数根除
    rx.recv_timeout(Duration::from_millis(500))
        .map_err(|_| "热键注册结果等待超时".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_standard_combo() {
        let c = parse("Ctrl+Alt+C").expect("标准组合必须可解析");
        assert_eq!(c.mods, MOD_CONTROL | MOD_ALT);
        assert_eq!(c.vk, b'C' as u32);
        assert_eq!(to_display(&c), "Ctrl+Alt+C");
    }

    #[test]
    fn parse_segment_order_irrelevant() {
        let a = parse("Alt+Ctrl+C").expect("乱序必须可解析");
        let b = parse("c+alt+ctrl").expect("大小写无关");
        assert_eq!(a, b, "乱序与大小写归一");
        assert_eq!(to_display(&a), "Ctrl+Alt+C");
    }

    #[test]
    fn parse_shift_and_win() {
        let c = parse("Win+Shift+F12").expect("Win/Shift/F12 必须可解析");
        assert_eq!(c.mods, MOD_WIN | MOD_SHIFT);
        assert_eq!(c.vk, 0x70 + 11);
        assert_eq!(to_display(&c), "Shift+Win+F12");
    }

    #[test]
    fn parse_digit_main_key() {
        let c = parse("Ctrl+7").expect("数字主键必须可解析");
        assert_eq!(c.vk, b'7' as u32);
    }

    #[test]
    fn parse_rejects_empty() {
        assert!(matches!(parse(""), Err(HotkeyError::Empty)));
        assert!(matches!(parse("Ctrl++C"), Err(HotkeyError::Empty)));
    }

    #[test]
    fn parse_rejects_no_modifier() {
        assert!(matches!(parse("C"), Err(HotkeyError::NoModifier)));
    }

    #[test]
    fn parse_rejects_missing_main_key() {
        assert!(matches!(
            parse("Ctrl+Alt"),
            Err(HotkeyError::MissingMainKey)
        ));
    }

    #[test]
    fn parse_rejects_unknown_key() {
        assert!(matches!(
            parse("Ctrl+Space"),
            Err(HotkeyError::UnknownKey(_))
        ));
    }

    #[test]
    fn parse_rejects_duplicate_segment() {
        assert!(matches!(
            parse("Ctrl+Ctrl+C"),
            Err(HotkeyError::Duplicate(_))
        ));
    }

    #[test]
    fn roundtrip_is_identity() {
        for text in ["Ctrl+Alt+C", "Win+Shift+F1", "Ctrl+5"] {
            let c = parse(text).expect("必须可解析");
            let display = to_display(&c);
            let reparsed = parse(&display).expect("显示串必须可回解析");
            assert_eq!(c, reparsed, "往返恒等：{text}");
        }
    }
}
