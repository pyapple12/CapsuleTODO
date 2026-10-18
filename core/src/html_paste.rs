//! 剪贴板 HTML 解析（PATCH002.25）：把 `text/html` 剪贴板载荷拆成"文本段落 + 图片 src"
//! 序列。**纯逻辑零 IO**（网络下载与落盘在命令层）——TDD 直测。
//!
//! 设计理由：粘贴含图网页时，剪贴板 HTML 是唯一能带**多图**的载体（纯文本只带占位）；
//! 解析只做"能落到块模型"的最小映射：块级标签/换行 → 段落（文本块）、`<img src>` → 图块。
//! 不做 DOM 树、不解析 CSS——超出块模型表达能力的信息一律丢弃（KISS）。

/// 解析片段
#[derive(Debug, PartialEq, Eq)]
pub enum HtmlPart {
    /// 文本段落（已去标签、解实体、trim；空段落不产出）
    Text(String),
    /// 图片 src 原样（`data:` / `http(s)` / 其他 scheme 分流在命令层）
    Image(String),
}

/// 块级分隔标签：出现即视为段落边界（与浏览器"复制网页"的段落切分口径近似）
const BLOCK_TAGS: [&str; 12] = [
    "<br",
    "</p",
    "</div",
    "</li",
    "</tr",
    "</h1",
    "</h2",
    "</h3",
    "</h4",
    "</h5",
    "</h6",
    "</blockquote",
];

/// 解析 HTML → 片段序列（`<img>` 单独成段；文本按块级标签与换行切段）
pub fn parse(html: &str) -> Vec<HtmlPart> {
    let mut parts: Vec<HtmlPart> = Vec::new();
    let mut rest = html;
    loop {
        // 找下一个 <img …>：之前的文本先切段，再产出图块
        let Some(start) = find_ci(rest, "<img") else {
            push_text(&mut parts, rest);
            break;
        };
        push_text(&mut parts, &rest[..start]);
        let after = &rest[start..];
        let Some(end) = after.find('>') else {
            push_text(&mut parts, after);
            break;
        };
        if let Some(src) = attr_src(&after[..=end]) {
            parts.push(HtmlPart::Image(src));
        }
        rest = &after[end + 1..];
    }
    parts
}

/// 文本段 → 去标签 + 解实体 + 按块级标签与换行切段（trim 后非空的才产出）
fn push_text(parts: &mut Vec<HtmlPart>, chunk: &str) {
    for line in split_blocks(chunk) {
        let text = unescape(&strip_tags(&line));
        let text = text.trim();
        if !text.is_empty() {
            parts.push(HtmlPart::Text(text.to_string()));
        }
    }
}

/// 按块级标签与换行切段（标签本身换成换行，随后再 strip）
fn split_blocks(chunk: &str) -> Vec<String> {
    let mut normalized = chunk.to_string();
    for tag in BLOCK_TAGS {
        normalized = replace_ci(&normalized, tag, "\n");
    }
    normalized
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .split('\n')
        .map(|s| s.to_string())
        .collect()
}

/// 去标签（`<…>` 整体丢弃；`<!--…-->` 一并丢弃）
pub fn strip_tags(chunk: &str) -> String {
    let mut out = String::with_capacity(chunk.len());
    let mut depth = 0usize;
    let bytes = chunk.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            if chunk[i..].starts_with("<!--") {
                match chunk[i..].find("-->") {
                    Some(at) => {
                        i += at + 3;
                        continue;
                    }
                    None => break,
                }
            }
            depth += 1;
            i += 1;
            continue;
        }
        if bytes[i] == b'>' {
            depth = depth.saturating_sub(1);
            i += 1;
            continue;
        }
        if depth == 0 {
            // 非 ASCII 边界安全：按字符推进
            let ch = chunk[i..].chars().next().unwrap_or(' ');
            out.push(ch);
            i += ch.len_utf8();
        } else {
            i += 1;
        }
    }
    out
}

/// 解常见实体（网页复制里最常出现的六个；其余原样保留）
pub fn unescape(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// 从 `<img …>` 标签里取 `src` 值（双/单引号与无引号三态；取不到 → None）
fn attr_src(tag: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let at = lower.find("src")?;
    let rest = tag[at + 3..].trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    let mut chars = rest.chars();
    match chars.next()? {
        '"' | '\'' => {
            let quote = rest.chars().next()?;
            let body = &rest[1..];
            let end = body.find(quote)?;
            Some(body[..end].trim().to_string())
        }
        _ => {
            let end = rest
                .find(|c: char| c.is_whitespace() || c == '>')
                .unwrap_or(rest.len());
            Some(rest[..end].trim().to_string())
        }
    }
}

/// 大小写无关查找
fn find_ci(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .to_ascii_lowercase()
        .find(&needle.to_ascii_lowercase())
}

/// 大小写无关替换（全量）
fn replace_ci(haystack: &str, needle: &str, with: &str) -> String {
    let lower = haystack.to_ascii_lowercase();
    let needle_lower = needle.to_ascii_lowercase();
    let mut out = String::with_capacity(haystack.len());
    let mut from = 0usize;
    while let Some(at) = lower[from..].find(&needle_lower) {
        let at = from + at;
        out.push_str(&haystack[from..at]);
        out.push_str(with);
        from = at + needle.len();
    }
    out.push_str(&haystack[from..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_text_and_images_in_order() {
        let html = "<div>第一段<br>第二行</div><p><img src=\"https://a/b.png\">图后文字</p>";
        assert_eq!(
            parse(html),
            vec![
                HtmlPart::Text("第一段".to_string()),
                HtmlPart::Text("第二行".to_string()),
                HtmlPart::Image("https://a/b.png".to_string()),
                HtmlPart::Text("图后文字".to_string()),
            ]
        );
    }

    #[test]
    fn keeps_data_url_and_single_quoted_src() {
        let html = "<img src='data:image/png;base64,AA=='><img src=data:image/jpeg;base64,BB>";
        assert_eq!(
            parse(html),
            vec![
                HtmlPart::Image("data:image/png;base64,AA==".to_string()),
                HtmlPart::Image("data:image/jpeg;base64,BB".to_string()),
            ]
        );
    }

    #[test]
    fn unescapes_entities_and_drops_empty_paragraphs() {
        let html = "<p>&nbsp;</p><p>a &amp; b &lt;c&gt;</p><p></p>";
        assert_eq!(parse(html), vec![HtmlPart::Text("a & b <c>".to_string())]);
    }

    #[test]
    fn strip_tags_handles_comments_and_attributes() {
        assert_eq!(strip_tags("x<!-- 注释 -->y<b class=\"z\">B</b>"), "xyB");
    }
}
