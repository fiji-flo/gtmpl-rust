use std::any::Any;
use std::io::Write;
use std::collections::VecDeque;

use template::Template;
use node::*;

type Variable<'a> = (String, &'a Box<Any>);

static MAX_EXEC_DEPTH: usize = 100000;

struct State<'a, T: Write> {
    template: &'a Template<'a>,
    writer: T,
    node: Option<&'a Nodes>,
    vars: VecDeque<Variable<'a>>,
    depth: usize,
}

impl<'a> Template<'a> {
    fn execute<T: Write>(&mut self, writer: T, data: Box<Any>) -> Result<(), String> {
        let mut vars = VecDeque::new();
        vars.push_back(("$".to_owned(), &data));

        let mut state = State {
            template: &self,
            writer,
            node: None,
            vars,
            depth: 0,
        };

        let root = self.tree_ids
            .get(&0usize)
            .and_then(|name| self.tree_set.get(name))
            .and_then(|tree| tree.root.as_ref())
            .ok_or_else(|| format!("{} is an incomplete or empty template", self.name))?;
        state.walk(&data, root)?;


        Ok(())
    }
}

impl<'a, T: Write> State<'a, T> {
    fn walk_list(&mut self, dot: &Box<Any>, node: &'a ListNode) -> Result<(), String>{
        for n in &node.nodes {
            self.walk(dot, n)?;
        }
        Ok(())
    }

    fn walk(&mut self, dot: &Box<Any>, node: &'a Nodes) -> Result<(), String>{
        self.node = Some(node);
        match *node {
            Nodes::Action(_) => {
                let val = self.eval_pipeline(dot, node);
                return Ok(())
            },
            Nodes::If(_) => {
                return self.walk_if_or_with(node, dot);
            }
            Nodes::List(ref n) => return self.walk_list(dot, n),
            _ => {}
            // TODO
        }
        Ok(())
    }

    fn eval_pipeline(&mut self, dot: &Box<Any>, node: &'a Nodes) -> Result<Box<Any>, String> {
        self.node = Some(node);
        let mut val: Option<Box<Any>> = None;
        if let &Nodes::Pipe(ref pipe) = node {
            let val = Some(self.eval_pipeline_raw(dot, pipe)?);
        }
        Ok(Box::new(val))
    }

    fn eval_pipeline_raw(&mut self, dot: &Box<Any>, pipe: &'a PipeNode) -> Result<Box<Any>, String> {
        let mut val: Option<Box<Any>> = None;
        for cmd in &pipe.cmds {
            val = Some(self.eval_command(dot, cmd, val)?);
            // TODO
        }
        Ok(Box::new(val))
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
            _ => {}
        }


        Err(format!("DOOM"))
    }

    fn eval_field_node(&mut self, dot: &Box<Any>, field: &FieldNode, args: &[Nodes], val: Option<Box<Any>>) -> Result<Box<Any>, String> {

        Err(format!("DOOM"))
    }

    fn walk_if_or_with(&mut self, node: &'a Nodes, dot: &Box<Any>) -> Result<(), String> {
        let pipe = match *node {
            Nodes::If(ref n) => &n.pipe,
            Nodes::With(ref n) => &n.pipe,
            _ => return Err(format!("expected if or with node, got {}", node)),
        };
        let val = self.eval_pipeline_raw(dot, &pipe)?;
        let truth = true;
        if true {
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
                },
                Nodes::With(ref n) => {
                    if let Some(ref otherwise) = n.else_list {
                        return self.walk_list(dot, otherwise);
                    }
                }
                _ => {}
            }
        }
        Err(format!("DOOM"))
    }
}

fn not_a_function(args: &[Nodes], val: Option<Box<Any>>) -> Result<(), String> {
    if args.len() > 1 || val.is_some() {
        return Err(format!("can't give arument to non-function {}", args[0]));
    }
    Ok(())
}
