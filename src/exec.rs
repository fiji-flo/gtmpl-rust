use std::any::Any;
use std::io::Write;
use std::collections::{HashMap, VecDeque};

use template::Template;
use node::*;

type Variable<'a> = (String, &'a Box<Any>);

enum Dot { Dot }


static MAX_EXEC_DEPTH: usize = 100000;

struct State<'a, 'c, 'b, T: Write>
    where T: 'b
{
    template: &'a Template<'a>,
    writer: &'b mut T,
    node: Option<&'a Nodes>,
    vars: VecDeque<Variable<'a>>,
    depth: usize,
    dot: &'c Box<Any>,
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
            .ok_or_else(|| format!("{} is an incomplete or empty template", self.name))?;
        state.walk(data, root)?;


        Ok(())
    }
}

impl<'a, 'b, 'c, T: Write> State<'a, 'b, 'c, T> {
    fn walk_list(&mut self, dot: &Box<Any>, node: &'a ListNode) -> Result<(), String> {
        for n in &node.nodes {
            self.walk(dot, n)?;
        }
        Ok(())
    }

    fn walk(&mut self, dot: &Box<Any>, node: &'a Nodes) -> Result<(), String> {
        self.node = Some(node);
        match *node {
            Nodes::Action(_) => {
                let val = self.eval_pipeline(dot, node);
                return Ok(());
            }
            Nodes::If(_) => {
                return self.walk_if_or_with(node, dot);
            }
            Nodes::List(ref n) => return self.walk_list(dot, n),
            Nodes::Text(ref n) => write!(self.writer, "{}", n).map_err(|e| format!("{}", e))?,
            _ => {}
            // TODO
        }
        Ok(())
    }

    fn eval_pipeline(&mut self, dot: &Box<Any>, node: &'a Nodes) -> Result<Box<Any>, String> {
        self.node = Some(node);
        let mut val: Option<Box<Any>> = None;
        if let &Nodes::Pipe(ref pipe) = node {
            val = Some(self.eval_pipeline_raw(dot, pipe)?);
        }
        val.ok_or_else(|| format!("error evaluating pipeline {}", node))
    }

    fn eval_pipeline_raw(&mut self,
                         dot: &Box<Any>,
                         pipe: &'a PipeNode)
                         -> Result<Box<Any>, String> {
        let mut val: Option<Box<Any>> = None;
        for cmd in &pipe.cmds {
            val = Some(self.eval_command(dot, cmd, val)?);
            // TODO
        }
        val.ok_or_else(|| format!("error evaluating pipeline {}", pipe))
    }

    fn eval_command(&mut self,
                    dot: &Box<Any>,
                    cmd: &CommandNode,
                    val: Option<Box<Any>>)
                    -> Result<Box<Any>, String> {
        let first_word = &cmd.args
                              .first()
                              .ok_or_else(|| format!("no arguments for command node: {}", cmd))?;

        match *first_word {
            &Nodes::Field(ref n) => return self.eval_field_node(dot, n, &cmd.args, val),
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

    fn eval_field_node(&mut self,
                       dot: &Box<Any>,
                       field: &FieldNode,
                       args: &[Nodes],
                       val: Option<Box<Any>>)
                       -> Result<Box<Any>, String> {

        Err(format!("DOOM"))
    }

    fn walk_if_or_with(&mut self, node: &'a Nodes, dot: &Box<Any>) -> Result<(), String> {
        let pipe = match *node {
            Nodes::If(ref n) => &n.pipe,
            Nodes::With(ref n) => &n.pipe,
            _ => return Err(format!("expected if or with node, got {}", node)),
        };
        let val = self.eval_pipeline_raw(dot, &pipe)?;
        let truth = is_true(&val);
        if truth.0 {
            match *node {
                Nodes::If(ref n) => return self.walk_list(dot, &n.list),
                Nodes::With(ref n) => return self.walk_list(&val, &n.list),
                _ => {}
            }
        } else {
            match *node {
                Nodes::If(ref n) => {
                    if let Some(ref otherwise) = n.else_list {
                        return self.walk_list(dot, otherwise);
                    }
                }
                Nodes::With(ref n) => {
                    if let Some(ref otherwise) = n.else_list {
                        return self.walk_list(dot, otherwise);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn is_true(&self, val: &Box<Any>) -> (bool, bool) {
        if let Some(_) = val.downcast_ref::<Dot>() {
            println!("yay");
            return is_true(self.dot);
        }
        return is_true(val);
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



    non_zero!(val -> u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);
    (true, true)
}

#[cfg(test)]
mod tests_mocked {
    use super::*;

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
        assert!(t.parse(r#"{{ if false -}} 2000 {{- else -}} 3000 {{- end }}"#)
                    .is_ok());
        let out = t.execute(&mut w, &data);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "3000");
    }

    #[test]
    fn basic_dot() {
        let data: Box<Any> = Box::new(1);
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(t.parse(r#"{{ if . -}} 2000 {{- else -}} 3000 {{- end }}"#)
                .is_ok());
        let out = t.execute(&mut w, &data);
        println!("{:?}", out);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "2000");

        let data: Box<Any> = Box::new(false);
        let mut w: Vec<u8> = vec![];
        let mut t = Template::new("foo");
        assert!(t.parse(r#"{{ if . -}} 2000 {{- else -}} 3000 {{- end }}"#)
                .is_ok());
        let out = t.execute(&mut w, &data);
        println!("{:?}", out);
        assert!(out.is_ok());
        assert_eq!(String::from_utf8(w).unwrap(), "3000");
    }
}
