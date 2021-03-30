//! Builtin functions.
use std::cmp::Ordering;
use std::fmt::Write;

use gtmpl_value::{Func, FuncError, Value};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

use crate::printf::sprintf;
use crate::utils::is_true;

const QUERY_ENCODE: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'#')
    .add(b'`')
    .add(b'`')
    .add(b'?')
    .add(b'{')
    .add(b'}');

pub static BUILTINS: &[(&str, Func)] = &[
    ("eq", eq as Func),
    ("ne", ne as Func),
    ("lt", lt as Func),
    ("le", le as Func),
    ("gt", gt as Func),
    ("ge", ge as Func),
    ("len", len as Func),
    ("and", and as Func),
    ("or", or as Func),
    ("not", not as Func),
    ("urlquery", urlquery as Func),
    ("print", print as Func),
    ("println", println as Func),
    ("printf", printf as Func),
    ("index", index as Func),
    ("call", call as Func),
];

macro_rules! val {
    ($x:expr) => {
        Value::from($x)
    };
}

/// Help to write new functions for gtmpl.
#[macro_export]
macro_rules! gtmpl_fn {
 (
  $(#[$outer:meta])*
  fn $name:ident() -> Result<$otyp:ty, FuncError>
  { $($body:tt)* }
 ) => {
  $(#[$outer])*
  pub fn $name(args: &[$crate::Value]) -> Result<$crate::Value, FuncError> {
   fn inner() -> Result<$otyp, FuncError> {
    $($body)*
   }
   Ok($crate::Value::from(inner()?))
  }
 };
 (
  $(#[$outer:meta])*
  fn $name:ident($arg0:ident : $typ0:ty) -> Result<$otyp:ty, FuncError>
  { $($body:tt)* }
 ) => {
  $(#[$outer])*
  pub fn $name(
   args: &[$crate::Value]
  ) -> Result<$crate::Value, FuncError> {
   if args.is_empty() {
    return Err(FuncError::AtLeastXArgs(stringify!($name).into(), 1));
   }
   let x = &args[0];
   let $arg0: $typ0 = $crate::from_value(x)
    .ok_or(FuncError::UnableToConvertFromValue)?;
   fn inner($arg0 : $typ0) -> Result<$otyp, FuncError> {
    $($body)*
   }
   let ret: $crate::Value = inner($arg0)?.into();
   Ok(ret)
  }
 };
 (
  $(#[$outer:meta])*
  fn $name:ident($arg0:ident : $typ0:ty$(, $arg:ident : $typ:ty)*) -> Result<$otyp:ty, FuncError>
  { $($body:tt)* }
 ) => {
  $(#[$outer])*
  pub fn $name(
   args: &[$crate::Value]
  ) -> Result<$crate::Value, FuncError> {
   #[allow(unused_mut)]
   let mut args = args;
   if args.is_empty() {
    return Err(FuncError::AtLeastXArgs(stringify!($name).into(), 1));
   }
   let x = &args[0];
   let $arg0: $typ0 = $crate::from_value(x)
    .ok_or(FuncError::UnableToConvertFromValue)?;
   $(args = &args[1..];
     let x = &args[0];
     let $arg: $typ = $crate::from_value(x)
    .ok_or(FuncError::UnableToConvertFromValue)?;)*
   fn inner($arg0 : $typ0, $($arg : $typ,)*) -> Result<$otyp, FuncError> {
    $($body)*
   }
   let ret: $crate::Value = inner($arg0, $($arg),*)?.into();
   Ok(ret)
  }
 }
}

macro_rules! gn {
 (
  $(#[$outer:meta])*
  $name:ident($arg1:ident : ref Value, $arg2:ident : ref Value) ->
   Result<Value, FuncError>
  { $($body:tt)* }
 ) => {
  $(#[$outer])*
  pub fn $name(args: &[Value]) -> Result<Value, FuncError> {
   if args.len() != 2 {
    return Err(FuncError::AtLeastXArgs(stringify!($name).into(), 2));
   }
   let $arg1 = &args[0];
   let $arg2 = &args[1];
   fn inner($arg1: &Value, $arg2: &Value) -> Result<Value, FuncError> {
    $($body)*
   }
   inner($arg1, $arg2)
  }
 }
}

/// Returns the boolean OR of its arguments by returning the
/// first non-empty argument or the last argument, that is,
/// "or x y" behaves as "if x then x else y". All the
/// arguments are evaluated.
///
/// # Example
/// ```
/// use gtmpl::template;
/// let equal = template("{{ or 1 2.0 false . }}", "foo");
/// assert_eq!(&equal.unwrap(), "1");
/// ```
pub fn or(args: &[Value]) -> Result<Value, FuncError> {
    for arg in args {
        if is_true(arg) {
            return Ok(arg.clone());
        }
    }
    args.iter()
        .cloned()
        .last()
        .ok_or_else(|| FuncError::AtLeastXArgs("or".into(), 1))
}

/// Returns the boolean AND of its arguments by returning the
/// first empty argument or the last argument, that is,
/// "and x y" behaves as "if x then y else x". All the
/// arguments are evaluated.
///
/// # Example
/// ```
/// use gtmpl::template;
/// let equal = template("{{ and 1 2.0 true . }}", "foo");
/// assert_eq!(&equal.unwrap(), "foo");
/// ```
pub fn and(args: &[Value]) -> Result<Value, FuncError> {
    for arg in args {
        if !is_true(arg) {
            return Ok(arg.clone());
        }
    }
    args.iter()
        .cloned()
        .last()
        .ok_or_else(|| FuncError::AtLeastXArgs("and".into(), 1))
}

/// Returns the boolean negation of its single argument.
///
/// # Example
/// ```
/// use gtmpl::template;
/// let equal = template("{{ not 0 }}", "");
/// assert_eq!(&equal.unwrap(), "true");
/// ```
pub fn not(args: &[Value]) -> Result<Value, FuncError> {
    if args.len() != 1 {
        Err(FuncError::ExactlyXArgs("not".into(), 1))
    } else {
        Ok(val!(!is_true(&args[0])))
    }
}

/// Returns the integer length of its argument.
///
/// # Example
/// ```
/// use gtmpl::template;
/// let equal = template("{{ len . }}", "foo");
/// assert_eq!(&equal.unwrap(), "3");
/// ```
pub fn len(args: &[Value]) -> Result<Value, FuncError> {
    if args.len() != 1 {
        return Err(FuncError::ExactlyXArgs("len".into(), 1));
    }
    let arg = &args[0];
    let len = match *arg {
        Value::String(ref s) => s.len(),
        Value::Array(ref a) => a.len(),
        Value::Object(ref o) => o.len(),
        _ => {
            return Err(FuncError::Generic(format!("unable to call len on {}", arg)));
        }
    };

    Ok(val!(len))
}

/// Returns the result of calling the first argument, which
/// must be a function, with the remaining arguments as parameters.
///
/// # Example
/// ```
/// use gtmpl::{gtmpl_fn, template, Value};
/// use gtmpl_value::{FuncError, Function};
///
/// gtmpl_fn!(
/// fn add(a: u64, b: u64) -> Result<u64, FuncError> {
///     Ok(a + b)
/// });
/// let equal = template(r#"{{ call . 1 2 }}"#, Value::Function(Function { f: add }));
/// assert_eq!(&equal.unwrap(), "3");
/// ```
pub fn call(args: &[Value]) -> Result<Value, FuncError> {
    if args.is_empty() {
        Err(FuncError::AtLeastXArgs("call".into(), 1))
    } else if let Value::Function(ref f) = args[0] {
        (f.f)(&args[1..])
    } else {
        Err(FuncError::Generic(
            "call requires the first argument to be a function".into(),
        ))
    }
}

/// An implementation of golang's fmt.Sprint
///
/// Golang's Sprint formats using the default formats for its operands and returns the
/// resulting string. Spaces are added between operands when neither is a string.
///
/// # Example
/// ```
/// use gtmpl::template;
/// let equal = template(r#"{{ print "Hello " . "!" }}"#, "world");
/// assert_eq!(&equal.unwrap(), "Hello world!");
/// ```
pub fn print(args: &[Value]) -> Result<Value, FuncError> {
    let mut no_space = true;
    let mut s = String::new();
    for val in args {
        if let Value::String(ref v) = *val {
            no_space = true;
            s.push_str(v);
        } else {
            if no_space {
                s += &val.to_string();
            } else {
                s += &format!(" {}", val.to_string())
            }
            no_space = false;
        }
    }
    Ok(val!(s))
}

/// An implementation of golang's fmt.Sprintln
///
/// Sprintln formats using the default formats for its operands and returns the
/// resulting string. Spaces are always added between operands and a newline is appended.
///
/// # Example
/// ```
/// use gtmpl::template;
/// let equal = template(r#"{{ println "Hello" . "!" }}"#, "world");
/// assert_eq!(&equal.unwrap(), "Hello world !\n");
/// ```
pub fn println(args: &[Value]) -> Result<Value, FuncError> {
    let mut iter = args.iter();
    let s = match iter.next() {
        None => String::from("\n"),
        Some(first_elt) => {
            let (lower, _) = iter.size_hint();
            let mut result = String::with_capacity(lower + 1);
            if let Value::String(ref v) = *first_elt {
                result.push_str(v);
            } else {
                write!(&mut result, "{}", first_elt).unwrap();
            }
            for elt in iter {
                result.push(' ');
                if let Value::String(ref v) = *elt {
                    result.push_str(v);
                } else {
                    write!(&mut result, "{}", elt).unwrap();
                }
            }
            result.push('\n');
            result
        }
    };
    Ok(val!(s))
}

/// An implementation of golang's fmt.Sprintf
/// Limitations:
/// - float:
///   * `g`, `G`, and `b` are weired and not implement yet
/// - pretty sure there are more
///
/// # Example
/// ```
/// use gtmpl::template;
/// let equal = template(r#"{{ printf "%v %s %v" "Hello" . "!" }}"#, "world");
/// assert_eq!(&equal.unwrap(), "Hello world !");
/// ```
pub fn printf(args: &[Value]) -> Result<Value, FuncError> {
    if args.is_empty() {
        return Err(FuncError::AtLeastXArgs("printf".into(), 1));
    }
    if let Value::String(ref s) = args[0] {
        let s = sprintf(s, &args[1..]).map_err(|e| FuncError::Other(e.into()))?;
        Ok(val!(s))
    } else {
        Err(FuncError::Generic("printf requires a format string".into()))
    }
}

/// Returns the result of indexing its first argument by the
/// following arguments. Thus "index x 1 2 3" is, in Go syntax,
/// x[1][2][3]. Each indexed item must be a map, slice or array.
///
/// # Example
/// ```
/// use gtmpl::template;
/// let ctx = vec![23, 42, 7];
/// let index = template("{{ index . 1 }}", ctx);
/// assert_eq!(&index.unwrap(), "42");
/// ```
pub fn index(args: &[Value]) -> Result<Value, FuncError> {
    if args.len() < 2 {
        return Err(FuncError::AtLeastXArgs("index".into(), 2));
    }
    let mut col = &args[0];
    for val in &args[1..] {
        col = get_item(col, val)?;
    }

    Ok(col.clone())
}

fn get_item<'a>(col: &'a Value, key: &Value) -> Result<&'a Value, FuncError> {
    let ret = match (col, key) {
        (&Value::Array(ref a), &Value::Number(ref n)) => {
            if let Some(i) = n.as_u64() {
                a.get(i as usize)
            } else {
                None
            }
        }
        (&Value::Object(ref o), &Value::Number(ref n))
        | (&Value::Map(ref o), &Value::Number(ref n)) => o.get(&n.to_string()),
        (&Value::Object(ref o), &Value::String(ref s))
        | (&Value::Map(ref o), &Value::String(ref s)) => o.get(s),
        _ => None,
    };
    match *col {
        Value::Map(_) => Ok(ret.unwrap_or(&Value::NoValue)),
        _ => ret.ok_or_else(|| FuncError::Generic(format!("unable to get {} in {}", key, col))),
    }
}

/// Returns the escaped value of the textual representation of
/// its arguments in a form suitable for embedding in a URL query.
///
/// # Example
/// ```
/// use gtmpl::template;
/// let url = template(r#"{{ urlquery "foo bar?" }}"#, 0);
/// assert_eq!(&url.unwrap(), "foo%20bar%3F");
/// ```
pub fn urlquery(args: &[Value]) -> Result<Value, FuncError> {
    if args.len() != 1 {
        return Err(FuncError::ExactlyXArgs("urlquery".into(), 1));
    }
    let val = &args[0];
    match *val {
        Value::String(ref s) => Ok(val!(utf8_percent_encode(s, QUERY_ENCODE).to_string())),
        _ => Err(FuncError::Generic(
            "Arguments need to be of type String".into(),
        )),
    }
}

/// Returns the boolean truth of arg1 == arg2 [== arg3 ...]
///
/// # Example
/// ```
/// use gtmpl::template;
/// let equal = template("{{ eq 1 1 . }}", 1);
/// assert_eq!(&equal.unwrap(), "true");
/// ```
pub fn eq(args: &[Value]) -> Result<Value, FuncError> {
    if args.len() < 2 {
        return Err(FuncError::AtLeastXArgs("eq".into(), 2));
    }
    let first = &args[0];
    Ok(Value::from(args.iter().skip(1).all(|x| *x == *first)))
}

gn!(
#[doc="
Returns the boolean truth of arg1 != arg2

# Example
```
use gtmpl::template;
let not_equal = template(\"{{ ne 2 . }}\", 1);
assert_eq!(&not_equal.unwrap(), \"true\");
```
"]
ne(a: ref Value, b: ref Value) -> Result<Value, FuncError> {
 Ok(Value::from(a != b))
});

gn!(
#[doc="
Returns the boolean truth of arg1 < arg2

# Example
```
use gtmpl::template;
let less_than = template(\"{{ lt 0 . }}\", 1);
assert_eq!(&less_than.unwrap(), \"true\");
```
"]
lt(a: ref Value, b: ref Value) -> Result<Value, FuncError> {
 let ret = match cmp(a, b) {
  None => return Err(FuncError::Generic(format!("unable to compare {} and {}", a, b))),
  Some(Ordering::Less) => true,
  _ => false,
 };
 Ok(Value::from(ret))
});

gn!(
#[doc="
Returns the boolean truth of arg1 <= arg2

# Example
```
use gtmpl::template;
let less_or_equal = template(\"{{ le 1.4 . }}\", 1.4);
assert_eq!(less_or_equal.unwrap(), \"true\");

let less_or_equal = template(\"{{ le 0.2 . }}\", 1.4);
assert_eq!(&less_or_equal.unwrap(), \"true\");
```
"]
le(a: ref Value, b: ref Value) -> Result<Value, FuncError> {
 let ret = match cmp(a, b) {
  None => return Err(FuncError::Generic(format!("unable to compare {} and {}", a, b))),
  Some(Ordering::Less) | Some(Ordering::Equal) => true,
  _ => false,
 };
 Ok(Value::from(ret))
});

gn!(
#[doc="
Returns the boolean truth of arg1 > arg2

# Example
```
use gtmpl::template;
let greater_than = template(\"{{ gt 1.4 . }}\", 1.2);
assert_eq!(&greater_than.unwrap(), \"true\");
```
"]
gt(a: ref Value, b: ref Value) -> Result<Value, FuncError> {
 let ret = match cmp(a, b) {
  None => return Err(FuncError::Generic(format!("unable to compare {} and {}", a, b))),
  Some(Ordering::Greater) => true,
  _ => false,
 };
 Ok(Value::from(ret))
});

gn!(
#[doc="
Returns the boolean truth of arg1 >= arg2

# Example
```
use gtmpl::template;
let greater_or_equal = template(\"{{ ge 1.4 1.3 }}\", 1.2);
assert_eq!(greater_or_equal.unwrap(), \"true\");

let greater_or_equal = template(\"{{ ge 1.4 . }}\", 0.2);
assert_eq!(&greater_or_equal.unwrap(), \"true\");
```
"]
ge(a: ref Value, b: ref Value) -> Result<Value, FuncError> {
 let ret = match cmp(a, b) {
  None => return Err(FuncError::Generic(format!("unable to compare {} and {}", a, b))),
  Some(Ordering::Greater) | Some(Ordering::Equal) => true,
  _ => false,
 };
 Ok(Value::from(ret))
});

fn cmp(left: &Value, right: &Value) -> Option<Ordering> {
    match (left, right) {
        (&Value::Number(ref l), &Value::Number(ref r)) => {
            if let (Some(lf), Some(rf)) = (l.as_f64(), r.as_f64()) {
                return lf.partial_cmp(&rf);
            }
            if let (Some(li), Some(ri)) = (l.as_i64(), r.as_i64()) {
                return li.partial_cmp(&ri);
            }
            if let (Some(lu), Some(ru)) = (l.as_u64(), r.as_u64()) {
                return lu.partial_cmp(&ru);
            }
            None
        }
        (&Value::Bool(ref l), &Value::Bool(ref r)) => l.partial_cmp(r),
        (&Value::String(ref l), &Value::String(ref r)) => l.partial_cmp(r),
        (&Value::Array(ref l), &Value::Array(ref r)) => l.len().partial_cmp(&r.len()),
        _ => None,
    }
}

#[cfg(test)]
mod tests_mocked {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_macro() {
        gtmpl_fn!(
            fn f1(i: i64) -> Result<i64, FuncError> {
                Ok(i + 1)
            }
        );
        let vals: Vec<Value> = vec![val!(1i64)];
        let ret = f1(&vals);
        assert_eq!(ret.unwrap(), Value::from(2i64));

        gtmpl_fn!(
            fn f3(i: i64, j: i64, k: i64) -> Result<i64, FuncError> {
                Ok(i + j + k)
            }
        );
        let vals: Vec<Value> = vec![val!(1i64), val!(2i64), val!(3i64)];
        let ret = f3(&vals);
        assert_eq!(ret.unwrap(), Value::from(6i64));
    }

    #[test]
    fn test_eq() {
        let vals: Vec<Value> = vec![val!("foo".to_owned()), val!("foo".to_owned())];
        let ret = eq(&vals);
        assert_eq!(ret.unwrap(), Value::Bool(true));
        let vals: Vec<Value> = vec![val!(1u32), val!(1u32), val!(1i8)];
        let ret = eq(&vals);
        assert_eq!(ret.unwrap(), Value::Bool(true));
        let vals: Vec<Value> = vec![val!(false), val!(false), val!(false)];
        let ret = eq(&vals);
        assert_eq!(ret.unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_and() {
        let vals: Vec<Value> = vec![val!(0i32), val!(1u8)];
        let ret = and(&vals);
        assert_eq!(ret.unwrap(), Value::from(0i32));

        let vals: Vec<Value> = vec![val!(1i32), val!(2u8)];
        let ret = and(&vals);
        assert_eq!(ret.unwrap(), Value::from(2u8));
    }

    #[test]
    fn test_or() {
        let vals: Vec<Value> = vec![val!(0i32), val!(1u8)];
        let ret = or(&vals);
        assert_eq!(ret.unwrap(), Value::from(1u8));

        let vals: Vec<Value> = vec![val!(0i32), val!(0u8)];
        let ret = or(&vals);
        assert_eq!(ret.unwrap(), Value::from(0u8));
    }

    #[test]
    fn test_ne() {
        let vals: Vec<Value> = vec![val!(0i32), val!(1u8)];
        let ret = ne(&vals);
        assert_eq!(ret.unwrap(), Value::from(true));

        let vals: Vec<Value> = vec![val!(0i32), val!(0u8)];
        let ret = ne(&vals);
        assert_eq!(ret.unwrap(), Value::from(false));

        let vals: Vec<Value> = vec![val!("foo"), val!("bar")];
        let ret = ne(&vals);
        assert_eq!(ret.unwrap(), Value::from(true));

        let vals: Vec<Value> = vec![val!("foo"), val!("foo")];
        let ret = ne(&vals);
        assert_eq!(ret.unwrap(), Value::from(false));
    }

    #[test]
    fn test_lt() {
        let vals: Vec<Value> = vec![val!(-1i32), val!(1u8)];
        let ret = lt(&vals);
        assert_eq!(ret.unwrap(), Value::from(true));

        let vals: Vec<Value> = vec![val!(0i32), val!(0u8)];
        let ret = lt(&vals);
        assert_eq!(ret.unwrap(), Value::from(false));

        let vals: Vec<Value> = vec![val!(1i32), val!(0u8)];
        let ret = lt(&vals);
        assert_eq!(ret.unwrap(), Value::from(false));
    }

    #[test]
    fn test_le() {
        let vals: Vec<Value> = vec![val!(-1i32), val!(1u8)];
        let ret = le(&vals);
        assert_eq!(ret.unwrap(), Value::from(true));

        let vals: Vec<Value> = vec![val!(0i32), val!(0u8)];
        let ret = le(&vals);
        assert_eq!(ret.unwrap(), Value::from(true));

        let vals: Vec<Value> = vec![val!(1i32), val!(0u8)];
        let ret = le(&vals);
        assert_eq!(ret.unwrap(), Value::from(false));
    }

    #[test]
    fn test_gt() {
        let vals: Vec<Value> = vec![val!(-1i32), val!(1u8)];
        let ret = gt(&vals);
        assert_eq!(ret.unwrap(), Value::from(false));

        let vals: Vec<Value> = vec![val!(0i32), val!(0u8)];
        let ret = gt(&vals);
        assert_eq!(ret.unwrap(), Value::from(false));

        let vals: Vec<Value> = vec![val!(1i32), val!(0u8)];
        let ret = gt(&vals);
        assert_eq!(ret.unwrap(), Value::from(true));
    }

    #[test]
    fn test_ge() {
        let vals: Vec<Value> = vec![val!(-1i32), val!(1u8)];
        let ret = ge(&vals);
        assert_eq!(ret.unwrap(), Value::from(false));

        let vals: Vec<Value> = vec![val!(0i32), val!(0u8)];
        let ret = ge(&vals);
        assert_eq!(ret.unwrap(), Value::from(true));

        let vals: Vec<Value> = vec![val!(1i32), val!(0u8)];
        let ret = ge(&vals);
        assert_eq!(ret.unwrap(), Value::from(true));
    }

    #[test]
    fn test_print() {
        let vals: Vec<Value> = vec![val!("foo"), val!(1u8)];
        let ret = print(&vals);
        assert_eq!(ret.unwrap(), Value::from("foo1"));

        let vals: Vec<Value> = vec![val!("foo"), val!(1u8), val!(2)];
        let ret = print(&vals);
        assert_eq!(ret.unwrap(), Value::from("foo1 2"));

        let vals: Vec<Value> = vec![val!(true), val!(1), val!("foo"), val!(2)];
        let ret = print(&vals);
        assert_eq!(ret.unwrap(), Value::from("true 1foo2"));
    }

    #[test]
    fn test_println() {
        let vals: Vec<Value> = vec![val!("foo"), val!(1u8)];
        let ret = println(&vals);
        assert_eq!(ret.unwrap(), Value::from("foo 1\n"));

        let vals: Vec<Value> = vec![];
        let ret = println(&vals);
        assert_eq!(ret.unwrap(), Value::from("\n"));
    }

    #[test]
    fn test_index() {
        let vals: Vec<Value> = vec![val!(vec![vec![1, 2], vec![3, 4]]), val!(1), val!(0)];
        let ret = index(&vals);
        assert_eq!(ret.unwrap(), Value::from(3));

        let mut o = HashMap::new();
        o.insert(String::from("foo"), vec![String::from("bar")]);
        let col = Value::from(o);
        let vals: Vec<Value> = vec![col, val!("foo"), val!(0)];
        let ret = index(&vals);
        assert_eq!(ret.unwrap(), Value::from("bar"));

        let mut o = HashMap::new();
        o.insert(String::from("foo"), String::from("bar"));
        let col = Value::from(o);
        let vals: Vec<Value> = vec![col, val!("foo2")];
        let ret = index(&vals);
        assert_eq!(ret.unwrap(), Value::NoValue);
    }

    #[test]
    fn test_builtins() {
        let vals: Vec<Value> = vec![val!("foo".to_owned()), val!("foo".to_owned())];
        let builtin_eq = BUILTINS
            .iter()
            .find(|&&(n, _)| n == "eq")
            .map(|&(_, f)| f)
            .unwrap();
        let ret = builtin_eq(&vals);
        assert_eq!(ret.unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_gtmpl_fn() {
        gtmpl_fn!(
            fn add(a: u64, b: u64) -> Result<u64, FuncError> {
                Ok(a + b)
            }
        );
        let vals: Vec<Value> = vec![val!(1u32), val!(2u32)];
        let ret = add(&vals);
        assert_eq!(ret.unwrap(), Value::from(3u32));

        gtmpl_fn!(
            fn has_prefix(s: String, prefix: String) -> Result<bool, FuncError> {
                Ok(s.starts_with(&prefix))
            }
        );
        let vals: Vec<Value> = vec![val!("foobar"), val!("foo")];
        let ret = has_prefix(&vals);
        assert_eq!(ret.unwrap(), Value::from(true));
    }
}
