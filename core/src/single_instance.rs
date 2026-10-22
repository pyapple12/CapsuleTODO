//! 单实例闸门（PATCH002.64，来源 = 系列项目 CapsulePulse FIX006.10）：在**进程第一行**
//! 判重——第二个 exe「开不了」，且**不碰数据库、不建窗、不出托盘**（零副作用；仅跨会话
//! `ACCESS_DENIED` 判重时会写一条诊断留痕，见 `acquire_mutex`）。
//!
//! **为什么不用 `tauri-plugin-single-instance`**（CP 实证两缺陷）：① 它的判定跑在**插件
//! setup**（晚于 `run()` 前段的开库与留痕）⇒ 第二实例被拦下前已经开库（建表 DDL）并可能
//! 往共享日志写过东西；② 它靠 `FindWindowW` 去找首实例**稍后才创建的隐藏消息窗**，取不到
//! 时**没有 else 分支**（不退出）⇒ 启动竞态下两实例**并存**、共享同一 `data/todo.db`。
//! 本模块把闸门前移到第一行，两个缺陷一并消除；**零新依赖**（手写 kernel32 FFI，沿
//! `glass_backdrop` / `window_icon` / `hotkey` 先例）。
//!
//! 命名空间取 **`Global\`**（全机器级判重、跨用户会话）——MSDN「Kernel object namespaces」：
//! `SeCreateGlobalPrivilege` **仅对 file-mapping / symbolic-link 的创建**有要求，命名
//! mutex / event **不需要**该特权 ⇒ 普通用户进程也能建 ✓。安全描述符用**系统默认**
//! （只授创建者）：跨用户会话时 `CreateMutexW` 返回 `0 + ERROR_ACCESS_DENIED`——本模块把
//! 该错误码**同样判为"已有实例"**（并**落一条诊断日志**，免"无声不启动"）⇒ 跨会话判重照样
//! 成立，且**不必**自建 DACL。
//!
//! **为什么不用 NULL DACL**（2026-10-20 复盘，含一次错误定性）：NULL DACL 版曾实测
//! `CreateMutexW` 返回 `ERROR_NOACCESS`(998)，当时误判为"绝对安全描述符须
//! `MakeSelfRelativeSD`" ✗ ——**真因是 SD 缓冲尺寸写错**：x64 的 `SECURITY_DESCRIPTOR` 是
//! **40 字节**（2×BYTE + CONTROL + 4 个指针），本模块当时只给了 20 字节 ⇒
//! `InitializeSecurityDescriptor` **越界写**、写坏紧邻的 `SECURITY_ATTRIBUTES`（其
//! `nLength` 本身是对的 24）⇒ 998，与"绝对 / 自相对"无关（系列项目 CapsulePulse 用
//! NULL DACL + 40B `[u64; 5]` + Box 持有长期稳定，即反证）。**最终仍取默认 SD**：功能
//! 等价（跨会话由错误码覆盖），而 NULL DACL 会让 `Global\` 对象对本机**任意用户**可写 ⇒
//! 任何人可占名让本应用起不来（只增暴露面、无功能收益 ✗）。将来若确需"错误码无歧义"的
//! 跨会话判重，再按**三查**（尺寸 / 对齐 / 保活）恢复。
//!
//! 唤起：`Global\…-show` 命名事件（**自动复位** = 脉冲语义，早到不丢）。第二实例
//! `SetEvent` 后即退；首实例在 `setup` 起守候线程 `wait_for_wake`，被唤醒即把主窗唤到
//! 前台（等价旧插件回调）。
//!
//! 失败降级：建对象失败（极罕见）⇒ **落日志并放行**——宁可可能双开，不可把用户挡在
//! 应用外（沿"启动迁移失败仅落档不阻断"同一条纪律）。
//!
//! 平台：本模块当前只实现 Windows（唯一实机平台）。非 Windows 恒返回 `Gate::First`
//! （不判重），待 macOS/Linux 适配时补等价实现（flock / 命名 socket）。
//!
//! 本模块**不 import tauri**（守候由调用方起线程、唤窗由调用方做）⇒ `cargo test` 免窗口可测。

/// 判重结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Gate {
    /// 本进程是首个实例：已在 `Global\` 命名空间占住闸门（句柄由内核持有至进程结束）。
    First,
    /// 已有实例在跑：已向它发出"把窗口唤到前台"的信号，调用方应**立即退出**。
    Second,
}

