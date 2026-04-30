///
/// 一个小型的 parser，用于将 embedLuaFile 给替换成目标 lua 文件！
///
use std::collections::HashMap;

// 开始替换！
pub fn parse_embed(main_name: &str, content: &str, files: &HashMap<String, Vec<u8>>) -> String {
    // 率先将 files 给解决成全部都是纯文本的情况！（排除图片、音频和视频等二进制文件）
    let mut r_map: HashMap<String, String> = HashMap::new();
    for (k, v) in files {
        if k == main_name {
            continue;
        }
        if k.ends_with(".lua") {
            let str = match String::from_utf8(v.clone()) {
                Ok(s) => s,
                Err(_) => continue,
            };
            r_map.insert(k.clone(), str);
        }
    }
    let mut result = Vec::new();
    // 遍历输出每一行！
    for line in content.lines() {
        if let Some(replaced) = try_parse_embed_line(line, &r_map) {
            result.push(replaced);
        } else {
            result.push(line.to_string());
        }
    }
    result.join("\n")
}
// 判断 lua 文件的每一行是否包含 embedLuaFile 这个词。
fn try_parse_embed_line(line: &str, files: &HashMap<String, String>) -> Option<String> {
    // 找到 embedLuaFile 这个词（单词边界检查）
    let pos = find_word(line, "embedLuaFile")?;
    let rest = &line[pos + "embedLuaFile".len()..];
    let mut chars = rest.chars().peekable();
    // 跳过空白
    skip_ws(&mut chars);
    // 必须是 (
    if chars.next() != Some('(') {
        return None;
    }
    // 跳过空白
    skip_ws(&mut chars);
    // 读取字符串参数
    let filename = read_string_literal(&mut chars)?;
    // 跳过空白
    skip_ws(&mut chars);
    // 如果是逗号，说明有多个参数，不处理
    if chars.peek() == Some(&',') {
        return None;
    }
    // 必须是 )
    if chars.next() != Some(')') {
        return None;
    }
    // 从 HashMap 获取替换内容
    files.get(&filename).cloned()
}
/// 查找完整单词，避免匹配到 myembedLuaFile 或 embedLuaFiles
fn find_word(s: &str, word: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let word_bytes = word.as_bytes();
    let mut i = 0;
    while i + word_bytes.len() <= bytes.len() {
        if &bytes[i..i + word_bytes.len()] == word_bytes {
            let prev_ok = i == 0 || !is_ident_char(bytes[i - 1]);
            let next_ok =
                i + word_bytes.len() >= bytes.len() || !is_ident_char(bytes[i + word_bytes.len()]);
            if prev_ok && next_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}
/// 判断当前的字符是否为英文、数字和下划线。
fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}
// 跳过空白字符
fn skip_ws(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while chars.peek().map_or(false, |c| c.is_whitespace()) {
        chars.next();
    }
}
/// 实现简单的字符串解析
/// 由于在真实的路径里面不可能出现双引号或者单引号作为路径名，因此可以省略判断 \" 或者 \' 这种可能性。
fn read_string_literal(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<String> {
    let bracket_type = chars.next();
    if bracket_type != Some('"') && bracket_type != Some('\'') {
        return None;
    }
    let bracket_type = bracket_type.unwrap();
    let mut s = String::new();
    loop {
        match chars.next() {
            Some(c) => {
                if c == bracket_type {
                    return Some(s);
                } else {
                    s.push(c)
                }
            }
            None => return None,
        }
    }
}
