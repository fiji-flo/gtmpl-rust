use std::any::Any;
use std::sync::Arc;
use std::io::Write;
use std::collections::{HashMap, VecDeque};

use funcs::Func;
use template::Template;
use node::*;

use serde_json::Value;

static MAX_EXEC_DEPTH: usize = 100000;

struct Variable {
    name: String,
    value: Arc<Any>,
}

struct State<'a, 'b, T: Write>
where
    T: 'b,
{
    template: &'a Template<'a>,
    writer: &'b mut T,
    node: Option<&'a Nodes>,
    vars: VecDeque<VecDeque<Variable>>,
    depth: usize,
    dot: Arc<Any>,
}

struct Context {
    dot: Arc<Any>,
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
    fn execute<T: Write>(&mut self, writer: &'b mut T, data: Arc<Any>) -> Result<(), String> {
        let mut vars: VecDeque<VecDeque<Variable>> = VecDeque::new();
        let mut dot = VecDeque::new();
        dot.push_back(Variable {
            name: "$".to_owned(),
            value: data.clone(),
        });
        vars.push_back(dot);

        let mut state = State {
            template: &self,
            writer,
            node: None,
            vars,
            depth: 0,
            dot: data.clone(),
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

impl<'a, 'b, T: Write> State<'a, 'b, T> {
    fn set_kth_last_var_value(&mut self, k: usize, value: Arc<Any>) -> Result<(), String> {
        if let Some(mut last_vars) = self.vars.back_mut() {
            let i = last_vars.len() - k;
            if let Some(kth_last_var) = last_vars.get_mut(i) {
                kth_last_var.value = value;
                return Ok(());
            }
            return Err(format!("current var context smaller than {}", k));
        }
        return Err(format!("empty var stack"));
    }

    fn var_value(&self, key: &str) -> Result<Arc<Any>, String> {
        for context in self.vars.iter().rev() {
            for var in context.iter().rev() {
                if var.name == key {
                    return Ok(var.value.clone());
                }
            }
        }
        Err(format!("variable {} not found", key))

    }

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
                let val = self.eval_pipeline_raw(ctx, &n.pipe)?;
                if n.pipe.decl.is_empty() {
                    self.print_value(&val)?;
                }
                return Ok(());
            }
            Nodes::If(_) | Nodes::With(_) => {
                return self.walk_if_or_with(node, ctx);
            }
            Nodes::Range(ref n) => return self.walk_range(ctx, n),
            Nodes::List(ref n) => return self.walk_list(ctx, n),
            Nodes::Text(ref n) => write!(self.writer, "{}", n).map_err(|e| format!("{}", e))?,
            _ => {}
            // TODO
        }
        Ok(())
    }

    fn eval_pipeline(&mut self, ctx: &Context, node: &'a Nodes) -> Result<Arc<Any>, String> {
        self.node = Some(node);
        let mut val: Option<Arc<Any>> = None;
        if let &Nodes::Pipe(ref pipe) = node {
            val = Some(self.eval_pipeline_raw(ctx, pipe)?);
        }
        val.ok_or_else(|| format!("error evaluating pipeline {}", node))
    }

    fn eval_pipeline_raw(&mut self, ctx: &Context, pipe: &PipeNode) -> Result<Arc<Any>, String> {
        let mut val: Option<Arc<Any>> = None;
        for cmd in &pipe.cmds {
            val = Some(self.eval_command(ctx, cmd, val)?);
            // TODO
        }
        let val = val.ok_or_else(
            || format!("error evaluating pipeline {}", pipe),
        )?;
        for var in &pipe.decl {
            self.vars
                .back_mut()
                .and_then(|v| {
                    Some(v.push_back(Variable {
                        name: var.ident[0].clone(),
                        value: val.clone(),
                    }))
                })
                .ok_or_else(|| format!("no stack while evaluating pipeline"))?;
        }
        Ok(val)
    }

