use std::char;
use std::fmt;

use crate::error::PrintError;
use crate::printf::{params_to_chars, FormatParams};

use gtmpl_value::Value;

/// Print a verb like golang's printf.
pub fn print(p: &FormatParams, typ: char, val: &Value) -> Result<String, PrintError> {
    match *val {
        Value::Number(ref n) if n.as_u64().is_some() => {
            let u = n.as_u64().unwrap();
            Ok(match typ {
                'b' => printf_b(p, u),
                'd' | 'v' => printf_generic(p, u),
                'o' => printf_o(p, u),
                'c' => {
                    let c = char::from_u32(u as u32).ok_or(PrintError::NotAValidChar(u as i128))?;
                    printf_generic(p, c)
                }
                'q' => {
                    let c = char::from_u32(u as u32).ok_or(PrintError::NotAValidChar(u as i128))?;
                    printf_generic(p, format!("'{}'", escape_char(c)))
                }
                'x' => printf_x(p, u),
                'X' => printf_xx(p, u),
                'U' => printf_generic(p, format!("U+{:X}", u)),
                _ => return Err(PrintError::UnableToFormat(val.clone(), typ)),
            })
        }
        Value::Number(ref n) if n.as_i64().is_some() => {
            let i = n.as_i64().unwrap();
            Ok(match typ {
                'b' => printf_b(p, i),
                'd' => printf_generic(p, i),
                'o' => printf_o(p, i),
                'c' => {
                    let c = char::from_u32(i as u32).ok_or(PrintError::NotAValidChar(i as i128))?;
                    printf_generic(p, c)
                }
                'q' => {
                    let c = char::from_u32(i as u32).ok_or(PrintError::NotAValidChar(i as i128))?;
                    printf_generic(p, format!("'{}'", escape_char(c)))
                }
                'x' => printf_x(p, i),
                'X' => printf_xx(p, i),
                'U' => printf_generic(p, format!("U+{:X}", i)),
                _ => return Err(PrintError::UnableToFormat(val.clone(), typ)),
            })
        }
        Value::Number(ref n) if n.as_f64().is_some() => {
            let f = n.as_f64().unwrap();
            Ok(match typ {
                'e' => printf_e(p, f),
                'E' => printf_ee(p, f),
                'f' | 'F' => printf_generic(p, f),
                _ => return Err(PrintError::UnableToFormat(val.clone(), typ)),
            })
        }
        Value::Bool(ref b) => Ok(match typ {
            'v' | 't' => printf_generic(p, b),
            _ => return Err(PrintError::UnableToFormat(val.clone(), typ)),
        }),
        Value::String(ref s) => Ok(match typ {
            's' | 'v' => printf_generic(p, s),
            'x' => printf_x(p, Hexer::from(s.as_str())),
            'X' => printf_xx(p, Hexer::from(s.as_str())),
            'q' => {
                let s = s
                    .chars()
                    .map(|c| c.escape_default().to_string())
                    .collect::<String>();
                printf_generic(p, s)
            }
            _ => return Err(PrintError::UnableToFormat(val.clone(), typ)),
        }),
        Value::Array(ref a) => Ok(match typ {
            'v' => {
                let values: Vec<String> = a.iter().map(|v| printf_generic(p, v)).collect();
                let res = format!("[{}]", values.join(" "));
                printf_generic(p, res)
            }
            _ => return Err(PrintError::UnableToFormat(val.clone(), typ)),
        }),
        Value::Map(ref m) => Ok(match typ {
            'v' => {
                let values: Vec<String> = m
                    .iter()
                    .map(|(k, v)| {
                        let v_str = printf_generic(p, v);
                        format!("{}:{}", k, v_str)
                    })
                    .collect();
                let res = format!("map[{}]", values.join(" "));
                printf_generic(p, res)
            }
            _ => return Err(PrintError::UnableToFormat(val.clone(), typ)),
        }),
        _ => Err(PrintError::UnableToFormat(val.clone(), typ)),
    }
}

fn printf_b<B: fmt::Binary>(p: &FormatParams, u: B) -> String {
    match params_to_chars(p) {
        ('#', '_', '+', '_', _) => format!("{:+#width$b}", u, width = p.width),
        ('_', '_', '+', '_', _) => format!("{:+width$b}", u, width = p.width),
        ('#', '_', '_', '_', _) => format!("{:#width$b}", u, width = p.width),
        ('#', '0', '+', '_', _) => format!("{:+#0width$b}", u, width = p.width),
        ('_', '0', '+', '_', _) => format!("{:+0width$b}", u, width = p.width),
        ('#', '0', '_', '_', _) => format!("{:#0width$b}", u, width = p.width),
        ('#', '_', '+', '-', _) => format!("{:<+#width$b}", u, width = p.width),
        ('_', '_', '+', '-', _) => format!("{:<+width$b}", u, width = p.width),
        ('#', '_', '_', '-', _) => format!("{:<#width$b}", u, width = p.width),
        ('#', '0', '+', '-', _) => format!("{:<+#0width$b}", u, width = p.width),
        ('_', '0', '+', '-', _) => format!("{:<+0width$b}", u, width = p.width),
        ('#', '0', '_', '-', _) => format!("{:<#0width$b}", u, width = p.width),
        (_, _, _, _, _) => format!("{:width$b}", u, width = p.width),
    }
}

