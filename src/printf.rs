use std::char;

use gtmpl_value::{FromValue, Value};

use print_verb::print;

pub fn sprintf(s: &str, args: &[Value]) -> Result<String, String> {
    let tokens = tokenize(s)?;
    let mut fmt = String::new();
    let mut i = 0;
    let mut index = 0;
    for t in tokens {
        fmt.push_str(&s[i..t.start]);
        let (s, idx) = process_verb(&s[t.start + 1..t.end], t.typ, args, index)?;
        fmt.push_str(&s);
        index = idx;
        i = t.end + 1;
    }
    fmt.push_str(&s[i..]);
    Ok(fmt)
}

struct FormatArg {
    pub start: usize,
    pub end: usize,
    pub typ: char,
}

static TYPS: &'static str = "vVtTbcdoqxXUeEfFgGsp";

#[derive(Default)]
pub struct FormatParams {
    pub sharp: bool,
    pub zero: bool,
    pub plus: bool,
    pub minus: bool,
    pub space: bool,
    pub width: usize,
    pub precision: Option<usize>,
}

fn process_verb(
    s: &str,
    typ: char,
    args: &[Value],
    mut index: usize,
) -> Result<(String, usize), String> {
    let mut params = FormatParams::default();
    let mut complex = false;
    let mut pos = 0;
    for (i, c) in s.chars().enumerate() {
        match c {
            '#' => params.sharp = true,
            '0' => params.zero = true,
            '+' => params.plus = true,
            '-' => {
                params.minus = true;
                // Golang does not pad with zeros to the right.
                params.zero = false;
            }
            ' ' => params.space = true,
            _ => {
                pos = i;
                complex = true;
                break;
            }
        }
    }
    if complex {
        let mut after_index = false;
        let arg_num = parse_index(&s[pos..])?.map(|(i, till)| {
            pos += till;
            after_index = true;
            index = i;
            i
        });
        if s[pos..].starts_with('*') {
            pos += 1;
            let arg_num = arg_num.unwrap_or_else(|| {
                let i = index;
                index += 1;
                i
            });
            if let Some(width) = args.get(arg_num).and_then(|v| i64::from_value(v)) {
                if width < 0 {
                    params.minus = true;
                    // Golang does not pad with zeros to the right.
                    params.zero = false;
                }
                params.width = width.abs() as usize;
            }
            after_index = false;
        } else if let Some((width, till)) = parse_num(&s[pos..])? {
            if after_index {
                return Err("width after index (e.g. %[3]2d)".to_owned());
            }
            pos += till;
            params.width = width;
        }

        if pos + 1 < s.len() && s[pos..].starts_with('.') {
            pos += 1;
            if after_index {
                return Err("precision after index (e.g. %[3].2d)".to_owned());
            }

            let arg_num = parse_index(&s[pos..])?.map(|(i, till)| {
                pos += till;
                after_index = true;
                index = i;
                i
            });
            if s[pos..].starts_with('*') {
                pos += 1;
                let arg_num = arg_num.unwrap_or_else(|| {
                    let i = index;
                    index += 1;
                    i
                });
                if let Some(prec) = args.get(arg_num).and_then(|v| i64::from_value(v)) {
                    if prec < 0 {
                        params.precision = None;
                    }
                    params.precision = Some(prec.abs() as usize);
                }
            } else if let Some((prec, till)) = parse_num(&s[pos..])? {
                if after_index {
                    return Err("precision after index (e.g. %[3].2d)".to_owned());
                }
                pos += till;
                params.precision = Some(prec);
            }
        }
    }

    let arg_num = if let Some((i, _)) = parse_index(&s[pos..])? {
        index = i;
        i
    } else {
        let i = index;
        index += 1;
        i
    };

    if arg_num < args.len() {
        return print(&params, typ, &args[arg_num]).map(|s| (s, index));
    }
    Err(format!("unable to process verb: {}", s))
}

fn parse_index(s: &str) -> Result<Option<(usize, usize)>, String> {
    if s.starts_with('[') {
        let till = s.find(']').ok_or_else(|| format!("missing ] in {}", s))?;
        s[1..till]
            .parse::<usize>()
            .map(|u| Some((u - 1, till + 1)))
            .map_err(|e| format!("unable to parse index: {}", e))
    } else {
        Ok(None)
    }
}

fn parse_num(s: &str) -> Result<Option<(usize, usize)>, String> {
    let end = s.find(|c: char| !c.is_digit(10)).unwrap_or_else(|| s.len());
    if end > 0 {
        s[..end]
            .parse()
            .map(|u| Some((u, end)))
            .map_err(|e| format!("unable to parse width: {}", e))
    } else {
        Ok(None)
    }
}

