use std::any::Any;

macro_rules! equal_as {
    ($typ:ty, $args:ident) => {
        if $args.iter().all(|x| x.is::<$typ>()) {
            let first = $args[0].downcast_ref::<$typ>().unwrap();
            return Ok(vec![Box::new($args.iter()
                                    .skip(1)
                                    .map(|x| x.downcast_ref::<$typ>().unwrap())
                                    .all(|x| x == first))]);
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

fn eq(args: &[Box<Any>]) -> Result<Vec<Box<Any>>, String> {
    if args.len() < 2 {
        return Err(format!("eq requires at least 2 arugments"));
    }
    equal_as!(String, args);
    equal_as!(bool, args);
    let first = to_num(&args[0]);
    if first != Num::None {
        let equals = args.iter()
            .skip(1)
            .all(|val| match (&first, to_num(val)) {
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
        return Ok(vec![Box::new(equals)]);
    }
    Err(format!("unable to compare arguments"))
}

fn to_num(val: &Box<Any>) -> Num {
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
        let vals: Vec<Box<Any>> = vec![Box::new("foo".to_owned()), Box::new("foo".to_owned())];
        let ret = eq(&vals).unwrap();
        let ret2 = &ret[0];
        let ret3 = ret2.downcast_ref::<bool>();
        assert_eq!(ret3, Some(&true));
        let vals: Vec<Box<Any>> = vec![Box::new(1u32), Box::new(1f32), Box::new(1i8)];
        let ret = eq(&vals).unwrap();
        let ret2 = &ret[0];
        let ret3 = ret2.downcast_ref::<bool>();
        assert_eq!(ret3, Some(&true));
        let vals: Vec<Box<Any>> = vec![Box::new(false), Box::new(false), Box::new(false)];
        let ret = eq(&vals).unwrap();
        let ret2 = &ret[0];
        let ret3 = ret2.downcast_ref::<bool>();
        assert_eq!(ret3, Some(&true));
    }
}