    fn eval_command(
        &mut self,
        ctx: &Context,
        cmd: &CommandNode,
        val: Option<Arc<Any>>,
    ) -> Result<Arc<Any>, String> {
        let first_word = &cmd.args.first().ok_or_else(|| {
            format!("no arguments for command node: {}", cmd)
        })?;

        match *first_word {
            &Nodes::Field(ref n) => return self.eval_field_node(ctx, n, &cmd.args, val),
            &Nodes::Variable(ref n) => return self.eval_variable_node(ctx, n, &cmd.args, val),
            &Nodes::Pipe(ref n) => return self.eval_pipeline_raw(ctx, n),
            &Nodes::Chain(ref n) => return self.eval_chain_node(ctx, n, &cmd.args, val),
            &Nodes::Identifier(ref n) => return self.eval_function(ctx, n, &cmd.args, val),
            _ => {}
        }
        not_a_function(&cmd.args, val)?;
        match *first_word {
            &Nodes::Bool(ref n) => return Ok(Arc::new(n.val)),
            &Nodes::Dot(_) => return Ok(ctx.dot.clone()),
            &Nodes::Number(ref n) => return Ok(n.as_any_arc()),
            _ => {}
        }


        Err(format!("cannot evaluate command {}", first_word))
    }

    fn eval_function(
        &mut self,
        ctx: &Context,
        ident: &IdentifierNode,
        args: &[Nodes],
        fin: Option<Arc<Any>>,
    ) -> Result<Arc<Any>, String> {
        let name = &ident.ident;
        let function = self.template
            .funcs
            .iter()
            .rev()
            .find(|map| map.contains_key(name))
            .and_then(|map| map.get(name))
            .ok_or_else(|| format!("{} is not a defined function", name))?;
        self.eval_call(ctx, function, &name, args, fin)
    }

    fn eval_call(
        &mut self,
        ctx: &Context,
        function: &Func,
        name: &str,
        args: &[Nodes],
        fin: Option<Arc<Any>>,
    ) -> Result<Arc<Any>, String> {
        let mut arg_vals = vec![];
        for arg in &args[1..] {
            let val = self.eval_arg(ctx, arg)?;
            arg_vals.push(val);
        }

        function(arg_vals)
    }

    fn eval_chain_node(
        &mut self,
        ctx: &Context,
        chain: &ChainNode,
        args: &[Nodes],
        fin: Option<Arc<Any>>,
    ) -> Result<Arc<Any>, String> {
        if chain.field.is_empty() {
            return Err(format!("internal error: no fields in eval_chain_node"));
        }
        if let Nodes::Nil(_) = *chain.node {
            return Err(format!("inderection throug explicit nul in {}", chain));
        }
        let pipe = self.eval_arg(ctx, &*chain.node)?;
        self.eval_field_chain(ctx, pipe, &chain.field, args, fin)
    }

    fn eval_arg(&mut self, ctx: &Context, node: &Nodes) -> Result<Arc<Any>, String> {
        match *node {
            Nodes::Dot(_) => Ok(ctx.dot.clone()),
            //Nodes::Nil
            Nodes::Field(ref n) => self.eval_field_node(ctx, n, &vec![], None), // args?
            Nodes::Variable(ref n) => self.eval_variable_node(ctx, n, &vec![], None),
            Nodes::Pipe(ref n) => self.eval_pipeline_raw(ctx, n),
            // Nodes::Identifier
            Nodes::Chain(ref n) => self.eval_chain_node(ctx, n, &vec![], None),
            Nodes::String(ref n) => Ok(Arc::new(n.text.clone())),
            Nodes::Bool(ref n) => Ok(Arc::new(n.val)),
            Nodes::Number(ref n) => {
                match n.number_typ {
                    NumberType::Char | NumberType::U64 => Ok(Arc::new(n.as_u64)),
                    NumberType::I64 => Ok(Arc::new(n.as_i64)),
                    NumberType::Float => Ok(Arc::new(n.as_f64)),
                }
            }
            _ => Err(format!("cant handle {} as arg", node)),
        }

    }

    fn eval_field_node(
        &mut self,
        ctx: &Context,
        field: &FieldNode,
        args: &[Nodes],
        fin: Option<Arc<Any>>,
    ) -> Result<Arc<Any>, String> {
        return self.eval_field_chain(ctx, ctx.dot.clone(), &field.ident, args, fin);
    }

