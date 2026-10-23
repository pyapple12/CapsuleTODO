//! 剪贴板 HTML 解析（PATCH002.25）：把 `text/html` 剪贴板载荷拆成"文本段落 + 图片 src"
//! 序列。**纯逻辑零 IO**（网络下载与落盘在命令层）——TDD 直测。
//!
//! 设计理由：粘贴含图网页时，剪贴板 HTML 是唯一能带**多图**的载体（纯文本只带占位）；
//! 解析只做"能落到块模型"的最小映射：块级标签/换行 → 段落（文本块）、`<img src>` → 图块。
//! 不做 DOM 树、不解析 CSS——超出块模型表达能力的信息一律丢弃（KISS）。

/// 解析片段
#[derive(Debug, PartialEq, Eq)]
pub enum HtmlPart {
    /// 文本段落（已去标签、解实体；行首/行尾空白保留，纯空白行不产出）
    Text(String),
    /// 图片 src 原样（`data:` / `http(s)` / 其他 scheme 分流在命令层）
    Image(String),
}

/// 块级分隔标签：出现即视为段落边界（与浏览器"复制网页"的段落切分口径近似）。
/// `<br` 不在此表：自闭合 `<br/>` / `<br />` 由 `replace_br_tags` 统一吃掉，避免
/// 只替换 `<br` 后把 `/>` 残留成文本。
const BLOCK_TAGS: [&str; 11] = [
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

/// HTML 解析前的字节上限（FIX014.20）：超出部分截断到 UTF-8 边界。
/// 详情笔记本身另有 10,000 字符上限；这里防超大标记把解析拖成 O(N²)。
const MAX_HTML_BYTES: usize = 2 * 1024 * 1024;

/// 解析 HTML → 片段序列（`<img>` 单独成段；文本按块级标签与换行切段）。
/// FIX014.20：先跳过注释 / script / style，再做标签扫描；超长输入先截断。
pub fn parse(html: &str) -> Vec<HtmlPart> {
    let limited = truncate_utf8(html, MAX_HTML_BYTES);
    let cleaned = strip_non_content(limited);
    let mut parts: Vec<HtmlPart> = Vec::new();
    let mut rest = cleaned.as_str();
    loop {
        // 找下一个 <img …>：之前的文本先切段，再产出图块
        let Some(start) = find_img_tag(rest) else {
            push_text(&mut parts, rest);
            break;
        };
        push_text(&mut parts, &rest[..start]);
        let after = &rest[start..];
        let Some(end) = find_tag_end(after) else {
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

/// 截到 UTF-8 字符边界；超长 HTML 只解析前缀。
fn truncate_utf8(text: &str, limit: usize) -> &str {
    if text.len() <= limit {
        return text;
    }
    let mut end = limit;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// 查找标签结束 `>`（FIX014.20：跳过引号内的 `>`，避免 `title="a > b"` 提前截断）。
fn find_tag_end(tag: &str) -> Option<usize> {
    let bytes = tag.as_bytes();
    let mut quote: Option<u8> = None;
    for (i, &byte) in bytes.iter().enumerate() {
        match quote {
            Some(q) if byte == q => quote = None,
            Some(_) => {}
            None if matches!(byte, b'"' | b'\'') => quote = Some(byte),
            None if byte == b'>' => return Some(i),
            None => {}
        }
    }
    None
}

/// FIX015.17：标签名边界——后面只能是空白、`>`、`/` 或输入结束。
/// 防止 `<scripture>` / `</picture>` 命中 `<script` / `</p` 前缀。
fn is_tag_boundary(bytes: &[u8], at: usize) -> bool {
    bytes
        .get(at)
        .map(|byte| byte.is_ascii_whitespace() || matches!(byte, b'>' | b'/'))
        .unwrap_or(true)
}

/// FIX016.6：查找真正的 `<img` / `<image>` 标签起点；跳过 `<img-foo>`、
/// `<imgsomething>` 等同前缀自定义标签。`<image>` 是 HTML 的 `<img` 别名。
fn find_img_tag(haystack: &str) -> Option<usize> {
    let bytes = haystack.as_bytes();
    let mut from = 0usize;
    while from < haystack.len() {
        let img = find_ci(&haystack[from..], "<img").map(|rel| from + rel);
        let image = find_ci(&haystack[from..], "<image").map(|rel| from + rel);
        let at = match (img, image) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => return None,
        };
        // 同一位置优先尝试更长的 <image> 别名；失败后再按 <img 边界判断。
        if haystack[at..].len() >= 6 && is_tag_boundary(bytes, at + 6) {
            return Some(at);
        }
        if haystack[at..].len() >= 4 && is_tag_boundary(bytes, at + 4) {
            return Some(at);
        }
        from = at + 1;
    }
    None
}

/// FIX015.17：查找带标签名边界的闭合标签起点（`</script` / `</style`）。
/// 跳过 `</scripture>` 这类同前缀自定义标签，避免脚本正文提前结束。
fn find_closing_tag(haystack: &str, name: &str) -> Option<usize> {
    let needle = format!("</{name}");
    let bytes = haystack.as_bytes();
    let mut from = 0usize;
    while from < haystack.len() {
        let rel = find_ci(&haystack[from..], &needle)?;
        let at = from + rel;
        if is_tag_boundary(bytes, at + needle.len()) {
            return Some(at);
        }
        from = at + 1;
    }
    None
}

/// 非正文块类型（注释 / 脚本 / 样式）。
#[derive(Clone, Copy)]
enum NonContent {
    Comment,
    Script,
    Style,
}

/// 单次扫描找最近的非正文块起点（FIX014.47：替代每轮 3 次全余量搜索，
/// 避免大量注释/脚本把解析拖成二次方）。
fn find_non_content(html: &str) -> Option<(usize, NonContent)> {
    let bytes = html.as_bytes();
    for (i, _) in bytes.iter().enumerate() {
        if bytes[i] != b'<' {
            continue;
        }
        let tail = &bytes[i..];
        if tail.starts_with(b"<!--") {
            return Some((i, NonContent::Comment));
        }
        if tail.len() >= 7
            && tail[..7].eq_ignore_ascii_case(b"<script")
            && is_tag_boundary(bytes, i + 7)
        {
            return Some((i, NonContent::Script));
        }
        if tail.len() >= 6
            && tail[..6].eq_ignore_ascii_case(b"<style")
            && is_tag_boundary(bytes, i + 6)
        {
            return Some((i, NonContent::Style));
        }
    }
    None
}

/// 去掉注释、`<script>` / `<style>` 整段内容（FIX014.20：脚本里的 `<img>` 不得成图，
/// 样式/脚本正文不得进入笔记）。未闭合时丢弃剩余内容，避免把脚本正文当文本。
fn strip_non_content(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    loop {
        let Some((start, kind)) = find_non_content(rest) else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..start]);
        match kind {
            NonContent::Comment => {
                let after = &rest[start + 4..];
                let Some(end) = after.find("-->") else {
                    break;
                };
                rest = &after[end + 3..];
            }
            NonContent::Script => {
                let Some(end) = find_closing_tag(&rest[start..], "script") else {
                    break;
                };
                let tail = &rest[start + end..];
                // FIX014.44：连闭合标签的 `>` 一起吃掉；否则会留下 `>` 污染正文。
                let Some(close) = find_tag_end(tail) else {
                    break;
                };
                rest = &tail[close + 1..];
            }
            NonContent::Style => {
                let Some(end) = find_closing_tag(&rest[start..], "style") else {
                    break;
                };
                let tail = &rest[start + end..];
                let Some(close) = find_tag_end(tail) else {
                    break;
                };
                rest = &tail[close + 1..];
            }
        }
    }
    out
}

/// 文本段 → 去标签 + 解实体 + 按块级标签与换行切段。
/// FIX015.23：空白是内容（代码块/缩进粘贴）；只用 `trim().is_empty()` 判空，
/// 入块保留行首/行尾空格，不再二次 trim。
fn push_text(parts: &mut Vec<HtmlPart>, chunk: &str) {
    for line in split_blocks(chunk) {
        let text = unescape(&strip_tags(&line));
        let text = text.trim_end_matches('\r');
        if !text.trim().is_empty() {
            parts.push(HtmlPart::Text(text.to_string()));
        }
    }
}

/// 按块级标签与换行切段（标签本身换成换行，随后再 strip）。
/// FIX014.20：`<br>` / `<br/>` / `<br />` 整体替换成换行，不再留下 `/>` 文本。
fn split_blocks(chunk: &str) -> Vec<String> {
    // FIX014.44：闭合块标签连 `>` 一起换行，`strip_tags` 才能保留正文里的裸 `>`。
    let normalized = replace_block_tags(&replace_br_tags(chunk));
    normalized
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .split('\n')
        .map(|s| s.to_string())
        .collect()
}

/// 把 `<br ...>` 变体整体替换为换行；属性/大小写/自闭合斜杠都吃掉。
fn replace_br_tags(chunk: &str) -> String {
    let mut out = String::with_capacity(chunk.len());
    let mut rest = chunk;
    while let Some(at) = find_ci(rest, "<br") {
        let after = &rest[at..];
        let next = after.as_bytes().get(3).copied();
        let is_br = next.is_some_and(|b| b.is_ascii_whitespace() || b == b'/' || b == b'>');
        if !is_br {
            out.push_str(&rest[..at + 3]);
            rest = &after[3..];
            continue;
        }
        out.push_str(&rest[..at]);
        let Some(end) = find_tag_end(after) else {
            // 未闭合 `<br`：保持原样，交给后续 strip_tags 兜底
            out.push_str(after);
            return out;
        };
        out.push('\n');
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// 单次扫描找最近的块级闭合标签起点（FIX014.47：避免每个标签都重新扫全部 11 个前缀）。
fn find_next_block_tag(chunk: &str) -> Option<usize> {
    let bytes = chunk.as_bytes();
    for (i, _) in bytes.iter().enumerate() {
        if bytes[i] != b'<' || bytes.get(i + 1) != Some(&b'/') {
            continue;
        }
        let tail = &bytes[i + 2..];
        for tag in BLOCK_TAGS {
            let name = &tag.as_bytes()[2..];
            if tail.len() >= name.len()
                && tail[..name.len()].eq_ignore_ascii_case(name)
                && is_tag_boundary(bytes, i + 2 + name.len())
            {
                return Some(i);
            }
        }
    }
    None
}

/// 把块级闭合标签（含其 `>`）整体替换为换行。FIX014.44：此前只替换 `</p` 前缀，
/// 残留的 `>` 逼得 `strip_tags` 跳过所有顶层 `>`，会误删正文里的比较符。
fn replace_block_tags(chunk: &str) -> String {
    let mut out = String::with_capacity(chunk.len());
    let mut rest = chunk;
    loop {
        let Some(at) = find_next_block_tag(rest) else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..at]);
        let after = &rest[at..];
        let Some(end) = find_tag_end(after) else {
            // 未闭合：保持原样，交给后续 strip_tags 兜底。
            out.push_str(after);
            break;
        };
        out.push('\n');
        rest = &after[end + 1..];
    }
    out
}

/// 去标签（`<…>` 整体丢弃；`<!--…-->` 一并丢弃）。
/// FIX014.20：标签结束由 `find_tag_end` 引号感知扫描，不再把 `title="a > b"` 的 `>`
/// 当成标签结束。
pub fn strip_tags(chunk: &str) -> String {
    let mut out = String::with_capacity(chunk.len());
    let mut i = 0usize;
    while i < chunk.len() {
        if chunk[i..].starts_with("<!--") {
            match chunk[i..].find("-->") {
                Some(at) => {
                    i += at + 3;
                    continue;
                }
                None => break,
            }
        }
        if chunk.as_bytes()[i] == b'<' {
            let next = chunk.as_bytes().get(i + 1).copied();
            let looks_like_tag = next
                .is_some_and(|b| b == b'/' || b == b'!' || b == b'?' || b.is_ascii_alphabetic());
            // 裸 `<`（如 “1 < 2”“<3”）不是标签，保留并继续；否则未闭合标签
            // 会把后面的全部正文一起吞掉。
            if !looks_like_tag {
                out.push('<');
                i += 1;
                continue;
            }
            let Some(end) = find_tag_end(&chunk[i..]) else {
                out.push_str(&chunk[i..]);
                break;
            };
            i += end + 1;
            continue;
        }
        // 非 ASCII 边界安全：按字符推进
        let ch = chunk[i..].chars().next().unwrap_or(' ');
        out.push(ch);
        i += ch.len_utf8();
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

/// 属性名字符（按 HTML 属性名边界解析，避免 `srcset` / `data-src` 误命中）
fn attr_name_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
}

/// 从 `<img …>` 标签里按属性名取 `src` 值（双/单引号与无引号三态；取不到 → None）。
/// **FIX014.7**：不再用裸子串 `find("src")`——`srcset` 前置或 `alt="src"` 都会误命中；
/// 这里顺序扫描属性名与值，只接受完整属性名 `src`。
fn attr_src(tag: &str) -> Option<String> {
    let bytes = tag.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        // 跳过标签名、空白与分隔符，定位属性名
        while i < bytes.len()
            && (bytes[i].is_ascii_whitespace() || matches!(bytes[i], b'<' | b'/' | b'>'))
        {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let name_start = i;
        while i < bytes.len() && attr_name_char(bytes[i]) {
            i += 1;
        }
        if i == name_start {
            i += 1; // 非属性名字符（如 `!`）——跳过，继续
            continue;
        }
        let name = &tag[name_start..i];
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            continue; // 无值属性（如 `disabled`）——继续找下一个属性
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            return None;
        }
        let value = if matches!(bytes[i], b'"' | b'\'') {
            let quote = bytes[i];
            i += 1;
            let value_start = i;
            while i < bytes.len() && bytes[i] != quote {
                i += 1;
            }
            let value = &tag[value_start..i];
            if i < bytes.len() {
                i += 1; // 跳过闭合引号
            }
            value
        } else {
            let value_start = i;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' {
                i += 1;
            }
            &tag[value_start..i]
        };
        if name.eq_ignore_ascii_case("src") {
            return Some(unescape(value.trim()).replace(['\n', '\r'], ""));
        }
    }
    None
}

/// 大小写无关查找（FIX014.20：字节级比较，不再每次整段 `to_ascii_lowercase`）。
/// needle 只用于 HTML 标签/属性名（ASCII）；非 ASCII 文本按原字节比较。
fn find_ci(haystack: &str, needle: &str) -> Option<usize> {
    let needle = needle.as_bytes();
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .as_bytes()
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle))
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
    fn unescapes_entities_in_img_src() {
        let html = "<img src=\"https://a/b.png?x=1&amp;y=2\">";
        assert_eq!(
            parse(html),
            vec![HtmlPart::Image("https://a/b.png?x=1&y=2".to_string())]
        );
    }

    #[test]
    fn strips_newlines_from_img_src() {
        let html = "<img src=\"https://a/b\n.png\">";
        assert_eq!(
            parse(html),
            vec![HtmlPart::Image("https://a/b.png".to_string())]
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

    #[test]
    fn strip_tags_ignores_gt_inside_quoted_attributes() {
        assert_eq!(
            strip_tags("<span title=\"a > b\">正文</span>"),
            "正文",
            "引号内 > 不得提前结束标签"
        );
    }

    #[test]
    fn strip_tags_preserves_bare_gt_in_text() {
        // FIX014.44：正文比较符不得被当成闭合标签尾巴丢掉。
        assert_eq!(strip_tags("1 > 0"), "1 > 0");
        assert_eq!(strip_tags("<b>1 > 0</b>"), "1 > 0");
    }

    #[test]
    fn parse_preserves_bare_gt_in_text() {
        assert_eq!(
            parse("<p>1 > 0</p>"),
            vec![HtmlPart::Text("1 > 0".to_string())]
        );
    }

    #[test]
    fn strip_tags_preserves_bare_lt_in_text() {
        assert_eq!(strip_tags("1 < 2"), "1 < 2");
        assert_eq!(strip_tags("<3"), "<3");
        assert_eq!(strip_tags("<p>1 < 2</p>"), "1 < 2");
    }

    #[test]
    fn split_blocks_consumes_all_br_variants() {
        assert_eq!(
            parse("甲<br>乙<br/>丙<br />丁"),
            vec![
                HtmlPart::Text("甲".to_string()),
                HtmlPart::Text("乙".to_string()),
                HtmlPart::Text("丙".to_string()),
                HtmlPart::Text("丁".to_string()),
            ]
        );
    }

    #[test]
    fn parse_skips_script_style_and_comments() {
        let html = "<p>前</p><script>var s = \"<img src='https://evil/x.png'>\";</script>\
                    <style>.x{content:\"<img src='https://evil/y.png'>\"}</style>\
                    <!-- <img src='https://evil/z.png'> --><p>后</p>";
        assert_eq!(
            parse(html),
            vec![
                HtmlPart::Text("前".to_string()),
                HtmlPart::Text("后".to_string()),
            ],
            "脚本/样式/注释里的 img 不得成图，正文也不得混入"
        );
    }

    #[test]
    fn parse_does_not_match_img_custom_tag_prefixes() {
        // FIX016.6：<img-foo>/<imgsomething> 不是图片；<image> 是合法别名。
        assert_eq!(
            parse("<p>前<img-foo src='x.png'>后</p>"),
            vec![HtmlPart::Text("前后".to_string())]
        );
        assert_eq!(
            parse("<p>前<imgsomething src='x.png'>后</p>"),
            vec![HtmlPart::Text("前后".to_string())]
        );
        assert_eq!(
            parse("<image src='x.png'>"),
            vec![HtmlPart::Image("x.png".to_string())]
        );
        assert_eq!(
            parse("<IMG SRC='x.png'>"),
            vec![HtmlPart::Image("x.png".to_string())]
        );
    }

    #[test]
    fn parse_does_not_match_custom_tag_prefixes() {
        // FIX015.17：<scripture>/<stylex> 不是 script/style，</picture> 不是 </p>。
        assert_eq!(
            parse("<scripture>正文</scripture>"),
            vec![HtmlPart::Text("正文".to_string())]
        );
        assert_eq!(
            parse("<stylex>样式</stylex>"),
            vec![HtmlPart::Text("样式".to_string())]
        );
        assert_eq!(
            parse("<p>前</picture>后</p>"),
            vec![HtmlPart::Text("前后".to_string())],
            "自定义闭合标签不得被当成 </p> 分段"
        );
        assert_eq!(
            parse("<script>var x='</scripture>';</script><p>正文</p>"),
            vec![HtmlPart::Text("正文".to_string())],
            "脚本内 </scripture> 不得提前结束 script"
        );
    }

    #[test]
    fn parse_preserves_indentation_and_spaces() {
        // FIX015.23：HTML 文本行不再 trim，缩进/行尾空格按原样进入块。
        assert_eq!(
            parse("<p>  缩进  </p>"),
            vec![HtmlPart::Text("  缩进  ".to_string())]
        );
        assert_eq!(
            parse("<pre>  code\n    next</pre>"),
            vec![
                HtmlPart::Text("  code".to_string()),
                HtmlPart::Text("    next".to_string()),
            ]
        );
        assert!(parse("<p>   \n   </p>").is_empty(), "纯空白行不产出");
    }

    #[test]
    fn parse_truncates_oversized_html_at_utf8_boundary() {
        // 截断点之后放可观察标记，证明上限确实生效；ASCII 与多字节边界各一次。
        let html = format!(
            "<p>{}</p><img src=\"after-limit.png\">",
            "a".repeat(MAX_HTML_BYTES)
        );
        let parts = parse(&html);
        assert!(
            parts.iter().all(|part| !matches!(
                part,
                HtmlPart::Image(src) if src == "after-limit.png"
            )),
            "超过上限的尾部图片不得被解析"
        );

        let body = "a".repeat(MAX_HTML_BYTES - 2);
        let html = format!("<p>{body}é</p><img src=\"after-limit.png\">");
        let parts = parse(&html);
        assert!(
            parts.iter().all(|part| !matches!(
                part,
                HtmlPart::Image(src) if src == "after-limit.png"
            )),
            "UTF-8 边界截断后不得 panic 或带入尾部标记"
        );
    }

    #[test]
    fn attr_src_ignores_srcset_and_alt_values() {
        // FIX014.7：属性名按边界解析，srcset 前置 / alt 值里的 src 不得误命中
        assert_eq!(
            parse("<img srcset=\"a.png 1x, b.png 2x\" src=\"real.png\">"),
            vec![HtmlPart::Image("real.png".to_string())]
        );
        assert_eq!(
            parse("<img alt=\"src\" src=\"real.png\">"),
            vec![HtmlPart::Image("real.png".to_string())]
        );
        assert_eq!(
            parse("<img data-src=\"placeholder.png\" SRC='real.png'>"),
            vec![HtmlPart::Image("real.png".to_string())]
        );
        assert_eq!(
            parse("<img\nsrc = real.png>"),
            vec![HtmlPart::Image("real.png".to_string())]
        );
        assert_eq!(
            parse("<img src=\"\">"),
            vec![HtmlPart::Image(String::new())]
        );
    }
}