fn printf_o<B: fmt::Octal>(p: &FormatParams, u: B) -> String {
    match params_to_chars(p) {
        ('#', '_', '+', '_', _) => format!("{:+#width$o}", u, width = p.width),
        ('_', '_', '+', '_', _) => format!("{:+width$o}", u, width = p.width),
        ('#', '_', '_', '_', _) => format!("{:#width$o}", u, width = p.width),
        ('#', '0', '+', '_', _) => format!("{:+#0width$o}", u, width = p.width),
        ('_', '0', '+', '_', _) => format!("{:+0width$o}", u, width = p.width),
        ('#', '0', '_', '_', _) => format!("{:#0width$o}", u, width = p.width),
        ('#', '_', '+', '-', _) => format!("{:<+#width$o}", u, width = p.width),
        ('_', '_', '+', '-', _) => format!("{:<+width$o}", u, width = p.width),
        ('#', '_', '_', '-', _) => format!("{:<#width$o}", u, width = p.width),
        ('#', '0', '+', '-', _) => format!("{:<+#0width$o}", u, width = p.width),
        ('_', '0', '+', '-', _) => format!("{:<+0width$o}", u, width = p.width),
        ('#', '0', '_', '-', _) => format!("{:<#0width$o}", u, width = p.width),
        (_, _, _, _, _) => format!("{:width$o}", u, width = p.width),
    }
}

fn printf_x<B: fmt::LowerHex>(p: &FormatParams, u: B) -> String {
    match params_to_chars(p) {
        ('#', '_', '+', '_', _) => format!("{:+#width$x}", u, width = p.width),
        ('_', '_', '+', '_', _) => format!("{:+width$x}", u, width = p.width),
        ('#', '_', '_', '_', _) => format!("{:#width$x}", u, width = p.width),
        ('#', '0', '+', '_', _) => format!("{:+#0width$x}", u, width = p.width),
        ('_', '0', '+', '_', _) => format!("{:+0width$x}", u, width = p.width),
        ('#', '0', '_', '_', _) => format!("{:#0width$x}", u, width = p.width),
        ('#', '_', '+', '-', _) => format!("{:<+#width$x}", u, width = p.width),
        ('_', '_', '+', '-', _) => format!("{:<+width$x}", u, width = p.width),
        ('#', '_', '_', '-', _) => format!("{:<#width$x}", u, width = p.width),
        ('#', '0', '+', '-', _) => format!("{:<+#0width$x}", u, width = p.width),
        ('_', '0', '+', '-', _) => format!("{:<+0width$x}", u, width = p.width),
        ('#', '0', '_', '-', _) => format!("{:<#0width$x}", u, width = p.width),
        (_, _, _, _, _) => format!("{:width$x}", u, width = p.width),
    }
}

fn printf_xx<B: fmt::UpperHex>(p: &FormatParams, u: B) -> String {
    match params_to_chars(p) {
        ('#', '_', '+', '_', _) => format!("{:+#width$X}", u, width = p.width),
        ('_', '_', '+', '_', _) => format!("{:+width$X}", u, width = p.width),
        ('#', '_', '_', '_', _) => format!("{:#width$X}", u, width = p.width),
        ('#', '0', '+', '_', _) => format!("{:+#0width$X}", u, width = p.width),
        ('_', '0', '+', '_', _) => format!("{:+0width$X}", u, width = p.width),
        ('#', '0', '_', '_', _) => format!("{:#0width$X}", u, width = p.width),
        ('#', '_', '+', '-', _) => format!("{:<+#width$X}", u, width = p.width),
        ('_', '_', '+', '-', _) => format!("{:<+width$X}", u, width = p.width),
        ('#', '_', '_', '-', _) => format!("{:<#width$X}", u, width = p.width),
        ('#', '0', '+', '-', _) => format!("{:<+#0width$X}", u, width = p.width),
        ('_', '0', '+', '-', _) => format!("{:<+0width$X}", u, width = p.width),
        ('#', '0', '_', '-', _) => format!("{:<#0width$X}", u, width = p.width),
        (_, _, _, _, _) => format!("{:width$X}", u, width = p.width),
    }
}

