//! 测试数据注入器（examples 工具，不入构建产物）：
//! 向 APP 真实库（data/todo.db，dev 双落址=仓库根）重复注入目测/功能检查用的样本数据。
//! 全部走存储层 API（参数绑定），拨动注入时钟精确落 created_at/done_at——
//! 龄期黄/红提醒、归档 done_at 倒序、详情板长文滑杆等场景一钻全覆盖。
//!
//! 用法（仓库根执行；注入前先关闭 APP 防库锁）：
//!   cargo run --manifest-path core/Cargo.toml --example seed_data            // 默认 10 待办 + 3 归档 + 6 气泡
//!   cargo run --manifest-path core/Cargo.toml --example seed_data -- 20 8    // 自定数量
//! 数据库之后整体删除 data/ 目录即重置，注入内容无需清理。
//!
//! 每轮内容：未完成待办 N 条（created_at 分四档：红龄期/黄龄期/近三小时/刚刚，
//! 三分之一带 note——含一条 500 字长文）+ 已完成归档条目 D 条（done_at 阶梯循环
//! 1h/30h/3d，归档视图倒序可验）+ 气泡 M 条（短/中/超长文轮转）+ 白板追加检查文本。

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use capsule_todo::paths;
use capsule_todo::storage::Storage;

const HOUR: i64 = 3600 * 1000;
const DAY: i64 = 24 * HOUR;

/// 未完成待办文本池（循环取用）
const TODO_POOL: [&str; 20] = [
    "买牛奶（全脂 2L）",
    "给窗台绿植浇水",
    "回复产品评审邮件",
    "预约周五羽毛球场地",
    "整理桌面线材",
    "读完《起风了》第 3 章",
    "取快递（菜鸟驿站）",
    "写周会发言提纲",
    "归还借用的充电宝",
    "检查车险到期日",
    "备份手机照片到 NAS",
    "换牙刷（满三个月了）",
    "给妈妈打电话",
    "订下周生日蛋糕",
    "慢跑 5 公里",
    "清空下载文件夹",
    "核对上月账单",
    "更新简历附件",
    "买感冒药备用",
    "擦一遍显示器",
];

/// 已完成归档文本池
const DONE_POOL: [&str; 6] = [
    "归档板交互原型评审",
    "定稿玻璃令牌三档阴影",
    "修复详情板收板闪烁",
    "给 CapsuleTODO 画个图标",
    "整理季度会议纪要",
    "升级构建依赖版本",
];

/// 短笔记池（三分之一待办随机带）
const NOTE_POOL: [&str; 5] = [
    "顺路取快递",
    "周五前完成",
    "问一下价格再说",
    "带上会员卡",
    "先做半小时",
];

/// 长笔记（~500 字，验详情板整板阅读与滑杆）
const LONG_NOTE: &str = "本周完成事项：\n一、玻璃材质迁移：卡片、页签、输入条三处背景从纯色叠加切换为 backdrop-filter 实时采样，GPU 占用峰值从 12% 降到 7%，滚动帧率稳定 60fps。踩坑记录：嵌套采样边界——外层 backdrop 会把内层 backdrop 的输出当背景再模糊一次，双层叠加区域整体发糊，最终采用内层只描边不做磨砂的方案绕过。\n二、勾选组件本地化：uiverse 霓虹勾选框原始实现依赖 fixed 定位粒子层，卡片 overflow 裁剪下粒子整体丢失；改为随行内绝对定位后，抛洒半径从 24px 压缩到 14px，keyframes 全程 transform 合成，快速连点不掉帧。\n三、数据层联调：todos 表新增时间戳字段并建索引，查询按未完成优先排序；过期提醒阈值判断收敛在渲染层，列表拉取保持全量。\n下周计划：详情面板接撤销栈、收敛阴影令牌为三档、补齐触摸长按拖拽、过一遍减动效全链路。";

/// 气泡文本池（短/中/超长轮转——中长验两行截断、超长验全文板滑杆）
const BUBBLE_POOL: [&str; 3] = [
    "玻璃质感三层叠加：半透明底色给体积，backdrop-filter 给实时模糊，描边与落影给边界。",
    "中长气泡：玻璃的质感来自克制而不是效果的堆叠——着色永远浅，21% 的 accent 淡染只负责提示身份；边界必须交代，1px 内描边加一道向下落影，玻璃才有厚度而不是一块滤镜。这一条在行里超过两行，只露两行并以省略号收尾，单击开板可读全文。",
    "超长气泡：临时剪贴板的定位本来就是十五秒记忆——捕获的成本必须远低于打开备忘录，看完即弃；溢出提醒只是温和地催你清台面。滚动在手写实现里比看起来简单：scrollHeight 减 clientHeight 得到总行程，scrollTop 除以它就是比例，比例乘轨道高就是浮钮位置；难的是手感，拖拽要跟手、松手要即停、边界不弹跳。玻璃负责记住，人类负责醒来——清单的意义不在于清空，而在于你每一次凝视它的时候，心里那一下轻轻的疼。疼一下，就长大一点。",
];

