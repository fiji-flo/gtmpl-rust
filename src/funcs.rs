use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

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
        m
    };
}

macro_rules! equal_as {
    ($typ:ty, $args:ident) => {
        if $args.iter().all(|x| x.is::<$typ>()) {
            let first = $args[0].downcast_ref::<$typ>().unwrap();
            return Ok(Arc::new($args.iter()
                                    .skip(1)
                                    .map(|x| x.downcast_ref::<$typ>().unwrap())
                                    .all(|x| x == first)));
        }
    }
}

macro_rules! gfn {
    ($name:ident($($arg:ident : $typ:ty),*) -> ($($out:ty),*) { $($body:tt)* }) => {
        fn $name($($arg : $typ,)*) -> ($($out),*) {
            $($body)*
        }
    }
}

macro_rules! count_exprs {
    () => (0);
    ($head:ty $(, $tail:ty)*) => (1 + count_exprs!($($tail),*));
}


macro_rules! gn {
    (
        $gname:ident :
        $name:ident($($arg:ident : ref $typ:ty),*) ->
            $otyp:ty
        { $($body:tt)* }
    ) => {
        fn $name(mut args: Vec<Arc<Any>>) -> Result<Arc<Any>, String> {
            $(let x = args.remove(0);
              let $arg = x.downcast_ref::<$typ>()
              .ok_or_else(|| format!("unable to downcast"))?;)*
            fn inner($($arg : &$typ,)*) -> $otyp {
                $($body)*
            }
            Ok(Arc::new(inner($($arg,)*)))
        }
        static $gname: (Func, i32) = (
            $name,
            count_exprs!($($arg),*),
        );
    }
}

gn!(ADD: add(a: ref i32, b: ref i32) -> i32 {
    a + b
});

// Thanks to: https://danielkeep.github.io/quick-intro-to-macros.html
gfn!(foo(a: usize, b: String) -> (i32, usize) {
    println!("{} {}", a, b);
    (0, 0)
});

#[derive(PartialEq)]
enum Num {
    None,
    Int(i64),
    Uint(u64),
    Float(f64),
}

static GEQ: (Func, i32) = (eq, -1);

fn eq(args: Vec<Arc<Any>>) -> Result<Arc<Any>, String> {
    if args.len() < 2 {
        return Err(format!("eq requires at least 2 arugments"));
    }
    equal_as!(String, args);
    equal_as!(bool, args);
    equal_as!(Value, args);
    let first = to_num(&args[0]);
    if first != Num::None {
        let equals = args.iter().skip(1).all(|val| match (&first, to_num(val)) {
            (&Num::None, _) | (_, Num::None) => false,
            (&Num::Uint(l), Num::Uint(r)) => l == r,
            (&Num::Int(l), Num::Int(r)) => l == r,
            (&Num::Uint(l), Num::Int(r)) => l as i64 == r,
            (&Num::Int(l), Num::Uint(r)) => l == r as i64,
            (&Num::Float(l), Num::Float(r)) => l == r,
            (&Num::Int(l), Num::Float(r)) => l as f64 == r,
            (&Num::Uint(l), Num::Float(r)) => l as f64 == r,
            (&Num::Float(l), Num::Int(r)) => l == r as f64,
            (&Num::Float(l), Num::Uint(r)) => l == r as f64,
        });
        return Ok(Arc::new(equals));
    }
    Err(format!("unable to compare arguments"))
}

fn to_num(val: &Arc<Any>) -> Num {
    if let Some(i) = val.downcast_ref::<u64>() {
        return Num::Uint(*i);
    }
    if let Some(i) = val.downcast_ref::<u32>() {
        return Num::Uint(*i as u64);
    }
    if let Some(i) = val.downcast_ref::<u16>() {
        return Num::Uint(*i as u64);
    }
    if let Some(i) = val.downcast_ref::<u8>() {
        return Num::Uint(*i as u64);
    }
    if let Some(i) = val.downcast_ref::<i64>() {
        return Num::Int(*i);
    }
    if let Some(i) = val.downcast_ref::<i32>() {
        return Num::Int(*i as i64);
    }
    if let Some(i) = val.downcast_ref::<i16>() {
        return Num::Int(*i as i64);
    }
    if let Some(i) = val.downcast_ref::<i8>() {
        return Num::Int(*i as i64);
    }
    if let Some(f) = val.downcast_ref::<f64>() {
        return Num::Float(*f);
    }
    if let Some(f) = val.downcast_ref::<f32>() {
        return Num::Float(*f as f64);
    }
    Num::None
}

#[cfg(test)]
mod tests_mocked {
    use super::*;

    #[test]
    fn test_eq() {
        let vals: Vec<Arc<Any>> = vec![Arc::new("foo".to_owned()), Arc::new("foo".to_owned())];
        let ret = eq(vals).unwrap();
        let ret_ = ret.downcast_ref::<bool>();
        assert_eq!(ret_, Some(&true));
        let vals: Vec<Arc<Any>> = vec![Arc::new(1u32), Arc::new(1f32), Arc::new(1i8)];
        let ret = eq(vals).unwrap();
        let ret_ = ret.downcast_ref::<bool>();
        assert_eq!(ret_, Some(&true));
        let vals: Vec<Arc<Any>> = vec![Arc::new(false), Arc::new(false), Arc::new(false)];
        let ret = eq(vals).unwrap();
        let ret_ = ret.downcast_ref::<bool>();
        assert_eq!(ret_, Some(&true));
    }

    #[test]
    fn test_builtins() {
        let vals: Vec<Arc<Any>> = vec![Arc::new("foo".to_owned()), Arc::new("foo".to_owned())];
        let builtin_eq = BUILTINS.get("eq").unwrap();
        let ret = builtin_eq(vals).unwrap();
        let ret_ = ret.downcast_ref::<bool>();
        assert_eq!(ret_, Some(&true));
    }

    #[test]
    fn test_add() {
        let vals: Vec<Arc<Any>> = vec![Arc::new(1i32), Arc::new(2i32)];
        let ret = ADD.0(vals).unwrap();
        let ret_ = ret.downcast_ref::<i32>();
        assert_eq!(ret_, Some(&3));
    }
}
