use std::any::Any;
use std::io::Write;
use std::collections::VecDeque;

use template::Template;
use node::Nodes;

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
        state.walk(&data, root);


        Ok(())
    }
}

impl<'a, T: Write> State<'a, T> {
    fn walk(&mut self, dot: &'a Box<Any>, node: &'a Nodes) {
        self.node = Some(node);
        match *node {
            Nodes::Action(ref n) => {

            },
            _ => {},
        }
    }

}
