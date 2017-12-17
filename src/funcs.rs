//! Builtin functions.
use std::any::Any;
use std::cmp::Ordering;
use std::fmt::Write;
use std::sync::Arc;

use gtmpl_value::{Func, Value};

extern crate percent_encoding;
use self::percent_encoding::{utf8_percent_encode, DEFAULT_ENCODE_SET};

use utils::is_true;
use printf::sprintf;

pub static BUILTINS: &[(&'static str, Func)] = &[
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

macro_rules! varc(
    ($x:expr) => { Arc::new(Value::from($x)) }
);

/// Help to write new functions for gtmpl.
#[macro_export]
macro_rules! gtmpl_fn {
    (
        $(#[$outer:meta])*
        fn $name:ident() -> Result<$otyp:ty, String>
        { $($body:tt)* }
    ) => {
        $(#[$outer])*
        pub fn $name(args: &[Arc<Any>]) -> Result<Arc<Any>, String> {
            fn inner() -> Result<$otyp, String> {
                $($body)*
            }
            Ok(Arc::new($crate::Value::from(inner()?)))
        }
    };
    (
        $(#[$outer:meta])*
        fn $name:ident($arg0:ident : $typ0:ty) -> Result<$otyp:ty, String>
        { $($body:tt)* }
    ) => {
        $(#[$outer])*
        pub fn $name(
            args: &[::std::sync::Arc<::std::any::Any>]
        ) -> Result<::std::sync::Arc<::std::any::Any>, String> {
            if args.is_empty() {
                return Err(String::from("at least one argument required"));
            }
            let x = &args[0];
            let $arg0 = x.downcast_ref::<$crate::Value>()
                .ok_or_else(|| "unable to downcast".to_owned())?;
            let $arg0: $typ0 = $crate::from_value($arg0)
                .ok_or_else(|| "unable to convert from Value".to_owned())?;
            fn inner($arg0 : $typ0) -> Result<$otyp, String> {
                $($body)*
            }
            let ret: $crate::Value = inner($arg0)?.into();
            Ok(::std::sync::Arc::new(ret))
        }
    };
    (
        $(#[$outer:meta])*
        fn $name:ident($arg0:ident : $typ0:ty$(, $arg:ident : $typ:ty)*) -> Result<$otyp:ty, String>
        { $($body:tt)* }
    ) => {
        $(#[$outer])*
        pub fn $name(
            args: &[::std::sync::Arc<::std::any::Any>]
        ) -> Result<::std::sync::Arc<::std::any::Any>, String> {
            #[allow(unused_mut)]
            let mut args = args;
            if args.is_empty() {
                return Err(String::from("at least one argument required"));
            }
            let x = &args[0];
            let $arg0 = x.downcast_ref::<$crate::Value>()
                .ok_or_else(|| "unable to downcast".to_owned())?;
            let $arg0: $typ0 = $crate::from_value($arg0)
                .ok_or_else(|| "unable to convert from Value".to_owned())?;
            $(args = &args[1..];
              let x = &args[0];
              let $arg = x.downcast_ref::<$crate::Value>()
              .ok_or_else(|| "unable to downcast".to_owned())?;
              let $arg: $typ = $crate::from_value($arg)
                .ok_or_else(|| "unable to convert from Value".to_owned())?;)*;
            fn inner($arg0 : $typ0, $($arg : $typ,)*) -> Result<$otyp, String> {
                $($body)*
            }
            let ret: $crate::Value = inner($arg0, $($arg),*)?.into();
            Ok(::std::sync::Arc::new(ret))
        }
    }
}

macro_rules! gn {
    (
        $(#[$outer:meta])*
        $name:ident($arg1:ident : ref Value, $arg2:ident : ref Value) ->
            Result<Value, String>
        { $($body:tt)* }
    ) => {
        $(#[$outer])*
        pub fn $name(args: &[Arc<Any>]) -> Result<Arc<Any>, String> {
            if args.len() != 2 {
                return Err(String::from("two arguments required"));
            }
            let x = &args[0];
            let $arg1 = x.downcast_ref::<Value>()
              .ok_or_else(|| String::from("unable to downcast"))?;
            let x = &args[1];
            let $arg2 = x.downcast_ref::<Value>()
              .ok_or_else(|| String::from("unable to downcast"))?;
            fn inner($arg1: &Value, $arg2: &Value) -> Result<Value, String> {
                $($body)*
            }
            Ok(Arc::new(inner($arg1, $arg2)?))
        }
    }
}

///	Returns the boolean OR of its arguments by returning the
///	first non-empty argument or the last argument, that is,
///	"or x y" behaves as "if x then x else y". All the
///	arguments are evaluated.
///
/// # Example
/// ```
/// use gtmpl::template;
/// let equal = template("{{ or 1 2.0 false . }}", "foo");
/// assert_eq!(&equal.unwrap(), "1");
/// ```
pub fn or(args: &[Arc<Any>]) -> Result<Arc<Any>, String> {
    for arg in args {
        if is_true(arg) {
            return Ok(Arc::clone(arg));
        }
    }
    args.into_iter()
        .last()
        .map(|a| Arc::clone(a))
        .ok_or_else(|| String::from("and needs at least one argument"))
}

/// Returns the boolean AND of its arguments by returning the
///	first empty argument or the last argument, that is,
///	"and x y" behaves as "if x then y else x". All the
///	arguments are evaluated.
///
/// # Example
/// ```
/// use gtmpl::template;
/// let equal = template("{{ and 1 2.0 true . }}", "foo");
/// assert_eq!(&equal.unwrap(), "foo");
/// ```
pub fn and(args: &[Arc<Any>]) -> Result<Arc<Any>, String> {
    for arg in args {
        if !is_true(arg) {
            return Ok(Arc::clone(arg));
        }
    }
    args.into_iter()
        .last()
        .map(|a| Arc::clone(a))
        .ok_or_else(|| String::from("and needs at least one argument"))
}

/// Returns the boolean negation of its single argument.
///
/// # Example
/// ```
/// use gtmpl::template;
/// let equal = template("{{ not 0 }}", "");
/// assert_eq!(&equal.unwrap(), "true");
/// ```
pub fn not(args: &[Arc<Any>]) -> Result<Arc<Any>, String> {
    if args.len() != 1 {
        Err(String::from("not requires a single argument"))
    } else {
        Ok(varc!(!is_true(&args[0])))
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
pub fn len(args: &[Arc<Any>]) -> Result<Arc<Any>, String> {
    if args.len() != 1 {
        return Err(String::from("len requires exactly 1 arugment"));
    }
    let arg = &args[0];
    let len = if let Some(x) = arg.downcast_ref::<Value>() {
        match *x {
            Value::String(ref s) => s.len(),
            Value::Array(ref a) => a.len(),
            Value::Object(ref o) => o.len(),
            _ => {
                return Err(format!("unable to call len on {}", x));
            }
        }
    } else {
        return Err(String::from("unable to call len on the given argument"));
    };

    Ok(varc!(len))
}

/// Returns the result of calling the first argument, which
///	must be a function, with the remaining arguments as parameters.
///
/// # Example
/// ```
/// #[macro_use]
/// extern crate gtmpl;
/// extern crate gtmpl_value;
/// use gtmpl_value::Function;
/// use gtmpl::{template, Value};
///
/// fn main() {
///     gtmpl_fn!(
///     fn add(a: u64, b: u64) -> Result<u64, String> {
///         Ok(a + b)
///     });
///     let equal = template(r#"{{ call . 1 2 }}"#, Value::Function(Function { f: add }));
///     assert_eq!(&equal.unwrap(), "3");
/// }
/// ```
pub fn call(args: &[Arc<Any>]) -> Result<Arc<Any>, String> {
    let vals: Vec<&Value> = args.iter()
        .map(|arg| {
            arg.downcast_ref::<Value>()
                .ok_or_else(|| String::from("print requires arguemnts of type Value"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    if vals.is_empty() {
        Err(String::from("call requires at least on argument"))
    } else if let Value::Function(ref f) = *vals[0] {
        (f.f)(&args[1..])
    } else {
        Err(String::from(
            "call requires the first argument to be a function",
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
pub fn print(args: &[Arc<Any>]) -> Result<Arc<Any>, String> {
    let vals: Vec<&Value> = args.iter()
        .map(|arg| {
            arg.downcast_ref::<Value>()
                .ok_or_else(|| String::from("print requires arguemnts of type Value"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut no_space = true;
    let mut s = String::new();
    for val in vals {
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
    Ok(varc!(s))
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
pub fn println(args: &[Arc<Any>]) -> Result<Arc<Any>, String> {
    let vals: Vec<&Value> = args.iter()
        .map(|arg| {
            arg.downcast_ref::<Value>()
                .ok_or_else(|| String::from("print requires arguemnts of type Value"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut iter = vals.iter();
    let s = match iter.next() {
        None => String::from("\n"),
        Some(first_elt) => {
            let (lower, _) = iter.size_hint();
            let mut result = String::with_capacity(lower + 1);
            if let Value::String(ref v) = *(*first_elt) {
                result.push_str(v);
            } else {
                write!(&mut result, "{}", first_elt).unwrap();
            }
            for elt in iter {
                result.push_str(" ");
                if let Value::String(ref v) = *(*elt) {
                    result.push_str(v);
                } else {
                    write!(&mut result, "{}", elt).unwrap();
                }
            }
            result.push_str("\n");
            result
        }
    };
    Ok(varc!(s))
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
pub fn printf(args: &[Arc<Any>]) -> Result<Arc<Any>, String> {
    let vals: Vec<&Value> = args.iter()
        .map(|arg| {
            arg.downcast_ref::<Value>()
                .ok_or_else(|| String::from("print requires arguemnts of type Value"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    if vals.is_empty() {
        return Err("printf requires at least one argument".to_owned());
    }
    if let Value::String(ref s) = *vals[0] {
        let s = sprintf(s, &vals[1..])?;
        Ok(varc!(s))
    } else {
        Err("printf requires a format string".to_owned())
    }
}

/// Returns the result of indexing its first argument by the
///	following arguments. Thus "index x 1 2 3" is, in Go syntax,
///	x[1][2][3]. Each indexed item must be a map, slice or array.
///
/// # Example
/// ```
/// use gtmpl::template;
/// let ctx = vec![23, 42, 7];
/// let index = template("{{ index . 1 }}", ctx);
/// assert_eq!(&index.unwrap(), "42");
/// ```
pub fn index(args: &[Arc<Any>]) -> Result<Arc<Any>, String> {
    if args.len() < 2 {
        return Err(String::from("index requires at least 2 arugments"));
    }
    let mut col = args[0]
        .downcast_ref::<Value>()
        .ok_or_else(|| String::from("index arguments must be of type Value"))?;
    for val in &args[1..] {
        if let Some(k) = val.downcast_ref::<Value>() {
            col = get_item(col, k)?;
        } else {
            return Err(String::from("index arguments must be of type Value"));
        }
    }

    Ok(Arc::new(col.clone()))
}

fn get_item<'a>(col: &'a Value, key: &Value) -> Result<&'a Value, String> {
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
        Value::Map(_) => Ok(ret.unwrap_or_else(|| &Value::NoValue)),
        _ => ret.ok_or_else(|| format!("unabled to get {} in {}", key, col)),
    }
}

#[doc = "
Returns the escaped value of the textual representation of
its arguments in a form suitable for embedding in a URL query.

# Example
```
use gtmpl::template;
let url = template(r#\"{{ urlquery \"foo bar?\" }}\"#, 0);
assert_eq!(&url.unwrap(), \"foo%20bar%3F\");
```
"]
pub fn urlquery(args: &[Arc<Any>]) -> Result<Arc<Any>, String> {
    if args.len() != 1 {
        return Err(String::from("urlquery requires one argument"));
    }
    let val = args[0]
        .downcast_ref::<Value>()
        .ok_or_else(|| String::from("unable to downcast"))?;

    match *val {
        Value::String(ref s) => Ok(varc!(
            utf8_percent_encode(s, DEFAULT_ENCODE_SET).to_string()
        )),
        _ => Err(String::from("Arguments need to be of type String")),
    }
}

#[doc = "
Returns the boolean truth of arg1 == arg2 [== arg3 ...]

# Example
```
use gtmpl::template;
let equal = template(\"{{ eq 1 1 . }}\", 1);
assert_eq!(&equal.unwrap(), \"true\");
```
"]
pub fn eq(args: &[Arc<Any>]) -> Result<Arc<Any>, String> {
    if args.len() < 2 {
        return Err(String::from("eq requires at least 2 arguments"));
    }
    let unpack = || String::from("Arguments need to be of type Value.");
    let first = args[0].downcast_ref::<Value>().ok_or_else(unpack)?;
    Ok(Arc::new(Value::from(
        args.iter()
            .skip(1)
            .map(|x| x.downcast_ref::<Value>())
            .all(|x| x.map(|x| x == first).unwrap_or(false)),
    )))
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
ne(a: ref Value, b: ref Value) -> Result<Value, String> {
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
lt(a: ref Value, b: ref Value) -> Result<Value, String> {
    let ret = match cmp(a, b) {
        None => return Err(format!("unable to compare {} and {}", a, b)),
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
le(a: ref Value, b: ref Value) -> Result<Value, String> {
    let ret = match cmp(a, b) {
        None => return Err(format!("unable to compare {} and {}", a, b)),
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
gt(a: ref Value, b: ref Value) -> Result<Value, String> {
    let ret = match cmp(a, b) {
        None => return Err(format!("unable to compare {} and {}", a, b)),
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
ge(a: ref Value, b: ref Value) -> Result<Value, String> {
    let ret = match cmp(a, b) {
        None => return Err(format!("unable to compare {} and {}", a, b)),
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
    fn test_eq() {
        let vals: Vec<Arc<Any>> = vec![varc!("foo".to_owned()), varc!("foo".to_owned())];
        let ret = eq(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::Bool(true)));
        let vals: Vec<Arc<Any>> = vec![varc!(1u32), varc!(1u32), varc!(1i8)];
        let ret = eq(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::Bool(true)));
        let vals: Vec<Arc<Any>> = vec![varc!(false), varc!(false), varc!(false)];
        let ret = eq(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::Bool(true)));
    }

    #[test]
    fn test_and() {
        let vals: Vec<Arc<Any>> = vec![varc!(0i32), varc!(1u8)];
        let ret = and(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(0i32)));

        let vals: Vec<Arc<Any>> = vec![varc!(1i32), varc!(2u8)];
        let ret = and(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(2u8)));
    }

    #[test]
    fn test_or() {
        let vals: Vec<Arc<Any>> = vec![varc!(0i32), varc!(1u8)];
        let ret = or(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(1u8)));

        let vals: Vec<Arc<Any>> = vec![varc!(0i32), varc!(0u8)];
        let ret = or(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(0u8)));
    }

    #[test]
    fn test_ne() {
        let vals: Vec<Arc<Any>> = vec![varc!(0i32), varc!(1u8)];
        let ret = ne(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(true)));

        let vals: Vec<Arc<Any>> = vec![varc!(0i32), varc!(0u8)];
        let ret = ne(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(false)));

        let vals: Vec<Arc<Any>> = vec![varc!("foo"), varc!("bar")];
        let ret = ne(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(true)));

        let vals: Vec<Arc<Any>> = vec![varc!("foo"), varc!("foo")];
        let ret = ne(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(false)));
    }

    #[test]
    fn test_lt() {
        let vals: Vec<Arc<Any>> = vec![varc!(-1i32), varc!(1u8)];
        let ret = lt(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(true)));

        let vals: Vec<Arc<Any>> = vec![varc!(0i32), varc!(0u8)];
        let ret = lt(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(false)));

        let vals: Vec<Arc<Any>> = vec![varc!(1i32), varc!(0u8)];
        let ret = lt(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(false)));
    }

    #[test]
    fn test_le() {
        let vals: Vec<Arc<Any>> = vec![varc!(-1i32), varc!(1u8)];
        let ret = le(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(true)));

        let vals: Vec<Arc<Any>> = vec![varc!(0i32), varc!(0u8)];
        let ret = le(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(true)));

        let vals: Vec<Arc<Any>> = vec![varc!(1i32), varc!(0u8)];
        let ret = le(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(false)));
    }

    #[test]
    fn test_gt() {
        let vals: Vec<Arc<Any>> = vec![varc!(-1i32), varc!(1u8)];
        let ret = gt(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(false)));

        let vals: Vec<Arc<Any>> = vec![varc!(0i32), varc!(0u8)];
        let ret = gt(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(false)));

        let vals: Vec<Arc<Any>> = vec![varc!(1i32), varc!(0u8)];
        let ret = gt(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(true)));
    }

    #[test]
    fn test_ge() {
        let vals: Vec<Arc<Any>> = vec![varc!(-1i32), varc!(1u8)];
        let ret = ge(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(false)));

        let vals: Vec<Arc<Any>> = vec![varc!(0i32), varc!(0u8)];
        let ret = ge(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(true)));

        let vals: Vec<Arc<Any>> = vec![varc!(1i32), varc!(0u8)];
        let ret = ge(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(true)));
    }

    #[test]
    fn test_print() {
        let vals: Vec<Arc<Any>> = vec![varc!("foo"), varc!(1u8)];
        let ret = print(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from("foo1")));

        let vals: Vec<Arc<Any>> = vec![varc!("foo"), varc!(1u8), varc!(2)];
        let ret = print(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from("foo1 2")));

        let vals: Vec<Arc<Any>> = vec![varc!(true), varc!(1), varc!("foo"), varc!(2)];
        let ret = print(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from("true 1foo2")));
    }

    #[test]
    fn test_println() {
        let vals: Vec<Arc<Any>> = vec![varc!("foo"), varc!(1u8)];
        let ret = println(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from("foo 1\n")));

        let vals: Vec<Arc<Any>> = vec![];
        let ret = println(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from("\n")));
    }

    #[test]
    fn test_index() {
        let vals: Vec<Arc<Any>> = vec![varc!(vec![vec![1, 2], vec![3, 4]]), varc!(1), varc!(0)];
        let ret = index(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(3)));

        let mut o = HashMap::new();
        o.insert(String::from("foo"), vec![String::from("bar")]);
        let col = Arc::new(Value::from(o));
        let vals: Vec<Arc<Any>> = vec![col, varc!("foo"), varc!(0)];
        let ret = index(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from("bar")));

        let mut o = HashMap::new();
        o.insert(String::from("foo"), String::from("bar"));
        let col = Arc::new(Value::from(o));
        let vals: Vec<Arc<Any>> = vec![col, varc!("foo2")];
        let ret = index(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::NoValue));
    }

    #[test]
    fn test_builtins() {
        let vals: Vec<Arc<Any>> = vec![varc!("foo".to_owned()), varc!("foo".to_owned())];
        let builtin_eq = BUILTINS
            .iter()
            .find(|&&(n, _)| n == "eq")
            .map(|&(_, f)| f)
            .unwrap();
        let ret = builtin_eq(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::Bool(true)));
    }

    #[test]
    fn test_gtmpl_fn() {
        gtmpl_fn!(
            fn add(a: u64, b: u64) -> Result<u64, String> {
                Ok(a + b)
            }
        );
        let vals: Vec<Arc<Any>> = vec![varc!(1u32), varc!(2u32)];
        let ret = add(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(3u32)));

        gtmpl_fn!(
            fn has_prefix(s: String, prefix: String) -> Result<bool, String> {
                Ok(s.starts_with(&prefix))
            }
        );
        let vals: Vec<Arc<Any>> = vec![varc!("foobar"), varc!("foo")];
        let ret = has_prefix(&vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(true)));
    }
}
