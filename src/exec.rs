use std::any::Any;
use std::io::Write;
use std::collections::{HashMap, VecDeque};

use template::Template;
use node::*;

use serde_json::Value;

type Variable<'a> = (String, &'a Box<Any>);

enum Dot {
    Dot,
}


static MAX_EXEC_DEPTH: usize = 100000;

struct State<'a, 'b, 'c, T: Write>
where
    T: 'b,
{
    template: &'a Template<'a>,
    writer: &'b mut T,
    node: Option<&'a Nodes>,
    vars: VecDeque<Variable<'a>>,
    depth: usize,
    dot: &'c Box<Any>,
}

struct Context<'d> {
    dot: &'d Box<Any>,
}

macro_rules! print_val {
    ($val:ident : $out:ident <- $($typ:ty,)*) => {
        $(
            if let Some(v) = $val.downcast_ref::<$typ>() {
                write!($out.writer, "{}", v).map_err(|e| format!("{}", e))?;
                return Ok(())
            }
        )*
    }
}

impl<'a, 'b> Template<'a> {
    fn execute<T: Write>(&mut self, writer: &'b mut T, data: &'b Box<Any>) -> Result<(), String> {
        let mut vars = VecDeque::new();
        vars.push_back(("$".to_owned(), data));

        let mut state = State {
            template: &self,
            writer,
            node: None,
            vars,
            depth: 0,
            dot: data,
        };

        let root = self.tree_ids
            .get(&1usize)
            .and_then(|name| self.tree_set.get(name))
            .and_then(|tree| tree.root.as_ref())
            .ok_or_else(|| {
                format!("{} is an incomplete or empty template", self.name)
            })?;
        let ctx = Context { dot: data };
        state.walk(&ctx, root)?;

        Ok(())
    }
}

impl<'a, 'b, 'c, T: Write> State<'a, 'b, 'c, T> {
    fn walk_list(&mut self, ctx: &Context, node: &'a ListNode) -> Result<(), String> {
        for n in &node.nodes {
            self.walk(ctx, n)?;
        }
        Ok(())
    }

    fn walk(&mut self, ctx: &Context, node: &'a Nodes) -> Result<(), String> {
        self.node = Some(node);
        match *node {
            Nodes::Action(ref n) => {
                let val = self.eval_pipeline(ctx, node)?;
                if !n.pipe.decl.is_empty() {
                    self.print_value(&val)?;
                }
                return Ok(());
            }
            Nodes::If(_) | Nodes::With(_) => {
                return self.walk_if_or_with(node, ctx);
            }
            Nodes::List(ref n) => return self.walk_list(ctx, n),
            Nodes::Text(ref n) => write!(self.writer, "{}", n).map_err(|e| format!("{}", e))?,
            _ => {}
            // TODO
        }
        Ok(())
    }

    fn eval_pipeline(&mut self, ctx: &Context, node: &'a Nodes) -> Result<Box<Any>, String> {
        self.node = Some(node);
        let mut val: Option<Box<Any>> = None;
        if let &Nodes::Pipe(ref pipe) = node {
            val = Some(self.eval_pipeline_raw(ctx, pipe)?);
        }
        val.ok_or_else(|| format!("error evaluating pipeline {}", node))
    }

    fn eval_pipeline_raw(&mut self, ctx: &Context, pipe: &'a PipeNode) -> Result<Box<Any>, String> {
        let mut val: Option<Box<Any>> = None;
        for cmd in &pipe.cmds {
            val = Some(self.eval_command(ctx, cmd, val)?);
            // TODO
        }
        val.ok_or_else(|| format!("error evaluating pipeline {}", pipe))
    }

