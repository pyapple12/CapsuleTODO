//! 圈选直达捕获（PL016）：全局热键触发时模拟 Ctrl+C 两段式——快照原剪贴板 →
//! 合成按键 → 轮询等剪贴板变化 → 读新文本入库。纯逻辑段（wait_clipboard_change）
//! 注入时间源与读函数 cargo test 直测零真实等待；运行时段（cfg windows）user32
//! keybd_event 直连零新依赖（沿 fullscreen/hotkey 先例）。
//! 用户定案（2026-09-30）：无选区/复制失败 = 静默（不回退捕获旧剪贴板）；
//! 不设设置板开关。实现注记：编排跑热键线程（沿 PL015.4 直调先例，行为等价）。

/// 轮询步进（毫秒，真实调用；测试注入 0 步进零真实等待）
pub const POLL_INTERVAL_MS: u64 = 10;

/// 等剪贴板变化总超时（毫秒；无选区/管理员窗拒绝合成时到点静默）
pub const WAIT_TIMEOUT_MS: u64 = 300;

/// 等剪贴板出现"非空且不同于快照"的新文本（10ms 步进轮询纯状态机）。
/// 命中判定 trim 后比较（复制工具常加尾随换行，语义不变）；返回原始文本
///（入库校验/去重由 bubble_capture_core 负责，此处不重复裁决）。
///
/// # 参数
/// - `before`：起点快照（None = 原剪贴板无文本；此态下读到任何非空文本即命中）
/// - `timeout_ms`：总等待上限
/// - `step_ms`：轮询步进（真实 10ms；测试传 0 零真实等待）
/// - `now_ms`：单调时钟（毫秒），测试注入
/// - `read`：剪贴板读函数（None = 剪贴板无文本/读取失败），测试注入
///
/// # 返回
/// `Some(新文本)` 命中；`None` 超时未变（无选区/复制被拒——调用方按用户定案静默）
pub fn wait_clipboard_change(
    before: Option<&str>,
    timeout_ms: u64,
    step_ms: u64,
    mut now_ms: impl FnMut() -> u64,
    mut read: impl FnMut() -> Option<String>,
) -> Option<String> {
    let deadline = now_ms() + timeout_ms;
    loop {
        if let Some(text) = read() {
            let trimmed = text.trim();
            if !trimmed.is_empty() && Some(trimmed) != before.map(str::trim) {
                return Some(text);
            }
        }
        if now_ms() >= deadline {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(step_ms));
    }
}

// —— Win32 运行时段（仅 Windows）——

#[cfg(target_os = "windows")]
mod win {
    /// 合成 Ctrl+C 到前台应用（keybd_event 直连）：先合成修饰键 keyup 清残留
    /// （优化定案 2026-09-30：热键触发瞬间用户手指/合成器仍按着 Ctrl+Shift 等，
    /// 直接叠加合成 Ctrl+C 会组合成 Ctrl+Shift+C 致前台复制静默失效——此前用
    /// "等释放"方案有 200ms 上限，按住更久则失效且白等拖慢触发；合成 keyup 归零
    /// 等待且对任意按住时长稳健，物理松开时的真实 keyup 冗余无害）→ Ctrl down →
    /// C down → C up → Ctrl up，键间 10ms 让前台应用吃到完整序列——有选区的应用
    /// 将其解释为"复制选中内容"，剪贴板随之更新（无选区/拒绝合成则剪贴板不变，
    /// 调用方轮询超时静默）。keybd_event 自 Vista 起即 SendInput 的兼容薄封装。
    pub fn synthesize_ctrl_c() {
        use std::time::Duration;
        const VK_SHIFT: u8 = 0x10;
        const VK_CONTROL: u8 = 0x11;
        const VK_MENU: u8 = 0x12;
        const VK_LWIN: u8 = 0x5B;
        const VK_RWIN: u8 = 0x5C;
        const VK_C: u8 = 0x43;
        const KEYEVENTF_KEYUP: u32 = 0x0002;
        unsafe {
            // 清残留：修饰键全部合成 keyup（物理按住时系统键态被提前置放，
            // 用户松手的真实 keyup 冗余无害）
            for vk in [VK_SHIFT, VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN] {
                keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
                std::thread::sleep(Duration::from_millis(5));
            }
            keybd_event(VK_CONTROL, 0, 0, 0);
            std::thread::sleep(Duration::from_millis(10));
            keybd_event(VK_C, 0, 0, 0);
            std::thread::sleep(Duration::from_millis(10));
            keybd_event(VK_C, 0, KEYEVENTF_KEYUP, 0);
            std::thread::sleep(Duration::from_millis(10));
            keybd_event(VK_CONTROL, 0, KEYEVENTF_KEYUP, 0);
        }
    }