    fn eval_field_chain(
        &mut self,
        ctx: &Context,
        receiver: Arc<Any>,
        ident: &[String],
        args: &[Nodes],
        fin: Option<Arc<Any>>,
    ) -> Result<Arc<Any>, String> {
        let n = ident.len();
        if n < 1 {
            return Err(format!("field chain without fields :/"));
        }
        // TODO clean shit up
        let mut r: Arc<Any> = Arc::new(0);
        for i in 0..n - 1 {
            r = self.eval_field(
                ctx,
                if i == 0 { &receiver } else { &r },
                &ident[i],
                &[],
                None,
            )?;
        }
        self.eval_field(
            ctx,
            if n == 1 { &receiver } else { &r },
            &ident[n - 1],
            args,
            fin,
        )
    }

    fn eval_field(
        &mut self,
        ctx: &Context,
        receiver: &Arc<Any>,
        field_name: &str,
        args: &[Nodes],
        fin: Option<Arc<Any>>,
    ) -> Result<Arc<Any>, String> {
        let has_args = args.len() > 1 || fin.is_some();
        if let Some(ref val) = receiver.downcast_ref::<Value>() {
            if has_args {
                return Err(format!(
                    "{} has arguments but cannot be invoked as function",
                    field_name
                ));
            }
            return val.get(field_name)
                .map(|v| Arc::new(v.clone()) as Arc<Any>)
                .ok_or_else(|| format!("no field {} for {}", field_name, val));
        }

        Err(format!("only basic fields are supported"))
    }

    fn eval_variable_node(
        &mut self,
        ctx: &Context,
        variable: &VariableNode,
        args: &[Nodes],
        fin: Option<Arc<Any>>,
    ) -> Result<Arc<Any>, String> {
        let val = self.var_value(&variable.ident[0])?;
        if variable.ident.len() == 1 {
            not_a_function(args, fin)?;
            return Ok(val);
        }
        self.eval_field_chain(ctx, val, &variable.ident[1..], args, fin)
    }