    fn eval_command(
        &mut self,
        ctx: &Context,
        cmd: &CommandNode,
        val: Option<Box<Any>>,
    ) -> Result<Box<Any>, String> {
        let first_word = &cmd.args.first().ok_or_else(|| {
            format!("no arguments for command node: {}", cmd)
        })?;

        match *first_word {
            &Nodes::Field(ref n) => return self.eval_field_node(ctx, n, &cmd.args, val),
            _ => {}
        }
        not_a_function(&cmd.args, val)?;
        match *first_word {
            &Nodes::Bool(ref n) => return Ok(Box::new(n.val)),
            &Nodes::Dot(_) => return Ok(Box::new(Dot::Dot)),
            _ => {}
        }


        Err(format!("DOOM"))
    }

    fn eval_field_node(
        &mut self,
        ctx: &Context,
        field: &FieldNode,
        args: &[Nodes],
        fin: Option<Box<Any>>,
    ) -> Result<Box<Any>, String> {
        return self.eval_field_chain(ctx, ctx.dot, &field.ident, args, fin)
    }

    fn eval_field_chain(
        &mut self,
        ctx: &Context,
        receiver: &Box<Any>,
        ident: &[String],
        args: &[Nodes],
        fin: Option<Box<Any>>,
    ) -> Result<Box<Any>, String> {
        let n = ident.len();
        if n < 1 {
            return Err(format!("field chain without fields :/"));
        }
        // TODO clean shit up
        let mut r: Box<Any> = Box::new(0);
        for i in 0..n-1 {
            r = self.eval_field(ctx, if i == 0 { receiver } else { &r }, &ident[i], &[], None)?;
        }
        self.eval_field(ctx, if n == 1 { receiver } else { &r }, &ident[n-1], args, fin)
    }

    fn eval_field(&mut self, ctx: &Context, receiver: &Box<Any>, fieldName: &str, args: &[Nodes], fin: Option<Box<Any>>) -> Result<Box<Any>, String> {
        if let Some(ref val) = receiver.downcast_ref::<Value>() {
            return val.get(fieldName).map(|v| Box::new(v.clone()) as Box<Any>).ok_or_else(|| format!("no field {} for {}", fieldName, val));
        }

        Err(format!("DOOM"))
    }

