use std::any::Any;
use std::collections::HashMap;
use std::cmp::Ordering;
use std::sync::Arc;

use serde_json::{self, Value};

pub type Func = fn(Vec<Arc<Any>>) -> Result<Arc<Any>, String>;
enum Funcy {
    Base {
        f: Func,
        input: usize,
        output: usize,
    },
    VarArgs {
        f: Func,
        min_input: usize,
        output: usize,
    },
}

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

macro_rules! equal_as {
    ($typ:ty, $args:ident) => {
        if $args.iter().all(|x| x.is::<$typ>()) {
            let first = $args[0].downcast_ref::<$typ>().unwrap();
            return Ok(Arc::new(serde_json::to_value(
                $args.iter()
                    .skip(1)
                    .map(|x| x.downcast_ref::<$typ>().unwrap())
                    .all(|x| x == first)
            ).unwrap()));
        }
    }
}

macro_rules! gn {
    (
        $name:ident($($arg:ident : ref $typ:ty),*) ->
            $otyp:ty
        { $($body:tt)* }
    ) => {
        fn $name(mut args: Vec<Arc<Any>>) -> Result<Arc<Any>, String> {
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

gn!(add(a: ref i32, b: ref i32) -> i32 {
    Ok(a + b)
});


#[derive(PartialEq)]
enum Num {
    None,
    Int(i64),
    Uint(u64),
    Float(f64),
}

fn or(args: Vec<Arc<Any>>) -> Result<Arc<Any>, String> {
    for arg in &args {
        if is_true(&arg).0 {
            return Ok(arg.clone());
        }
    }
    args.into_iter().last().ok_or_else(
        || format!("and needs at least one argument"),
    )
}


fn and(args: Vec<Arc<Any>>) -> Result<Arc<Any>, String> {
    for arg in &args {
        if !is_true(&arg).0 {
            return Ok(arg.clone());
        }
    }
    args.into_iter().last().ok_or_else(
        || format!("and needs at least one argument"),
    )
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

fn eq(args: Vec<Arc<Any>>) -> Result<Arc<Any>, String> {
    if args.len() < 2 {
        return Err(format!("eq requires at least 2 arugments"));
    }
    equal_as!(Value, args);
    Err(format!("unable to compare arguments"))
}

gn!(ne(a: ref Value, b: ref Value) -> Value {
    Ok(Value::from(a != b))
});

gn!(lt(a: ref Value, b: ref Value) -> Value {
    let ret = match cmp(a, b) {
        None => return Err(format!("unable to compare {} and {}", a, b)),
        Some(Ordering::Less) => true,
        _ => false,
    };
    Ok(Value::from(ret))
});

gn!(le(a: ref Value, b: ref Value) -> Value {
    let ret = match cmp(a, b) {
        None => return Err(format!("unable to compare {} and {}", a, b)),
        Some(Ordering::Less) | Some(Ordering::Equal) => true,
        _ => false,
    };
    Ok(Value::from(ret))
});


gn!(gt(a: ref Value, b: ref Value) -> Value {
    let ret = match cmp(a, b) {
        None => return Err(format!("unable to compare {} and {}", a, b)),
        Some(Ordering::Greater) => true,
        _ => false,
    };
    Ok(Value::from(ret))
});

gn!(ge(a: ref Value, b: ref Value) -> Value {
    let ret = match cmp(a, b) {
        None => return Err(format!("unable to compare {} and {}", a, b)),
        Some(Ordering::Greater) | Some(Ordering::Equal) => true,
        _ => false,
    };
    Ok(Value::from(ret))
});

pub fn is_true(val: &Arc<Any>) -> (bool, bool) {
    if let Some(v) = val.downcast_ref::<Vec<Arc<Any>>>() {
        return (!v.is_empty(), true);
    }
    if let Some(v) = val.downcast_ref::<Value>() {
        if let Some(i) = v.as_i64() {
            return (i != 0i64, true);
        }
        if let Some(i) = v.as_u64() {
            return (i != 0u64, true);
        }
        if let Some(i) = v.as_f64() {
            return (i != 0f64, true);
        }
        if let Some(s) = v.as_str() {
            return (!s.is_empty(), true);
        }
        if let Some(b) = v.as_bool() {
            return (b, true);
        }
        if let Some(a) = v.as_array() {
            return (!a.is_empty(), true);
        }
        if let Some(o) = v.as_object() {
            return (!o.is_empty(), true);
        }
    }

    (true, true)
}

fn cmp(left: &Value, right: &Value) -> Option<Ordering> {
    if let (&Value::Number(ref l), &Value::Number(ref r)) = (left, right) {
        if let (Some(li), Some(ri)) = (l.as_i64(), r.as_i64()) {
            return li.partial_cmp(&ri);
        }
        if let (Some(lu), Some(ru)) = (l.as_u64(), r.as_u64()) {
            return lu.partial_cmp(&ru);
        }
        if let (Some(lf), Some(rf)) = (l.as_f64(), r.as_f64()) {
            return lf.partial_cmp(&rf);
        }
    }
    None
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
    fn test_add() {
        let vals: Vec<Arc<Any>> = vec![Arc::new(1i32), Arc::new(2i32)];
        let ret = add(vals).unwrap();
        let ret_ = ret.downcast_ref::<i32>();
        assert_eq!(ret_, Some(&3i32));
    }

    #[test]
    fn test_is_true() {
        let t: Arc<Any> = varc!(1u32);
        assert_eq!(is_true(&t).0, true);
        let t: Arc<Any> = varc!(0u32);
        assert_eq!(is_true(&t).0, false);
    }

}
