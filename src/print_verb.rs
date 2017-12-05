use std::char;

use printf::FormatParams;

use gtmpl_value::Value;

pub fn print(p: &FormatParams, typ: char, v: &Value) -> Result<String, String> {
    match *v {
        Value::Number(ref n) if n.as_u64().is_some() => {
            let u = n.as_u64().unwrap();
            return Ok(match typ {
                'b' => {
                    if p.sharp && p.plus {
                        format!("{:+#b}", u)
                    } else if p.sharp {
                        format!("{:#b}", u)
                    } else if p.plus {
                        format!("{:+b}", u)
                    } else {
                        format!("{:b}", u)
                    }
                }
                'c' => {
                    format!(
                        "{}",
                        char::from_u32(u as u32).ok_or_else(|| {
                            format!("{:X} is not a valid char", u)
                        })?
                    )
                }
                'd' => {
                    if p.plus {
                        format!("{:+}", u)
                    } else {
                        format!("{}", u)
                    }
                }
                'o' => {
                    if p.sharp && p.plus {
                        format!("{:+#o}", u)
                    } else if p.sharp {
                        format!("{:#o}", u)
                    } else if p.plus {
                        format!("{:+o}", u)
                    } else {
                        format!("{:o}", u)
                    }
                }
                'q' => {
                    format!(
                        "'{}'",
                        char::from_u32(u as u32).ok_or_else(|| {
                            format!("{:X} is not a valid char", u)
                        })?
                    )
                }
                'x' => {
                    if p.sharp && p.plus {
                        format!("{:+#x}", u)
                    } else if p.sharp {
                        format!("{:#x}", u)
                    } else if p.plus {
                        format!("{:+x}", u)
                    } else {
                        format!("{:x}", u)
                    }
                }
                'X' => {
                    if p.sharp && p.plus {
                        format!("{:+#X}", u)
                    } else if p.sharp {
                        format!("{:#X}", u)
                    } else if p.plus {
                        format!("{:+X}", u)
                    } else {
                        format!("{:X}", u)
                    }
                }
                'U' => format!("U+{:X}", u),
                _ => return Err(format!("unable to format {} as %{}", v, typ)),
            });
        }
        Value::Number(ref n) if n.as_i64().is_some() => {
            let i = n.as_i64().unwrap();
            return Ok(match typ {
                'b' => {
                    if p.sharp && p.plus {
                        format!("{:+#b}", i)
                    } else if p.sharp {
                        format!("{:#b}", i)
                    } else if p.plus {
                        format!("{:+b}", i)
                    } else {
                        format!("{:b}", i)
                    }
                }
                'c' => {
                    format!(
                        "{}",
                        char::from_u32(i as u32).ok_or_else(|| {
                            format!("{:X} is not a valid char", i)
                        })?
                    )
                }
                'd' => {
                    if p.plus {
                        format!("{:+}", i)
                    } else {
                        format!("{}", i)
                    }
                }
                'o' => {
                    if p.sharp && p.plus {
                        format!("{:+#o}", i)
                    } else if p.sharp {
                        format!("{:#o}", i)
                    } else if p.plus {
                        format!("{:+o}", i)
                    } else {
                        format!("{:o}", i)
                    }
                }
                'q' => {
                    format!(
                        "'{}'",
                        char::from_u32(i as u32).ok_or_else(|| {
                            format!("{:X} is not a valid char", i)
                        })?
                    )
                }
                'x' => {
                    if p.sharp && p.plus {
                        format!("{:+#x}", i)
                    } else if p.sharp {
                        format!("{:#x}", i)
                    } else if p.plus {
                        format!("{:+x}", i)
                    } else {
                        format!("{:x}", i)
                    }
                }
                'X' => {
                    if p.sharp && p.plus {
                        format!("{:+#X}", i)
                    } else if p.sharp {
                        format!("{:#X}", i)
                    } else if p.plus {
                        format!("{:+X}", i)
                    } else {
                        format!("{:X}", i)
                    }
                }
                'U' => format!("U+{:X}", i),
                _ => return Err(format!("unable to format {} as %{}", v, typ)),
            });
        }
        Value::Number(ref n) if n.as_f64().is_some() => {
            let f = n.as_f64().unwrap();
            return Ok(match typ {
                'b' => format!("{}", f),
                'e' => format!("{}", f),
                'E' => format!("{}", f),
                'f' => format!("{}", f),
                'F' => format!("{}", f),
                'g' => format!("{}", f),
                'G' => format!("{}", f),
                _ => return Err(format!("unable to format {} as %{}", v, typ)),
            });
        }
        _ => return Err(format!("unable to format {} as %{}", v, typ)),
    }
}