    fn walk_if_or_with(&mut self, node: &'a Nodes, ctx: &Context) -> Result<(), String> {
        let pipe = match *node {
            Nodes::If(ref n) => &n.pipe,
            Nodes::With(ref n) => &n.pipe,
            _ => return Err(format!("expected if or with node, got {}", node)),
        };
        let val = self.eval_pipeline_raw(ctx, &pipe)?;
        let truth = is_true(&val);
        if truth.0 {
            match *node {
                Nodes::If(ref n) => self.walk_list(ctx, &n.list)?,
                Nodes::With(ref n) => {
                    let ctx = Context { dot: val };
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

    fn one_iteration(
        &mut self,
        key: String,
        val: Arc<Any>,
        range: &'a RangeNode,
    ) -> Result<(), String> {
        if range.pipe.decl.len() > 0 {
            self.set_kth_last_var_value(1, val.clone())?;
        }
        if range.pipe.decl.len() > 1 {
            self.set_kth_last_var_value(2, Arc::new(key))?;
        }
        let vars = VecDeque::new();
        self.vars.push_back(vars);
        let ctx = Context { dot: val };
        self.walk_list(&ctx, &range.list)?;
        self.vars.pop_back();
        Ok(())
    }

    fn walk_range(&mut self, ctx: &Context, range: &'a RangeNode) -> Result<(), String> {
        let val = self.eval_pipeline_raw(ctx, &range.pipe)?;
        if let Some(map) = val.downcast_ref::<HashMap<String, Arc<Any>>>() {
            for (k, v) in map {
                self.one_iteration(k.clone(), v.clone(), range)?;
            }
        }
        if let Some(value) = val.downcast_ref::<Value>() {
            match value {
                &Value::Object(ref map) => {
                    for (k, v) in map.clone() {
                        self.one_iteration(k.clone(), Arc::new(v), range)?;
                    }
                }
                _ => return Err(format!("invalid range: {:?}", value)),
            }
        }
        if let Some(ref else_list) = range.else_list {
            self.walk_list(ctx, else_list)?;
        }
        Ok(())
    }

    fn print_value(&mut self, val: &Arc<Any>) -> Result<(), String> {
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
                    isize,
                    usize,
        };
        if let Some(v) = val.downcast_ref::<Value>() {
            write!(self.writer, "{}", v).map_err(|e| format!("{}", e))?;
            return Ok(());
        }
        Err(format!("unable to format value"))
    }
}

fn not_a_function(args: &[Nodes], val: Option<Arc<Any>>) -> Result<(), String> {
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

fn is_true(val: &Arc<Any>) -> (bool, bool) {
    if let Some(i) = val.downcast_ref::<bool>() {
        return (*i, true);
    }
    if let Some(s) = val.downcast_ref::<String>() {
        return (!s.is_empty(), true);
    }
    if let Some(v) = val.downcast_ref::<Vec<Arc<Any>>>() {
        return (!v.is_empty(), true);
    }
    if let Some(v) = val.downcast_ref::<HashMap<String, Arc<Any>>>() {
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
        let t: Arc<Any> = Arc::new(1u32);
        assert_eq!(is_true(&t).0, true);
        let t: Arc<Any> = Arc::new(0u32);
        assert_eq!(is_true(&t).0, false);
    }

    #[test]
    fn simple_template() {
        let data: Arc<Any> = Arc::new(1);
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(t.parse(r#"{{ if false }} 2000 {{ end }}"#).is_ok());
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "");

        let data: Arc<Any> = Arc::new(1);
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(t.parse(r#"{{ if true }} 2000 {{ end }}"#).is_ok());
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), " 2000 ");

        let data: Arc<Any> = Arc::new(1);
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(t.parse(r#"{{ if true -}} 2000 {{- end }}"#).is_ok());
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "2000");

        let data: Arc<Any> = Arc::new(1);
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ if false -}} 2000 {{- else -}} 3000 {{- end }}"#)
                .is_ok()
        );
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "3000");
    }

    #[test]
    fn test_dot() {
        let data: Arc<Any> = Arc::new(1);
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ if . -}} 2000 {{- else -}} 3000 {{- end }}"#)
                .is_ok()
        );
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "2000");

        let data: Arc<Any> = Arc::new(false);
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ if . -}} 2000 {{- else -}} 3000 {{- end }}"#)
                .is_ok()
        );
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "3000");
    }

    #[test]
    fn test_sub() {
        let data: Arc<Any> = Arc::new(1u8);
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(t.parse(r#"{{.}}"#).is_ok());
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "1");

        #[derive(Serialize)]
        struct Foo {
            foo: u8,
        }
        let foo = Foo { foo: 1 };
        let data: Arc<Any> = Arc::new(serde_json::to_value(foo).unwrap());
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(t.parse(r#"{{.foo}}"#).is_ok());
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "1");
    }

    #[test]
    fn test_dot_value() {
        #[derive(Serialize)]
        struct Foo {
            foo: u8,
        }
        #[derive(Serialize)]
        struct Bar {
            bar: Foo,
        }
        let foo = Foo { foo: 1 };
        let data: Arc<Any> = Arc::new(serde_json::to_value(foo).unwrap());
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ if .foo -}} 2000 {{- else -}} 3000 {{- end }}"#)
                .is_ok()
        );
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "2000");

        let foo = Foo { foo: 0 };
        let data: Arc<Any> = Arc::new(serde_json::to_value(foo).unwrap());
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ if .foo -}} 2000 {{- else -}} 3000 {{- end }}"#)
                .is_ok()
        );
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "3000");

        let bar = Bar { bar: Foo { foo: 1 } };
        let data: Arc<Any> = Arc::new(serde_json::to_value(bar).unwrap());
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ if .bar.foo -}} 2000 {{- else -}} 3000 {{- end }}"#)
                .is_ok()
        );
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "2000");

        let bar = Bar { bar: Foo { foo: 0 } };
        let data: Arc<Any> = Arc::new(serde_json::to_value(bar).unwrap());
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ if .bar.foo -}} 2000 {{- else -}} 3000 {{- end }}"#)
                .is_ok()
        );
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "3000");
    }

    #[test]
    fn test_with() {
        #[derive(Serialize)]
        struct Foo {
            foo: u16,
        }
        #[derive(Serialize)]
        struct Bar {
            bar: Foo,
        }
        let foo = Foo { foo: 1000 };
        let data: Arc<Any> = Arc::new(serde_json::to_value(foo).unwrap());
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ with .foo -}} {{.}} {{- else -}} 3000 {{- end }}"#)
                .is_ok()
        );
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "1000");
    }

    #[test]
    fn test_range() {
        let mut map = HashMap::new();
        map.insert("a".to_owned(), 1);
        map.insert("b".to_owned(), 2);
        let data: Arc<Any> = Arc::new(serde_json::to_value(map).unwrap());
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(t.parse(r#"{{ range . -}} {{.}} {{- end }}"#).is_ok());
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "12");
    }

    #[test]
    fn test_proper_range() {
        let mut map = HashMap::new();
        map.insert("a".to_owned(), 1);
        map.insert("b".to_owned(), 2);
        let data: Arc<Any> = Arc::new(serde_json::to_value(map).unwrap());
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ range $k, $v := . -}} {{ $v }} {{- end }}"#)
                .is_ok()
        );
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "12");

        let mut map = HashMap::new();
        map.insert("a".to_owned(), 1);
        map.insert("b".to_owned(), 2);
        let data: Arc<Any> = Arc::new(serde_json::to_value(map).unwrap());
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ range $k, $v := . -}} {{ $k }}{{ $v }} {{- end }}"#)
                .is_ok()
        );
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "a1b2");

        let mut map = HashMap::new();
        map.insert("a".to_owned(), 1);
        map.insert("b".to_owned(), 2);
        #[derive(Serialize)]
        struct Foo {
            foo: HashMap<String, i32>,
        }
        let foo = Foo { foo: map };
        let data: Arc<Any> = Arc::new(serde_json::to_value(foo).unwrap());
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ range $k, $v := .foo -}} {{ $v }} {{- end }}"#)
                .is_ok()
        );
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "12");

        let mut map = HashMap::new();
        #[derive(Serialize)]
        struct Bar {
            bar: i32,
        }
        map.insert("a".to_owned(), Bar { bar: 1 });
        map.insert("b".to_owned(), Bar { bar: 2 });
        let data: Arc<Any> = Arc::new(serde_json::to_value(map).unwrap());
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ range $k, $v := . -}} {{ $v.bar }} {{- end }}"#)
                .is_ok()
        );
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "12");
    }

    #[test]
    fn test_length() {
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(t.parse(r#"my len is {{ length . }}"#).is_ok());
        let data: Arc<Any> = Arc::new(serde_json::to_value(vec![1, 2, 3]).unwrap());
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "my len is 3");

        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(t.parse(r#"{{ length . }}"#).is_ok());
        let data: Arc<Any> = Arc::new(serde_json::to_value("hello".to_owned()).unwrap());
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "5");
    }

    #[test]
    #[ignore]
    fn test_pipeline_function() {
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(t.parse(r#"{{ if ( 1 | eq . ) -}} 2000 {{- end }}"#).is_ok());
        let data: Arc<Any> = Arc::new(serde_json::to_value(1).unwrap());
        let out = t.execute(&mut w, data);
        println!("{:?}", out);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "2000");
    }

    #[test]
    fn test_function() {
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(t.parse(r#"{{ if eq . . -}} 2000 {{- end }}"#).is_ok());
        let data: Arc<Any> = Arc::new(serde_json::to_value(1).unwrap());
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "2000");
    }

    #[test]
    fn test_eq() {
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(t.parse(r#"{{ if eq "a" "a" -}} 2000 {{- end }}"#).is_ok());
        let data: Arc<Any> = Arc::new(serde_json::to_value(1).unwrap());
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "2000");

        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(t.parse(r#"{{ if eq "a" "b" -}} 2000 {{- end }}"#).is_ok());
        let data: Arc<Any> = Arc::new(serde_json::to_value(1).unwrap());
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "");

        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(t.parse(r#"{{ if eq true true -}} 2000 {{- end }}"#).is_ok());
        let data: Arc<Any> = Arc::new(serde_json::to_value(1).unwrap());
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "2000");

        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ if eq true false -}} 2000 {{- end }}"#)
                .is_ok()
        );
        let data: Arc<Any> = Arc::new(serde_json::to_value(1).unwrap());
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "");

        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(
            t.parse(r#"{{ if eq 23.42 23.42 -}} 2000 {{- end }}"#)
                .is_ok()
        );
        let data: Arc<Any> = Arc::new(serde_json::to_value(1).unwrap());
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "2000");

        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(t.parse(r#"{{ if eq 1 . -}} 2000 {{- end }}"#).is_ok());
        let data: Arc<Any> = Arc::new(1);
        let out = t.execute(&mut w, data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "2000");
    }
}
