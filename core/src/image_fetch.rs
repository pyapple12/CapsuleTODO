//! HTTP(S) 远端图片下载（PATCH002.76）：只做网络取字节，**不碰 tauri / DB**。
//!
//! 设计理由：粘贴 CF_HTML 时图片可能来自 `http(s)`，但网络不能在粘贴命令里同步等；
//! 本模块供后台任务调用，限制单张 10MB、单张 10s、整批 20s、并发 4、总字节 50MB，
//! 失败按段返回（由调用方降级为文本链接，不升格整条粘贴）。纯函数（URL 白名单 /
//! 限长读取 / 并发数）与本地回环 HTTP 用例可在 `cargo test` 直测。

use std::collections::VecDeque;
use std::io::Read;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use reqwest::blocking::{Client, Response};
use reqwest::redirect::Policy;

/// 单张图片上限（与详情块粘贴上限同口径）
pub const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;
/// 整批总字节上限（防一次粘贴多张大图把内存打满）
pub const MAX_TOTAL_BYTES: usize = 50 * 1024 * 1024;
/// 并发 worker 上限
pub const MAX_CONCURRENCY: usize = 4;
/// 单张请求超时
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// 连接超时（请求超时的子限制）
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// 整批总预算（超时后剩余 URL 直接降级文本）
pub const TOTAL_BUDGET: Duration = Duration::from_secs(20);

/// 单张下载失败原因（调用方逐张降级，不抛顶层错误）
#[derive(Debug, Clone)]
pub enum FetchError {
    /// 非 http(s) URL（相对路径 / data: / file: 等）
    InvalidUrl,
    /// 请求失败 / HTTP 非 2xx
    Request(String),
    /// 单张超过 10MB
    TooLarge,
    /// 整批超过 50MB
    TotalLimit,
    /// 单张或整批超时
    Timeout,
    /// 读取响应体失败
    Read(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUrl => write!(f, "非 http(s) 图片地址"),
            Self::Request(msg) => write!(f, "请求失败：{msg}"),
            Self::TooLarge => write!(f, "图片超过单张上限"),
            Self::TotalLimit => write!(f, "本批图片总字节超限"),
            Self::Timeout => write!(f, "下载超时"),
            Self::Read(msg) => write!(f, "读取响应失败：{msg}"),
        }
    }
}

/// 单张结果（成功 = 原字节；失败 = 降级原因）
type FetchResult = Result<Vec<u8>, FetchError>;
/// 并发任务的结果槽（同序；未执行 = None）
type FetchResults = Vec<Option<FetchResult>>;

/// 规范化 http(s) URL（trim + 小写 scheme）；其他 scheme / 相对路径 → None。
/// 返回的是 `Url::to_string()` 后的安全形态（无空白/换行），供占位 `source_url`
/// 与失败降级文本共用。
pub fn normalize_url(src: &str) -> Option<String> {
    let url = reqwest::Url::parse(src.trim()).ok()?;
    match url.scheme() {
        "http" | "https" => Some(url.to_string()),
        _ => None,
    }
}

/// 并发数收敛（纯函数，测试直测）
fn worker_count(n: usize) -> usize {
    n.min(MAX_CONCURRENCY)
}

/// FIX014.21：CAS 预留整批总字节；失败时**不改变**计数器。
/// 只有真正被接纳的图片才占预算，超限图片不会把后续小图一起拖下水。
fn reserve_total(total: &AtomicUsize, len: usize) -> bool {
    let mut current = total.load(Ordering::Relaxed);
    loop {
        let Some(next) = current.checked_add(len) else {
            return false;
        };
        if next > MAX_TOTAL_BYTES {
            return false;
        }
        match total.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return true,
            Err(actual) => current = actual,
        }
    }
}

/// 读取最多 `limit` 字节；多读 1 字节用于判定超限（**不信任 Content-Length**）
fn read_limited<R: Read>(reader: R, limit: usize) -> FetchResult {
    let mut buf = Vec::new();
    let mut limited = reader.take(limit as u64 + 1);
    limited
        .read_to_end(&mut buf)
        .map_err(|err| FetchError::Read(err.to_string()))?;
    if buf.len() > limit {
        Err(FetchError::TooLarge)
    } else {
        Ok(buf)
    }
}

/// 读响应（状态码 / Content-Length 预检 / 限长读取）
fn read_response(response: Response) -> FetchResult {
    if !response.status().is_success() {
        return Err(FetchError::Request(format!("HTTP {}", response.status())));
    }
    if let Some(len) = response.content_length() {
        if len > MAX_IMAGE_BYTES as u64 {
            return Err(FetchError::TooLarge);
        }
    }
    read_limited(response, MAX_IMAGE_BYTES)
}

/// 单张下载（`timeout` 由总预算动态收紧）
fn fetch_one(client: &Client, url: &str, timeout: Duration) -> FetchResult {
    let response = client
        .get(url)
        .timeout(timeout)
        .send()
        .map_err(|err| FetchError::Request(err.to_string()))?;
    read_response(response)
}

