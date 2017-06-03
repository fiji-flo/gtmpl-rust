use std::any::Any;
use std::io::{Write,BufWriter};
use std::collections::VecDeque;

use template::Template;
use node::Nodes;

static MAX_EXEC_DEPTH: usize = 100000;

struct State<'a, T: Write> {
    template: Template<'a>,
    writer: BufWriter<T>,
    node: Option<Nodes>,
    vars: VecDeque<(String, Box<Any>)>,
    depth: usize,
}