fn main() {
    // 参数：[待办数=10] [气泡数=6] [归档数=3]
    let args: Vec<String> = std::env::args().skip(1).collect();
    let todo_count: usize = args.first().and_then(|s| s.parse().ok()).unwrap_or(10);
    let bubble_count: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(6);
    let done_count: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(3);

    let db = match paths::db_path() {
        Ok(p) => p,
        Err(err) => {
            eprintln!("定位数据库失败：{err}");
            std::process::exit(1);
        }
    };
    println!("目标库：{}", db.display());

    // 可拨动注入时钟：从当前时刻起，逐条拨到目标 created_at/done_at
    let clock = Arc::new(AtomicI64::new(now_ms()));
    let clock2 = Arc::clone(&clock);
    let storage = match Storage::open_with_now(&db, Arc::new(move || clock2.load(Ordering::SeqCst)))
    {
        Ok(s) => s,
        Err(err) => {
            eprintln!("打开库失败：{err}\n（若提示数据库被锁，请先关闭正在运行的 APP 再注入）");
            std::process::exit(1);
        }
    };

    let base = now_ms();
    let pool_len = TODO_POOL.len();
    // 已有条数作为池子偏移：重复注入不撞同文本
    let offset = storage.list().map(|l| l.len()).unwrap_or(0);

    // —— 未完成待办：created_at 四档分布（红 1/4、黄 1/4、3h 1/4、刚刚 1/4）——
    for i in 0..todo_count {
        let age_hours: i64 = match i % 4 {
            0 => 49, // 红（>48h）
            1 => 30, // 黄（>24h）
            2 => 3,
            _ => 0,
        };
        clock.store(base - age_hours * HOUR, Ordering::SeqCst);
        let text = TODO_POOL[(offset + i) % pool_len];
        let item = match storage.add(text) {
            Ok(it) => it,
            Err(err) => {
                eprintln!("注入待办失败（APP 可能正在运行占用库）：{err}");
                std::process::exit(1);
            }
        };
        if i % 3 == 0 {
            let note = if i == 0 {
                LONG_NOTE
            } else {
                NOTE_POOL[i % NOTE_POOL.len()]
            };
            storage.set_note(item.id, note).expect("写笔记必须成功");
        }
    }

    // —— 已完成归档条目：done_at 三档阶梯循环（1h/30h/3d），归档倒序可验 ——
    for i in 0..done_count {
        let age: i64 = [HOUR, 30 * HOUR, 3 * DAY][i % 3];
        clock.store(base - 100 * DAY, Ordering::SeqCst); // 创建时刻推远
        let text = DONE_POOL[(offset + i) % DONE_POOL.len()];
        let item = storage.add(text).expect("注入归档条目必须成功");
        clock.store(base - age, Ordering::SeqCst);
        storage.toggle(item.id).expect("勾选归档必须成功");
    }

    // —— 气泡：短/中/超长轮转 ——
    for i in 0..bubble_count {
        let text = BUBBLE_POOL[i % BUBBLE_POOL.len()];
        storage.add_bubble(text).expect("注入气泡必须成功");
    }

    // —— 白板：追加一段检查文本（不覆盖既有草稿）——
    let stamp = chrono_stamp(base);
    let board = storage.load_whiteboard().expect("读白板必须成功");
    let appended = format!(
        "{board}\n【注入检查 {stamp}】白板整板阅读样本：把这一段写长一些，让内容高度超过玻璃板，观察文字滑过上下溶解带、▲▼ 三角显隐、到底抬带与板内滑杆跟随——随手记的内容在此只是过客，玻璃负责记住，人类负责醒来。\n"
    );
    storage.save_whiteboard(&appended).expect("存白板必须成功");

    // —— 摘要 ——
    let todos = storage.list().expect("复读清单必须成功");
    let bubbles = storage.list_bubbles().expect("复读气泡必须成功");
    println!(
        "注入完成：本轮 +{todo_count} 待办 +{done_count} 归档 +{bubble_count} 气泡；\n\
         当前库内：未完成 {} 条 / 归档 {} 条 / 气泡 {} 条。\n\
         白板已追加检查段落。打开 APP 即可目测；再次运行本命令可继续追加。",
        todos.iter().filter(|t| !t.done).count(),
        todos.iter().filter(|t| t.done).count(),
        bubbles.len(),
    );
}

/// 当前时刻（epoch 毫秒）
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// epoch 毫秒 → 本地可读时间戳（注入标记用，不引外部依赖）
fn chrono_stamp(ms: i64) -> String {
    let secs = ms / 1000;
    let days = secs / 86400;
    let rem = secs % 86400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // 粗略民用日期（从 2000-01-01 起算足够做标记）
    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02}:{s:02}")
}

/// 天数 → (年, 月, 日)（Howard Hinnant 民用算法）
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