/// 并发下载（同序返回）。每张失败独立；不返回顶层错误。
/// 总预算/总字节在 worker 内动态检查，剩余 URL 标 `Timeout` / `TotalLimit`。
pub fn fetch_all(urls: &[String]) -> Vec<FetchResult> {
    if urls.is_empty() {
        return Vec::new();
    }
    let client = match Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .redirect(Policy::limited(5))
        .user_agent(concat!("CapsuleTODO/", env!("CARGO_PKG_VERSION")))
        .build()
    {
        Ok(client) => client,
        // 客户端都建不起来（TLS 初始化失败等）→ 每张都降级，不阻断粘贴
        Err(err) => {
            return urls
                .iter()
                .map(|_| Err(FetchError::Request(format!("HTTP 客户端初始化失败：{err}"))))
                .collect()
        }
    };
    let workers = worker_count(urls.len());
    let queue: Mutex<VecDeque<(usize, String)>> =
        Mutex::new(urls.iter().cloned().enumerate().collect());
    let results: Mutex<FetchResults> = Mutex::new(vec![None; urls.len()]);
    let total_bytes = AtomicUsize::new(0);
    let start = Instant::now();

    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let item = {
                    let Ok(mut queue) = queue.lock() else {
                        break;
                    };
                    queue.pop_front()
                };
                let Some((index, url)) = item else {
                    break;
                };
                let result = if start.elapsed() >= TOTAL_BUDGET {
                    Err(FetchError::Timeout)
                } else if total_bytes.load(Ordering::Relaxed) >= MAX_TOTAL_BYTES {
                    Err(FetchError::TotalLimit)
                } else {
                    let remaining = TOTAL_BUDGET.saturating_sub(start.elapsed());
                    let per_request = REQUEST_TIMEOUT.min(remaining);
                    if per_request.is_zero() {
                        Err(FetchError::Timeout)
                    } else {
                        match normalize_url(&url) {
                            Some(normalized) => fetch_one(&client, &normalized, per_request),
                            None => Err(FetchError::InvalidUrl),
                        }
                    }
                };
                let result = match result {
                    Ok(bytes) => {
                        // 只有接纳成功才计数；CAS 失败/超限不留下虚增配额。
                        if reserve_total(&total_bytes, bytes.len()) {
                            Ok(bytes)
                        } else {
                            Err(FetchError::TotalLimit)
                        }
                    }
                    Err(err) => Err(err),
                };
                if let Ok(mut results) = results.lock() {
                    results[index] = Some(result);
                }
            });
        }
    });

    let results = match results.into_inner() {
        Ok(results) => results,
        Err(poisoned) => poisoned.into_inner(),
    };
    results
        .into_iter()
        .map(|result| result.unwrap_or(Err(FetchError::Request("下载任务未执行".to_string()))))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    use super::*;

    #[test]
    fn normalize_url_allows_only_http_https() {
        assert!(normalize_url("https://a/b.png").is_some());
        assert!(normalize_url("HTTP://a/b.png").is_some());
        assert!(normalize_url("  https://a/b.png  ").is_some());
        assert!(normalize_url("data:image/png;base64,AA==").is_none());
        assert!(normalize_url("file:///C:/a.png").is_none());
        assert!(normalize_url("ftp://a/b.png").is_none());
        assert!(normalize_url("//cdn/a.png").is_none());
        assert!(normalize_url("relative/a.png").is_none());
    }

    #[test]
    fn read_limited_rejects_one_byte_over() {
        let exact = read_limited(std::io::Cursor::new(vec![1u8; 4]), 4).expect("恰好上限必须过");
        assert_eq!(exact.len(), 4);
        let err = read_limited(std::io::Cursor::new(vec![1u8; 5]), 4).expect_err("超 1 字节必须拒");
        assert!(matches!(err, FetchError::TooLarge));
    }

    #[test]
    fn worker_count_caps_at_four() {
        assert_eq!(worker_count(1), 1);
        assert_eq!(worker_count(4), 4);
        assert_eq!(worker_count(10), 4);
    }

    #[test]
    fn total_reservation_does_not_consume_rejected_bytes() {
        // FIX014.21：超限拒绝不得虚增计数器；随后的小图仍可入预算。
        let total = AtomicUsize::new(MAX_TOTAL_BYTES - 5);
        assert!(!reserve_total(&total, 10));
        assert_eq!(total.load(Ordering::Relaxed), MAX_TOTAL_BYTES - 5);
        assert!(reserve_total(&total, 5));
        assert_eq!(total.load(Ordering::Relaxed), MAX_TOTAL_BYTES);
        assert!(!reserve_total(&total, 1));
        assert_eq!(total.load(Ordering::Relaxed), MAX_TOTAL_BYTES);
    }

    #[test]
    fn fetch_all_rejects_non_http_without_network() {
        let out = fetch_all(&["file:///C:/a.png".to_string()]);
        assert!(matches!(out[0], Err(FetchError::InvalidUrl)));
    }

    /// 本地回环单请求服务：返回固定状态与 body（测试不依赖外网）
    fn serve_once(status: &str, body: &[u8]) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("回环端口必须可绑");
        let addr = listener.local_addr().expect("端口地址");
        let status = status.to_string();
        let body = body.to_vec();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("必须收到连接");
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let header = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(&body);
        });
        (format!("http://{addr}/img.png"), handle)
    }

    #[test]
    fn fetch_all_downloads_bytes() {
        let (url, server) = serve_once("200 OK", b"abc");
        let out = fetch_all(&[url]);
        assert_eq!(out[0].as_ref().expect("必须成功"), b"abc");
        server.join().expect("服务线程");
    }

    #[test]
    fn fetch_all_maps_http_error() {
        let (url, server) = serve_once("404 Not Found", b"nope");
        let out = fetch_all(&[url]);
        assert!(matches!(out[0], Err(FetchError::Request(_))));
        server.join().expect("服务线程");
    }
}