fn printf_generic<D: fmt::Display>(p: &FormatParams, c: D) -> String {
    if let Some(pr) = p.precision {
        match params_to_chars(p) {
            ('#', '_', '+', '_', _) => format!("{:+#width$.pr$}", c, width = p.width, pr = pr),
            ('_', '_', '+', '_', _) => format!("{:+width$.pr$}", c, width = p.width, pr = pr),
            ('#', '_', '_', '_', _) => format!("{:#width$.pr$}", c, width = p.width, pr = pr),
            ('#', '0', '+', '_', _) => format!("{:+#0width$.pr$}", c, width = p.width, pr = pr),
            ('_', '0', '+', '_', _) => format!("{:+0width$.pr$}", c, width = p.width, pr = pr),
            ('#', '0', '_', '_', _) => format!("{:#0width$.pr$}", c, width = p.width, pr = pr),
            ('#', '_', '+', '-', _) => format!("{:<+#width$.pr$}", c, width = p.width, pr = pr),
            ('_', '_', '+', '-', _) => format!("{:<+width$.pr$}", c, width = p.width, pr = pr),
            ('#', '_', '_', '-', _) => format!("{:<#width$.pr$}", c, width = p.width, pr = pr),
            ('#', '0', '+', '-', _) => format!("{:<+#0width$.pr$}", c, width = p.width, pr = pr),
            ('_', '0', '+', '-', _) => format!("{:<+0width$.pr$}", c, width = p.width, pr = pr),
            ('#', '0', '_', '-', _) => format!("{:<#0width$.pr$}", c, width = p.width, pr = pr),
            (_, _, _, _, _) => format!("{:width$.pr$}", c, width = p.width, pr = pr),
        }
    } else {
        match params_to_chars(p) {
            ('#', '_', '+', '_', _) => format!("{:+#width$}", c, width = p.width),
            ('_', '_', '+', '_', _) => format!("{:+width$}", c, width = p.width),
            ('#', '_', '_', '_', _) => format!("{:#width$}", c, width = p.width),
            ('#', '0', '+', '_', _) => format!("{:+#0width$}", c, width = p.width),
            ('_', '0', '+', '_', _) => format!("{:+0width$}", c, width = p.width),
            ('#', '0', '_', '_', _) => format!("{:#0width$}", c, width = p.width),
            ('#', '_', '+', '-', _) => format!("{:<+#width$}", c, width = p.width),
            ('_', '_', '+', '-', _) => format!("{:<+width$}", c, width = p.width),
            ('#', '_', '_', '-', _) => format!("{:<#width$}", c, width = p.width),
            ('#', '0', '+', '-', _) => format!("{:<+#0width$}", c, width = p.width),
            ('_', '0', '+', '-', _) => format!("{:<+0width$}", c, width = p.width),
            ('#', '0', '_', '-', _) => format!("{:<#0width$}", c, width = p.width),
            (_, _, _, _, _) => format!("{:width$}", c, width = p.width),
        }
    }
}

fn printf_e<E: fmt::LowerExp>(p: &FormatParams, f: E) -> String {
    if let Some(pr) = p.precision {
        match params_to_chars(p) {
            ('#', '_', '+', '_', _) => format!("{:+#width$.pr$e}", f, width = p.width, pr = pr),
            ('_', '_', '+', '_', _) => format!("{:+width$.pr$e}", f, width = p.width, pr = pr),
            ('#', '_', '_', '_', _) => format!("{:#width$.pr$e}", f, width = p.width, pr = pr),
            ('#', '0', '+', '_', _) => format!("{:+#0width$.pr$e}", f, width = p.width, pr = pr),
            ('_', '0', '+', '_', _) => format!("{:+0width$.pr$e}", f, width = p.width, pr = pr),
            ('#', '0', '_', '_', _) => format!("{:#0width$.pr$e}", f, width = p.width, pr = pr),
            ('#', '_', '+', '-', _) => format!("{:<+#width$.pr$e}", f, width = p.width, pr = pr),
            ('_', '_', '+', '-', _) => format!("{:<+width$.pr$e}", f, width = p.width, pr = pr),
            ('#', '_', '_', '-', _) => format!("{:<#width$.pr$e}", f, width = p.width, pr = pr),
            ('#', '0', '+', '-', _) => format!("{:<+#0width$.pr$e}", f, width = p.width, pr = pr),
            ('_', '0', '+', '-', _) => format!("{:<+0width$.pr$e}", f, width = p.width, pr = pr),
            ('#', '0', '_', '-', _) => format!("{:<#0width$.pr$e}", f, width = p.width, pr = pr),
            (_, _, _, _, _) => format!("{:width$.pr$e}", f, width = p.width, pr = pr),
        }
    } else {
        match params_to_chars(p) {
            ('#', '_', '+', '_', _) => format!("{:+#width$e}", f, width = p.width),
            ('_', '_', '+', '_', _) => format!("{:+width$e}", f, width = p.width),
            ('#', '_', '_', '_', _) => format!("{:#width$e}", f, width = p.width),
            ('#', '0', '+', '_', _) => format!("{:+#0width$e}", f, width = p.width),
            ('_', '0', '+', '_', _) => format!("{:+0width$e}", f, width = p.width),
            ('#', '0', '_', '_', _) => format!("{:#0width$e}", f, width = p.width),
            ('#', '_', '+', '-', _) => format!("{:<+#width$e}", f, width = p.width),
            ('_', '_', '+', '-', _) => format!("{:<+width$e}", f, width = p.width),
            ('#', '_', '_', '-', _) => format!("{:<#width$e}", f, width = p.width),
            ('#', '0', '+', '-', _) => format!("{:<+#0width$e}", f, width = p.width),
            ('_', '0', '+', '-', _) => format!("{:<+0width$e}", f, width = p.width),
            ('#', '0', '_', '-', _) => format!("{:<#0width$e}", f, width = p.width),
            (_, _, _, _, _) => format!("{:width$e}", f, width = p.width),
        }
    }
}

