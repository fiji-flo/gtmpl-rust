use std::char;

pub fn unquote_char(s: &str, quote: char) -> Option<char> {
    if s.len() < 2 || !s.starts_with(quote) || !s.ends_with(quote) {
        return None;
    }
    let raw = &s[1..(s.len() - 1)];
    if raw.starts_with('\\') {
        match &raw[..2] {
            r"\x" => {
                extract_bytes_x(raw)
                    .and_then(|b| String::from_utf8(b).ok())
                    .and_then(|s| get_char(&s))
            }
            r"\U" => extract_bytes_u32(&raw[2..]).and_then(char::from_u32),
            r"\u" => {
                extract_bytes_u16(raw)
                    .and_then(|b| String::from_utf16(&b).ok())
                    .and_then(|s| get_char(&s))
            }
            _ => None,
        }
    } else {
        get_char(raw)
    }
}

fn get_char(s: &str) -> Option<char> {
    if s.chars().count() != 1 {
        return None;
    }
    s.chars().next()
}

fn extract_bytes_u32(s: &str) -> Option<u32> {
    if s.len() != 8 {
        return None;
    }
    u32::from_str_radix(s, 16).ok()
}

fn extract_bytes_u16(s: &str) -> Option<Vec<u16>> {
    let by6 = s.len() / 6;
    if s.len() % 6 != 0 {
        return None;
    }
    let mut bytes = vec![];
    for i in 0..by6 {
        let j = i * 6;
        if &s[j..(j + 2)] != r"\u" {
            return None;
        }
        match u16::from_str_radix(&s[(j + 2)..(j + 6)], 16) {
            Ok(x) => bytes.push(x),
            _ => {
                return None;
            }
        }
    }
    Some(bytes)
}

fn extract_bytes_x(s: &str) -> Option<Vec<u8>> {
    let by4 = s.len() / 4;
    if s.len() % 4 != 0 {
        return None;
    }
    let mut bytes = vec![];
    for i in 0..by4 {
        let j = i * 4;
        if &s[j..(j + 2)] != r"\x" {
            return None;
        }
        match u8::from_str_radix(&s[(j + 2)..(j + 4)], 16) {
            Ok(x) => bytes.push(x),
            _ => {
                return None;
            }
        }
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_raw_bytes() {
        let s = r"\xab\x12";
        let e = extract_bytes_x(s);
        assert_eq!(e, Some(vec![0xab, 0x12]));
        let s = r"\xde\xad\xbe\xef";
        let e = extract_bytes_x(s);
        assert_eq!(e, Some(vec![0xde, 0xad, 0xbe, 0xef]));
    }

    fn test_extract_raw_bytes_fails() {
        let s = r"\xab\x123";
        let e = extract_bytes_x(s);
        assert!(e.is_none());
        let s = r"\xab\x1h";
        let e = extract_bytes_x(s);
        assert!(e.is_none());
        let s = r"\xab\a1b";
        let e = extract_bytes_x(s);
        assert!(e.is_none());
    }

    #[test]
    fn test_unquote_char() {
        let s = "'→'";
        let c = unquote_char(s, '\'');
        assert_eq!(c, Some('→'));
        let s = r"'\xf0\x9f\x92\xa9'";
        let c = unquote_char(s, '\'');
        assert_eq!(c, Some('💩'));
        let s = r"'\uD83D\uDCA9'";
        let c = unquote_char(s, '\'');
        assert_eq!(c, Some('💩'));
        let s = r"'\U0001F4A9'";
        let c = unquote_char(s, '\'');
        assert_eq!(c, Some('💩'));
    }
}
