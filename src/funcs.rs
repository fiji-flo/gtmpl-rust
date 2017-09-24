use std::any::Any;
use std::collections::HashMap;
use std::cmp::Ordering;
use std::sync::Arc;

use serde_json::{self, Value};

/// Function type that is used to implement builtin and custom functions.
pub type Func = fn(Vec<Arc<Any>>) -> Result<Arc<Any>, String>;

lazy_static! {
    pub static ref BUILTINS: HashMap<String, Func> = {
        let mut m = HashMap::new();
        m.insert("eq".to_owned(), eq as Func);
        m.insert("ne".to_owned(), ne as Func);
        m.insert("lt".to_owned(), lt as Func);
        m.insert("le".to_owned(), le as Func);
        m.insert("gt".to_owned(), gt as Func);
        m.insert("ge".to_owned(), ge as Func);
        m.insert("length".to_owned(), length as Func);
        m.insert("and".to_owned(), and as Func);
        m.insert("or".to_owned(), or as Func);
        m
    };
}

macro_rules! gn {
    (
        $(#[$outer:meta])*
        $name:ident($($arg:ident : ref $typ:ty),*) ->
            $otyp:ty
        { $($body:tt)* }
    ) => {
        $(#[$outer])*
        pub fn $name(mut args: Vec<Arc<Any>>) -> Result<Arc<Any>, String> {
            $(let x = args.remove(0);
              let $arg = x.downcast_ref::<$typ>()
              .ok_or_else(|| format!("unable to downcast"))?;)*
            fn inner($($arg : &$typ,)*) -> Result<$otyp, String> {
                $($body)*
            }
            Ok(Arc::new(inner($($arg,)*)?))
        }
    }
}

#[derive(PartialEq)]
enum Num {
    None,
    Int(i64),
    Uint(u64),
    Float(f64),
}

fn or(args: Vec<Arc<Any>>) -> Result<Arc<Any>, String> {
    for arg in &args {
        if is_true(&arg) {
            return Ok(arg.clone());
        }
    }
    args.into_iter().last().ok_or_else(|| {
        format!("and needs at least one argument")
    })
}


fn and(args: Vec<Arc<Any>>) -> Result<Arc<Any>, String> {
    for arg in &args {
        if !is_true(&arg) {
            return Ok(arg.clone());
        }
    }
    args.into_iter().last().ok_or_else(|| {
        format!("and needs at least one argument")
    })
}

fn length(args: Vec<Arc<Any>>) -> Result<Arc<Any>, String> {
    if args.len() != 1 {
        return Err(format!("length requires exactly 1 arugment"));
    }
    let arg = &args[0];
    let len = if let Some(x) = arg.downcast_ref::<Value>() {
        match *x {
            Value::String(ref s) => s.len(),
            Value::Array(ref a) => a.len(),
            Value::Object(ref o) => o.len(),
            _ => {
                return Err(format!("unable to call length on {}", x));
            }
        }
    } else {
        return Err(format!("unable to call length on the given argument"));
    };

    Ok(Arc::new(serde_json::to_value(len).unwrap()))
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
pub fn eq(args: Vec<Arc<Any>>) -> Result<Arc<Any>, String> {
    if args.len() < 2 {
        return Err(format!("eq requires at least 2 arguments"));
    }
    let unpack = || format!("Arguments need to be of type Value.");
    let first = args[0].downcast_ref::<Value>().ok_or_else(unpack)?;
    return Ok(Arc::new(Value::from(
        args.iter().skip(1).map(|x| x.downcast_ref::<Value>()).all(
            |x| {
                x.map(|x| x == first).unwrap_or(false)
            },
        ),
    )));
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
ne(a: ref Value, b: ref Value) -> Value {
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
lt(a: ref Value, b: ref Value) -> Value {
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
le(a: ref Value, b: ref Value) -> Value {
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
gt(a: ref Value, b: ref Value) -> Value {
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
ge(a: ref Value, b: ref Value) -> Value {
    let ret = match cmp(a, b) {
        None => return Err(format!("unable to compare {} and {}", a, b)),
        Some(Ordering::Greater) | Some(Ordering::Equal) => true,
        _ => false,
    };
    Ok(Value::from(ret))
});

/// Returns
pub fn is_true(val: &Arc<Any>) -> bool {
    if let Some(v) = val.downcast_ref::<Value>() {
        if let Some(i) = v.as_i64() {
            return i != 0i64;
        }
        if let Some(i) = v.as_u64() {
            return i != 0u64;
        }
        if let Some(i) = v.as_f64() {
            return i != 0f64;
        }
        if let Some(s) = v.as_str() {
            return !s.is_empty();
        }
        if let Some(b) = v.as_bool() {
            return b;
        }
        if let Some(a) = v.as_array() {
            return !a.is_empty();
        }
        if let Some(o) = v.as_object() {
            return !o.is_empty();
        }
    }

    false
}

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

    macro_rules! varc(
        ($x:expr) => { Arc::new(Value::from($x)) }
    );

    #[test]
    fn test_eq() {
        let vals: Vec<Arc<Any>> = vec![varc!("foo".to_owned()), varc!("foo".to_owned())];
        let ret = eq(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::Bool(true)));
        let vals: Vec<Arc<Any>> = vec![varc!(1u32), varc!(1u32), varc!(1i8)];
        let ret = eq(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::Bool(true)));
        let vals: Vec<Arc<Any>> = vec![varc!(false), varc!(false), varc!(false)];
        let ret = eq(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::Bool(true)));
    }

    #[test]
    fn test_and() {
        let vals: Vec<Arc<Any>> = vec![varc!(0i32), varc!(1u8)];
        let ret = and(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(0i32)));

        let vals: Vec<Arc<Any>> = vec![varc!(1i32), varc!(2u8)];
        let ret = and(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(2u8)));
    }

    #[test]
    fn test_or() {
        let vals: Vec<Arc<Any>> = vec![varc!(0i32), varc!(1u8)];
        let ret = or(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(1u8)));

        let vals: Vec<Arc<Any>> = vec![varc!(0i32), varc!(0u8)];
        let ret = or(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(0u8)));
    }

    #[test]
    fn test_ne() {
        let vals: Vec<Arc<Any>> = vec![varc!(0i32), varc!(1u8)];
        let ret = ne(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(true)));

        let vals: Vec<Arc<Any>> = vec![varc!(0i32), varc!(0u8)];
        let ret = ne(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(false)));

        let vals: Vec<Arc<Any>> = vec![varc!("foo"), varc!("bar")];
        let ret = ne(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(true)));

        let vals: Vec<Arc<Any>> = vec![varc!("foo"), varc!("foo")];
        let ret = ne(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(false)));
    }

    #[test]
    fn test_lt() {
        let vals: Vec<Arc<Any>> = vec![varc!(-1i32), varc!(1u8)];
        let ret = lt(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(true)));

        let vals: Vec<Arc<Any>> = vec![varc!(0i32), varc!(0u8)];
        let ret = lt(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(false)));

        let vals: Vec<Arc<Any>> = vec![varc!(1i32), varc!(0u8)];
        let ret = lt(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(false)));
    }

    #[test]
    fn test_le() {
        let vals: Vec<Arc<Any>> = vec![varc!(-1i32), varc!(1u8)];
        let ret = le(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(true)));

        let vals: Vec<Arc<Any>> = vec![varc!(0i32), varc!(0u8)];
        let ret = le(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(true)));

        let vals: Vec<Arc<Any>> = vec![varc!(1i32), varc!(0u8)];
        let ret = le(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(false)));
    }

    #[test]
    fn test_gt() {
        let vals: Vec<Arc<Any>> = vec![varc!(-1i32), varc!(1u8)];
        let ret = gt(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(false)));

        let vals: Vec<Arc<Any>> = vec![varc!(0i32), varc!(0u8)];
        let ret = gt(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(false)));

        let vals: Vec<Arc<Any>> = vec![varc!(1i32), varc!(0u8)];
        let ret = gt(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(true)));
    }

    #[test]
    fn test_ge() {
        let vals: Vec<Arc<Any>> = vec![varc!(-1i32), varc!(1u8)];
        let ret = ge(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(false)));

        let vals: Vec<Arc<Any>> = vec![varc!(0i32), varc!(0u8)];
        let ret = ge(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(true)));

        let vals: Vec<Arc<Any>> = vec![varc!(1i32), varc!(0u8)];
        let ret = ge(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::from(true)));
    }

    #[test]
    fn test_builtins() {
        let vals: Vec<Arc<Any>> = vec![varc!("foo".to_owned()), varc!("foo".to_owned())];
        let builtin_eq = BUILTINS.get("eq").unwrap();
        let ret = builtin_eq(vals).unwrap();
        let ret_ = ret.downcast_ref::<Value>();
        assert_eq!(ret_, Some(&Value::Bool(true)));
    }

    #[test]
    fn test_is_true() {
        let t: Arc<Any> = varc!(1u32);
        assert_eq!(is_true(&t), true);
        let t: Arc<Any> = varc!(0u32);
        assert_eq!(is_true(&t), false);
    }

}