fn printf_ee<E: fmt::UpperExp>(p: &FormatParams, f: E) -> String {
    if let Some(pr) = p.precision {
        match params_to_chars(p) {
            ('#', '_', '+', '_', _) => format!("{:+#width$.pr$E}", f, width = p.width, pr = pr),
            ('_', '_', '+', '_', _) => format!("{:+width$.pr$E}", f, width = p.width, pr = pr),
            ('#', '_', '_', '_', _) => format!("{:#width$.pr$E}", f, width = p.width, pr = pr),
            ('#', '0', '+', '_', _) => format!("{:+#0width$.pr$E}", f, width = p.width, pr = pr),
            ('_', '0', '+', '_', _) => format!("{:+0width$.pr$E}", f, width = p.width, pr = pr),
            ('#', '0', '_', '_', _) => format!("{:#0width$.pr$E}", f, width = p.width, pr = pr),
            ('#', '_', '+', '-', _) => format!("{:<+#width$.pr$E}", f, width = p.width, pr = pr),
            ('_', '_', '+', '-', _) => format!("{:<+width$.pr$E}", f, width = p.width, pr = pr),
            ('#', '_', '_', '-', _) => format!("{:<#width$.pr$E}", f, width = p.width, pr = pr),
            ('#', '0', '+', '-', _) => format!("{:<+#0width$.pr$E}", f, width = p.width, pr = pr),
            ('_', '0', '+', '-', _) => format!("{:<+0width$.pr$E}", f, width = p.width, pr = pr),
            ('#', '0', '_', '-', _) => format!("{:<#0width$.pr$E}", f, width = p.width, pr = pr),
            (_, _, _, _, _) => format!("{:width$.pr$E}", f, width = p.width, pr = pr),
        }
    } else {
        match params_to_chars(p) {
            ('#', '_', '+', '_', _) => format!("{:+#width$E}", f, width = p.width),
            ('_', '_', '+', '_', _) => format!("{:+width$E}", f, width = p.width),
            ('#', '_', '_', '_', _) => format!("{:#width$E}", f, width = p.width),
            ('#', '0', '+', '_', _) => format!("{:+#0width$E}", f, width = p.width),
            ('_', '0', '+', '_', _) => format!("{:+0width$E}", f, width = p.width),
            ('#', '0', '_', '_', _) => format!("{:#0width$E}", f, width = p.width),
            ('#', '_', '+', '-', _) => format!("{:<+#width$E}", f, width = p.width),
            ('_', '_', '+', '-', _) => format!("{:<+width$E}", f, width = p.width),
            ('#', '_', '_', '-', _) => format!("{:<#width$E}", f, width = p.width),
            ('#', '0', '+', '-', _) => format!("{:<+#0width$E}", f, width = p.width),
            ('_', '0', '+', '-', _) => format!("{:<+0width$E}", f, width = p.width),
            ('#', '0', '_', '-', _) => format!("{:<#0width$E}", f, width = p.width),
            (_, _, _, _, _) => format!("{:width$E}", f, width = p.width),
        }
    }
}

fn escape_char(c: char) -> String {
    let mut s = c.escape_default().to_string();
    if s.starts_with(r"\u") {
        s = s.replace("{", "").replace("}", "");
    }
    s
}

struct Hexer<'a> {
    s: &'a str,
}

impl<'a> From<&'a str> for Hexer<'a> {
    fn from(s: &'a str) -> Self {
        Hexer { s }
    }
}

impl<'a> fmt::UpperHex for Hexer<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for u in self.s.as_bytes() {
            write!(f, "{:X}", u)?
        }
        Ok(())
    }
}

impl<'a> fmt::LowerHex for Hexer<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for u in self.s.as_bytes() {
            write!(f, "{:x}", u)?
        }
        Ok(())
    }
}