    fn walk_if_or_with(&mut self, node: &'a Nodes, ctx: &Context) -> Result<(), String> {
        let pipe = match *node {
            Nodes::If(ref n) => &n.pipe,
            Nodes::With(ref n) => &n.pipe,
            _ => return Err(format!("expected if or with node, got {}", node)),
        };
        let val = self.eval_pipeline_raw(ctx, &pipe)?;
        let truth = is_true(val_ref(ctx, &val));
        if truth.0 {
            match *node {
                Nodes::If(ref n) => self.walk_list(ctx, &n.list)?,
                Nodes::With(ref n) => {
                    let ctx = Context { dot: &val };
                    self.walk_list(&ctx, &n.list)?;
                }
                _ => {}
            }
        } else {
            match *node {
                Nodes::If(ref n) => {
                    if let Some(ref otherwise) = n.else_list {
                        self.walk_list(ctx, otherwise)?;
                    }
                }
                Nodes::With(ref n) => {
                    if let Some(ref otherwise) = n.else_list {
                        self.walk_list(ctx, otherwise)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn print_value(&mut self, val: &Box<Any>) -> Result<(), String> {
        print_val!{ val: self <-
                    String,
                    bool,
                    u8,
                    u16,
                    u32,
                    u64,
                    i8,
                    i16,
                    i32,
                    i64,
                    f32,
                    f64,
        };
        Err(format!("unable to format value"))
    }
}

fn not_a_function(args: &[Nodes], val: Option<Box<Any>>) -> Result<(), String> {
    if args.len() > 1 || val.is_some() {
        return Err(format!("can't give arument to non-function {}", args[0]));
    }
    Ok(())
}


macro_rules! non_zero {
    ($val:ident -> $($typ:ty),*) => {
        $(
            if let Some(i) = $val.downcast_ref::<$typ>() {
                return (i != &(0 as $typ), true);
            }
          )*
    }
}

fn val_ref<'e>(ctx: &'e Context, val: &'e Box<Any>) -> &'e Box<Any> {
    if let Some(_) = val.downcast_ref::<Dot>() {
        return ctx.dot;
    }
    val
}

fn is_true(val: &Box<Any>) -> (bool, bool) {
    if let Some(i) = val.downcast_ref::<bool>() {
        return (*i, true);
    }
    if let Some(s) = val.downcast_ref::<String>() {
        return (!s.is_empty(), true);
    }
    if let Some(v) = val.downcast_ref::<Vec<Box<Any>>>() {
        return (!v.is_empty(), true);
    }
    if let Some(v) = val.downcast_ref::<HashMap<String, Box<Any>>>() {
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



    non_zero!(val -> u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);
    (true, true)
}

#[cfg(test)]
mod tests_mocked {
    use super::*;
    use serde_json;

    #[test]
    fn test_is_true() {
        let t: Box<Any> = Box::new(1u32);
        assert_eq!(is_true(&t).0, true);
        let t: Box<Any> = Box::new(0u32);
        assert_eq!(is_true(&t).0, false);
    }

    #[test]
    fn simple_template() {
        let data: Box<Any> = Box::new(1);
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(t.parse(r#"{{ if false }} 2000 {{ end }}"#).is_ok());
        let out = t.execute(&mut w, &data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "");

        let data: Box<Any> = Box::new(1);
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(t.parse(r#"{{ if true }} 2000 {{ end }}"#).is_ok());
        let out = t.execute(&mut w, &data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), " 2000 ");

        let data: Box<Any> = Box::new(1);
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(t.parse(r#"{{ if true -}} 2000 {{- end }}"#).is_ok());
        let out = t.execute(&mut w, &data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "2000");

        let data: Box<Any> = Box::new(1);
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ if false -}} 2000 {{- else -}} 3000 {{- end }}"#)
                .is_ok()
        );
        let out = t.execute(&mut w, &data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "3000");
    }

    #[test]
    fn basic_dot() {
        let data: Box<Any> = Box::new(1);
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ if . -}} 2000 {{- else -}} 3000 {{- end }}"#)
                .is_ok()
        );
        let out = t.execute(&mut w, &data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "2000");

        let data: Box<Any> = Box::new(false);
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ if . -}} 2000 {{- else -}} 3000 {{- end }}"#)
                .is_ok()
        );
        let out = t.execute(&mut w, &data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "3000");
    }

    #[test]
    fn basic_dot_value() {
        #[derive(Serialize)]
        struct Foo {
            foo: u8,
        }
        #[derive(Serialize)]
        struct Bar {
            bar: Foo,
        }
        let foo = Foo { foo: 1 };
        let data: Box<Any> = Box::new(serde_json::to_value(foo).unwrap());
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ if .foo -}} 2000 {{- else -}} 3000 {{- end }}"#)
                .is_ok()
        );
        let out = t.execute(&mut w, &data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "2000");

        let foo = Foo { foo: 0 };
        let data: Box<Any> = Box::new(serde_json::to_value(foo).unwrap());
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ if .foo -}} 2000 {{- else -}} 3000 {{- end }}"#)
                .is_ok()
        );
        let out = t.execute(&mut w, &data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "3000");

        let bar = Bar { bar: Foo { foo: 1 } };
        let data: Box<Any> = Box::new(serde_json::to_value(bar).unwrap());
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ if .bar.foo -}} 2000 {{- else -}} 3000 {{- end }}"#)
                .is_ok()
        );
        let out = t.execute(&mut w, &data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "2000");

        let bar = Bar { bar: Foo { foo: 0 } };
        let data: Box<Any> = Box::new(serde_json::to_value(bar).unwrap());
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ if .bar.foo -}} 2000 {{- else -}} 3000 {{- end }}"#)
                .is_ok()
        );
        let out = t.execute(&mut w, &data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "3000");
    }
}