    /// 前台窗口是否控制台宿主（PL016.4 终端边界守卫）：conhost
    /// （ConsoleWindowClass）与 Windows Terminal（CASCADIA_HOSTING_WINDOW_CLASS）
    /// 无选区时 Ctrl+C = 中断信号（SIGINT）——合成会打断前台进程，命中即跳过
    /// 合成直接静默。防御式落地依据微软文档公认行为（条目要求实测确认，
    /// live 复核待用户授权；若实测无风险可摘除）。
    pub fn foreground_is_console() -> bool {
        const CLASS_CONHOST: &str = "ConsoleWindowClass";
        const CLASS_TERMINAL: &str = "CASCADIA_HOSTING_WINDOW_CLASS";
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd == 0 {
                return false;
            }
            let mut buf = [0u16; 256];
            let len = GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
            if len <= 0 {
                return false;
            }
            let class = String::from_utf16_lossy(&buf[..len as usize]);
            class == CLASS_CONHOST || class == CLASS_TERMINAL
        }
    }

    #[link(name = "user32")]
    extern "system" {
        /// 合成键盘事件（SendInput 兼容封装；全局影响 = 向前台窗口投递按键）
        fn keybd_event(bvk: u8, bscan: u8, dwflags: u32, dwextrainfo: usize);
        /// 取前台窗口句柄（0 = 无前台窗）
        fn GetForegroundWindow() -> isize;
        /// 前台窗口类名（返回拷入字符数，0 = 失败）
        fn GetClassNameW(hwnd: isize, buf: *mut u16, max_count: i32) -> i32;
    }
}

#[cfg(target_os = "windows")]
pub use win::{foreground_is_console, synthesize_ctrl_c};

#[cfg(test)]
mod tests {
    use super::wait_clipboard_change;

    /// 递增假时钟（毫秒）：tick 返回累加值，保证轮询有限轮收敛（step=0 零真实等待）
    struct Clock(u64);
    impl Clock {
        fn tick(&mut self, ms: u64) -> u64 {
            self.0 += ms;
            self.0
        }
    }

    #[test]
    fn hits_immediately_on_new_text() {
        // 首次读取即返回新文本 → 立即命中（不走到超时判断）
        let mut clock = Clock(0);
        let got = wait_clipboard_change(
            Some("旧"),
            300,
            0,
            || clock.tick(1),
            || Some("新".to_string()),
        );
        assert_eq!(got.as_deref(), Some("新"));
    }

    #[test]
    fn times_out_silently_when_unchanged() {
        // 读到的恒为快照原值 → 超时 None（无选区静默定案的状态机基础）
        let mut clock = Clock(0);
        let got = wait_clipboard_change(
            Some("旧"),
            300,
            0,
            || clock.tick(100),
            || Some("旧".to_string()),
        );
        assert_eq!(got, None);
    }

    #[test]
    fn hits_after_text_changes_midway() {
        // 前 2 次旧文本、第 3 次起新文本 → 中途命中（模拟复制落剪贴板有时延）
        let mut clock = Clock(0);
        let mut calls = 0u32;
        let got = wait_clipboard_change(
            Some("旧"),
            300,
            0,
            || clock.tick(50),
            || {
                calls += 1;
                if calls < 3 {
                    Some("旧".to_string())
                } else {
                    Some("新".to_string())
                }
            },
        );
        assert_eq!(got.as_deref(), Some("新"));
        assert_eq!(calls, 3);
    }

    #[test]
    fn ignores_empty_and_whitespace_text() {
        // 空串/纯空白不算命中（条目语义：非空且不同才命中）→ 超时 None
        let mut clock = Clock(0);
        let mut calls = 0u32;
        let got = wait_clipboard_change(
            Some("旧"),
            100,
            0,
            || clock.tick(50),
            || {
                calls += 1;
                if calls % 2 == 1 {
                    Some(String::new())
                } else {
                    Some("   ".to_string())
                }
            },
        );
        assert_eq!(got, None);
    }

    #[test]
    fn trim_equivalent_text_is_not_a_change() {
        // 尾随换行与快照 trim 等价 → 不算变化（复制工具尾随换行容忍）
        let mut clock = Clock(0);
        let got = wait_clipboard_change(
            Some("旧文本"),
            100,
            0,
            || clock.tick(50),
            || Some("旧文本\n".to_string()),
        );
        assert_eq!(got, None);
    }

    #[test]
    fn snapshot_none_hits_any_nonempty() {
        // 原剪贴板无文本（None）→ 任何非空新文本即命中
        let mut clock = Clock(0);
        let got = wait_clipboard_change(
            None,
            300,
            0,
            || clock.tick(1),
            || Some("圈选内容".to_string()),
        );
        assert_eq!(got.as_deref(), Some("圈选内容"));
    }
}