fn tokenize(s: &str) -> Result<Vec<FormatArg>, String> {
    let mut iter = s.char_indices().peekable();
    let mut args = Vec::new();
    loop {
        let start = match iter.next() {
            None => break,
            Some((i, '%')) => i,
            _ => continue,
        };

        if let Some(&(_, '%')) = iter.peek() {
            iter.next();
            continue;
        }

        loop {
            match iter.next() {
                None => return Err(format!("unable to terminate format arg: {}", &s[start..])),
                Some((i, t)) if TYPS.contains(t) => {
                    args.push(FormatArg {
                        start,
                        end: i,
                        typ: t,
                    });
                    break;
                }
                _ => continue,
            };
        }
    }
    Ok(args)
}

pub fn params_to_chars(params: &FormatParams) -> (char, char, char, char, char) {
    (
        if params.sharp { '#' } else { '_' },
        if params.zero { '0' } else { '_' },
        if params.plus { '+' } else { '_' },
        if params.minus { '-' } else { '_' },
        if params.space { ' ' } else { '_' },
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sprinttf_to_format() {
        let s = sprintf("foo%v2000", &vec!["bar".into()]);
        assert!(s.is_ok());
        let s = s.unwrap();
        assert_eq!(s, r"foobar2000");

        let s = sprintf("%+0v", &vec![1.into()]);
        assert!(s.is_ok());
        let s = s.unwrap();
        assert_eq!(s, r"+1");
    }

    #[test]
    fn test_sprintf_fancy() {
        let s = sprintf("%+-#10c", &vec![10000.into()]);
        assert!(s.is_ok());
        let s = s.unwrap();
        assert_eq!(s, r"✐         ");

        let s = sprintf("%+-#10q", &vec![10000.into()]);
        assert!(s.is_ok());
        let s = s.unwrap();
        assert_eq!(s, r"'\u2710'  ");
    }

    #[test]
    fn test_sprintf_string_to_hex() {
        let s = sprintf("%x", &vec!["foobar2000".into()]);
        assert!(s.is_ok());
        let s = s.unwrap();
        assert_eq!(s, r"666f6f62617232303030");

        let s = sprintf("%X", &vec!["foobar2000".into()]);
        assert!(s.is_ok());
        let s = s.unwrap();
        assert_eq!(s, r"666F6F62617232303030");
    }

    #[test]
    fn test_sprintf_string_prec() {
        let s = sprintf("%.6s", &vec!["foobar2000".into()]);
        assert!(s.is_ok());
        let s = s.unwrap();
        assert_eq!(s, r"foobar");
    }

    #[test]
    fn test_sprintf_index() {
        let s = sprintf("%[1]v %v", &vec!["foo".into(), "bar".into(), 2000.into()]);
        assert!(s.is_ok());
        let s = s.unwrap();
        assert_eq!(s, r"foo bar");

        let s = sprintf(
            "%[2]v %v%[1]v %v%[1]v",
            &vec!["!".into(), "wtf".into(), "golang".into()],
        );
        assert!(s.is_ok());
        let s = s.unwrap();
        assert_eq!(s, r"wtf golang! wtf!");
    }

    #[test]
    fn test_sprintf_number() {
        let s = sprintf("foobar%d", &vec![2000.into()]);
        assert!(s.is_ok());
        let s = s.unwrap();
        assert_eq!(s, r"foobar2000");

        let s = sprintf("%+0d", &vec![1.into()]);
        assert!(s.is_ok());
        let s = s.unwrap();
        assert_eq!(s, r"+1");

        let s = sprintf("%+0b", &vec![5.into()]);
        assert!(s.is_ok());
        let s = s.unwrap();
        assert_eq!(s, r"+101");
    }

    #[test]
    fn test_tokenize() {
        let t = tokenize("foobar%6.2ffoobar");
        assert!(t.is_ok());
        let t = t.unwrap();
        assert_eq!(t.len(), 1);
        let a = &t[0];
        assert_eq!(a.start, 6);
        assert_eq!(a.end, 10);
        assert_eq!(a.typ, 'f');
    }

    #[test]
    fn test_tokenize_err() {
        let t = tokenize(" %6.2 ");
        assert!(t.is_err());
    }

    #[test]
    fn test_tokenize_none() {
        let t = tokenize(" foo %% bar ");
        assert!(t.is_ok());
        let t = t.unwrap();
        assert!(t.is_empty());
    }

    #[test]
    fn test_parse_index() {
        let x = parse_index("[12]");
        assert!(x.is_ok());
        let x = x.unwrap();
        // Go starts with 1 in stead of 0
        assert_eq!(x, Some((11, 4)));

        let x = parse_index("[12");
        assert!(x.is_err());

        let x = parse_index("*[12]");
        assert!(x.is_ok());
        let x = x.unwrap();
        assert_eq!(x, None);
    }
}
