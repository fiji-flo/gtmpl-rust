use gtmpl_value::Value;
use std::char;

pub fn unquote_char(s: &str, quote: char) -> Option<char> {
    if s.len() < 2 || !s.starts_with(quote) || !s.ends_with(quote) {
        return None;
    }
    let raw = &s[1..s.len() - 1];
    match unqote(raw) {
        Some((c, l)) => {
            if l == raw.len() {
                c.chars().next()
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn unquote_str(s: &str) -> Option<String> {
    if s.len() < 2 {
        return None;
    }
    let quote = &s[0..1];
    if !s.ends_with(quote) {
        return None;
    }
    let mut r = String::new();
    let raw = &s[1..s.len() - 1];
    let mut i = 0;
    while i < raw.len() {
        match unqote(&raw[i..]) {
            Some((c, len)) => {
                r += &c;
                i += len;
            }
            None => return None,
        }
    }
    Some(r)
}

fn unqote(raw: &str) -> Option<(String, usize)> {
    if raw.starts_with('\\') {
        match &raw[..2] {
            r"\x" => extract_bytes_x(raw),
            r"\U" => extract_bytes_u32(raw),
            r"\u" => extract_bytes_u16(raw),
            r"\b" => Some(('\u{0008}'.to_string(), 2)),
            r"\f" => Some(('\u{000C}'.to_string(), 2)),
            r"\n" => Some(('\n'.to_string(), 2)),
            r"\r" => Some(('\r'.to_string(), 2)),
            r"\t" => Some(('\t'.to_string(), 2)),
            r"\'" => Some(('\''.to_string(), 2)),
            r#"\""# => Some(('\"'.to_string(), 2)),
            r#"\\"# => Some(('\\'.to_string(), 2)),
            _ => None,
        }
    } else {
        get_char(raw)
    }
}

fn get_char(s: &str) -> Option<(String, usize)> {
    s.char_indices()
        .next()
        .map(|(i, c)| (c.to_string(), i + c.len_utf8()))
}

fn extract_bytes_u32(s: &str) -> Option<(String, usize)> {
    if s.len() != 10 {
        return None;
    }
    u32::from_str_radix(&s[2..10], 16)
        .ok()
        .and_then(char::from_u32)
        .map(|c| (c.to_string(), 10))
}

fn extract_bytes_u16(s: &str) -> Option<(String, usize)> {
    let mut bytes = vec![];
    let mut i = 0;
    while s.len() > i && s.starts_with(r"\u") && s[i..].len() >= 6 {
        match u16::from_str_radix(&s[(i + 2)..(i + 6)], 16) {
            Ok(x) => bytes.push(x),
            _ => {
                return None;
            }
        };
        i += 6;
    }
    String::from_utf16(&bytes).ok().map(|s| (s, i))
}

fn extract_bytes_x(s: &str) -> Option<(String, usize)> {
    let mut bytes = vec![];
    let mut i = 0;
    while s.len() > i && s.starts_with(r"\x") && s[i..].len() >= 4 {
        match u8::from_str_radix(&s[(i + 2)..(i + 4)], 16) {
            Ok(x) => bytes.push(x),
            _ => {
                return None;
            }
        };
        i += 4;
    }
    String::from_utf8(bytes).ok().map(|s| (s, i))
}

/// Returns
pub fn is_true(val: &Value) -> bool {
    match *val {
        Value::Bool(ref b) => *b,
        Value::String(ref s) => !s.is_empty(),
        Value::Array(ref a) => !a.is_empty(),
        Value::Object(ref o) => !o.is_empty(),
        Value::Map(ref m) => !m.is_empty(),
        Value::Function(_) => true,
        Value::NoValue | Value::Nil => false,
        Value::Number(ref n) => n.as_u64().map(|u| u != 0).unwrap_or_else(|| true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unquote_char() {
        let s = "'→'";
        let c = unquote_char(s, '\'');
        assert_eq!(c, Some('→'));
        let s = "'→←'";
        let c = unquote_char(s, '\'');
        assert_eq!(c, None);
        let s = r"'\xf0\x9f\x92\xa9'";
        let c = unquote_char(s, '\'');
        assert_eq!(c, Some('💩'));
        let s = r"'\xf0\x9f\x92\xa'";
        let c = unquote_char(s, '\'');
        assert_eq!(c, None);
        let s = r"'\xf0\x9f\x92\xa99'";
        let c = unquote_char(s, '\'');
        assert_eq!(c, None);
        let s = r"'\u263a'";
        let c = unquote_char(s, '\'');
        assert_eq!(c, Some('☺'));
        let s = r"'\uD83D\uDCA9'";
        let c = unquote_char(s, '\'');
        assert_eq!(c, Some('💩'));
        let s = r"'\uD83\uDCA9'";
        let c = unquote_char(s, '\'');
        assert_eq!(c, None);
        let s = r"'\uD83D\uDCA9B'";
        let c = unquote_char(s, '\'');
        assert_eq!(c, None);
        let s = r"'\U0001F4A9'";
        let c = unquote_char(s, '\'');
        assert_eq!(c, Some('💩'));
        let s = r"'\U0001F4A'";
        let c = unquote_char(s, '\'');
        assert_eq!(c, None);
        let s = r"'\U0001F4A99'";
        let c = unquote_char(s, '\'');
        assert_eq!(c, None);
    }

    #[test]
    fn test_unquote_str() {
        let s = r#""Fran & Freddie's Diner""#;
        let u = unquote_str(s);
        assert_eq!(u, Some("Fran & Freddie's Diner".to_owned()));
        let s = r#""Fran & Freddie's Diner\t\u263a""#;
        let u = unquote_str(s);
        assert_eq!(u, Some("Fran & Freddie's Diner\t☺".to_owned()));
    }

    #[test]
    fn test_is_true() {
        let t = Value::from(1i8);
        assert!(is_true(&t));
        let t = Value::from(0u32);
        assert!(!is_true(&t));
    }
}