/// 第二实例的退出码（0 = 非错误；与旧插件 `exit(0)` 同义）。
pub(crate) const SECOND_INSTANCE_EXIT: i32 = 0;

/// 进程启动第一行调用：占住闸门，或给已有实例发"唤前"信号。
///
/// # 参数
/// - `identifier`：`tauri.conf.json` 的 `identifier`（同一 identifier 的副本视为同一应用）。
///
/// # 返回
/// `Gate::First` = 继续启动；`Gate::Second` = 调用方应立即以 `SECOND_INSTANCE_EXIT` 退出。
/// 建对象失败时返回 `Gate::First` 并落一条诊断日志（放行优先）。
pub(crate) fn enforce(identifier: &str) -> Gate {
    #[cfg(target_os = "windows")]
    {
        let name = format!("Global\\CapsuleTODO-SingleInstance-{identifier}");
        match acquire_mutex(&name) {
            MutexOutcome::First(handle) => {
                // 句柄存起来**一直持有**（进程退出由内核回收 ⇒ 永不残留脏状态）
                MUTEX_HANDLE.store(handle, std::sync::atomic::Ordering::SeqCst);
                // 首实例：建"唤前"事件（自动复位），供第二实例发脉冲
                let event = create_show_event(&show_event_name(identifier));
                if event == 0 {
                    // FIX014.22：唤前是增强入口；事件建不起来只损失双击唤前，
                    // 落日志留痕，不阻断首实例启动。
                    crate::diag::warn_diag("单实例唤前事件创建失败（双击 exe 唤前不可用）");
                }
                SHOW_EVENT.store(event, std::sync::atomic::Ordering::SeqCst);
                Gate::First
            }
            MutexOutcome::Second => {
                signal_show_window(identifier);
                Gate::Second
            }
            MutexOutcome::Failed => {
                crate::diag::warn_diag(&format!(
                    "单实例闸门创建失败（放行，可能双开）err={}",
                    unsafe { GetLastError() }
                ));
                Gate::First
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = identifier;
        Gate::First
    }
}

/// 阻塞等待"唤前"脉冲（首实例 `setup` 起守候线程调用）。
///
/// # 返回
/// `true` = 收到第二实例的唤前信号（调用方应唤主窗到前台）；`false` = 事件未建 / 等待失败
/// （调用方**不动作**——防误唤）。
pub(crate) fn wait_for_wake() -> bool {
    #[cfg(target_os = "windows")]
    {
        let event = SHOW_EVENT.load(std::sync::atomic::Ordering::SeqCst);
        if event == 0 {
            return false;
        }
        const INFINITE: u32 = 0xFFFF_FFFF;
        const WAIT_OBJECT_0: u32 = 0;
        unsafe { WaitForSingleObject(event, INFINITE) == WAIT_OBJECT_0 }
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

#[cfg(target_os = "windows")]
use std::sync::atomic::AtomicIsize;

/// 闸门句柄（首实例持有至进程结束；0 = 未持有）。
#[cfg(target_os = "windows")]
static MUTEX_HANDLE: AtomicIsize = AtomicIsize::new(0);

/// "唤前"事件句柄（首实例创建；0 = 未建）。
#[cfg(target_os = "windows")]
static SHOW_EVENT: AtomicIsize = AtomicIsize::new(0);

/// 判重结果（内部：区分"建对象失败"以走放行降级）。
#[cfg(target_os = "windows")]
enum MutexOutcome {
    /// 本进程占住闸门（携带句柄）。
    First(isize),
    /// 已有实例在跑。
    Second,
    /// 建对象失败（放行）。
    Failed,
}

/// "唤前"事件名（与闸门同前缀 + `-show` 后缀）。
#[cfg(target_os = "windows")]
fn show_event_name(identifier: &str) -> String {
    format!("Global\\CapsuleTODO-SingleInstance-{identifier}-show")
}

// 注：NULL DACL 构造（`SECURITY_DESCRIPTOR` / `SECURITY_ATTRIBUTES`）**未采用** ——
// 曾因 **SD 缓冲尺寸不足**（x64 需 40B，当时只给 20B）而越界写、`CreateMutexW` 返回
// `ERROR_NOACCESS`(998)；完整纠偏与取舍见模块头「为什么不用 NULL DACL」。现行实现 =
// **系统默认安全描述符** + "`ERROR_ACCESS_DENIED` 也判已有实例"（见 `acquire_mutex`）。

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
extern "system" {
    /// 创建 / 打开命名互斥体（`bInitialOwner` = 0；已存在时返回既有句柄 + `ERROR_ALREADY_EXISTS`）。
    fn CreateMutexW(attrs: *const core::ffi::c_void, initial_owner: i32, name: *const u16)
        -> isize;
    /// 创建命名事件（`manual_reset` = 0 ⇒ 自动复位 = 脉冲语义；`initial_state` = 0）。
    fn CreateEventW(
        attrs: *const core::ffi::c_void,
        manual_reset: i32,
        initial_state: i32,
        name: *const u16,
    ) -> isize;
    /// 打开既有命名事件（`EVENT_MODIFY_STATE` = 0x0002）。
    fn OpenEventW(desired_access: u32, inherit: i32, name: *const u16) -> isize;
    /// 置事件为有信号（唤前脉冲）。
    fn SetEvent(handle: isize) -> i32;
    /// 等待对象（此处 `INFINITE`，不会超时）。
    fn WaitForSingleObject(handle: isize, millis: u32) -> u32;
    /// 取上一条错误码。
    fn GetLastError() -> u32;
    /// 清错误码（成功新建对象时 `GetLastError` 不保证被清零 ⇒ 调用前置 0 防误判）。
    fn SetLastError(code: u32);
}

/// 字符串 → 以 NUL 结尾的 UTF-16（Win32 宽字符入参）。
#[cfg(target_os = "windows")]
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 占闸门：`CreateMutexW`（**系统默认安全描述符**）+ 错误码判已有实例。
///
/// 判据：`ERROR_ALREADY_EXISTS`(183) = 同会话已有实例；`ERROR_ACCESS_DENIED`(5) = 对象
/// 已存在但属**另一用户会话**（默认 DACL 打不开）—— **同样判"已有实例"** ⇒ 跨会话
/// （快速切换用户）判重成立，无需自建 DACL。其余失败走放行降级。
#[cfg(target_os = "windows")]
fn acquire_mutex(name: &str) -> MutexOutcome {
    const ERROR_ACCESS_DENIED: u32 = 5;
    const ERROR_ALREADY_EXISTS: u32 = 183;
    let name_w = wide(name);
    // 先清错误码：成功新建时 GetLastError 不保证被清零，残留 183 会误判"已有实例"
    unsafe { SetLastError(0) };
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name_w.as_ptr()) };
    let err = unsafe { GetLastError() };
    if handle == 0 {
        if err == ERROR_ACCESS_DENIED {
            // 跨用户会话：对象已存在、但默认 DACL 打不开 ⇒ 判"已有实例"。
            // **必须留痕**：万一被组策略 / 沙箱拦住，否则表现成"无声不启动"（2026-10-20 复盘）
            crate::diag::warn_diag(
                "单实例闸门：创建被拒（ACCESS_DENIED）⇒ 判为已有实例；若非双开请查安全策略",
            );
            return MutexOutcome::Second;
        }
        return MutexOutcome::Failed;
    }
    if err == ERROR_ALREADY_EXISTS {
        // 既有实例在跑：关掉本进程拿到的这个句柄（避免泄漏），走 Second
        unsafe { CloseHandle(handle) };
        MutexOutcome::Second
    } else {
        MutexOutcome::First(handle)
    }
}

/// 建"唤前"事件（首实例）：失败返回 0（守候线程随即自灭，仅损失"双击唤前"）。
#[cfg(target_os = "windows")]
fn create_show_event(name: &str) -> isize {
    let name_w = wide(name);
    unsafe { CreateEventW(std::ptr::null(), 0, 0, name_w.as_ptr()) }
}

/// 第二实例：打开唤前事件并置信号（失败静默——唤前是便利层，不得阻断第二实例退出）。
#[cfg(target_os = "windows")]
fn signal_show_window(identifier: &str) {
    const EVENT_MODIFY_STATE: u32 = 0x0002;
    let name_w = wide(&show_event_name(identifier));
    let handle = unsafe { OpenEventW(EVENT_MODIFY_STATE, 0, name_w.as_ptr()) };
    if handle == 0 {
        return;
    }
    unsafe {
        SetEvent(handle);
        CloseHandle(handle);
    }
}

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
extern "system" {
    /// 关闭内核对象句柄（本进程不再需要时调用）。
    fn CloseHandle(handle: isize) -> i32;
}
