use std::fmt::{Display, Formatter};

use crate::error::NodeError;
use crate::lexer::ItemType;
use crate::utils::unquote_char;

use gtmpl_value::Value;

macro_rules! nodes {
    ($($node:ident, $name:ident),*) => {
        #[derive(Debug)]
        #[derive(Clone)]
        #[derive(PartialEq)]
        pub enum NodeType {
           $($name,)*
        }

        #[derive(Clone)]
        #[derive(Debug)]
        pub enum Nodes {
            $($name($node),)*
        }

        impl Display for Nodes {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
                match *self {
                    $(Nodes::$name(ref t) => t.fmt(f),)*
                }
            }
        }

        impl Nodes {
            pub fn typ(&self) -> &NodeType {
                match *self {
                    $(Nodes::$name(ref t) => t.typ(),)*
                }
            }
            pub fn pos(&self) -> Pos {
                match *self {
                    $(Nodes::$name(ref t) => t.pos(),)*
                }
            }
            pub fn tree(&self) -> TreeId {
                match *self {
                    $(Nodes::$name(ref t) => t.tree(),)*
                }
            }
        }
    }
}

nodes!(
    ListNode,
    List,
    TextNode,
    Text,
    PipeNode,
    Pipe,
    ActionNode,
    Action,
    CommandNode,
    Command,
    IdentifierNode,
    Identifier,
    VariableNode,
    Variable,
    DotNode,
    Dot,
    NilNode,
    Nil,
    FieldNode,
    Field,
    ChainNode,
    Chain,
    BoolNode,
    Bool,
    NumberNode,
    Number,
    StringNode,
    String,
    EndNode,
    End,
    ElseNode,
    Else,
    IfNode,
    If,
    WithNode,
    With,
    RangeNode,
    Range,
    TemplateNode,
    Template
);

pub type Pos = usize;

pub type TreeId = usize;

pub trait Node: Display {
    fn typ(&self) -> &NodeType;
    fn pos(&self) -> Pos;
    fn tree(&self) -> TreeId;
}

macro_rules! node {
    ($name:ident {
        $($field:ident : $typ:ty),* $(,)*
    }) => {
        #[derive(Clone)]
        #[derive(Debug)]
        pub struct $name {
            typ: NodeType,
            pos: Pos,
            tr: TreeId,
            $(pub $field: $typ,)*
        }
        impl Node for $name {
            fn typ(&self) -> &NodeType {
                &self.typ
            }
            fn pos(&self) -> Pos {
                self.pos
            }
            fn tree(&self) -> TreeId {
                self.tr
            }
        }
    }
}

impl Nodes {
    pub fn is_empty_tree(&self) -> Result<bool, NodeError> {
        match *self {
            Nodes::List(ref n) => n.is_empty_tree(),
            Nodes::Text(ref n) => Ok(n.text.is_empty()),
            Nodes::Action(_)
            | Nodes::If(_)
            | Nodes::Range(_)
            | Nodes::Template(_)
            | Nodes::With(_) => Ok(false),
            _ => Err(NodeError::NaTN),
        }
    }
}

node!(
    ListNode {
        nodes: Vec<Nodes>
    }
);

impl ListNode {
    pub fn append(&mut self, n: Nodes) {
        self.nodes.push(n);
    }
    pub fn new(tr: TreeId, pos: Pos) -> ListNode {
        ListNode {
            typ: NodeType::List,
            pos,
            tr,
            nodes: vec![],
        }
    }
    pub fn is_empty_tree(&self) -> Result<bool, NodeError> {
        for n in &self.nodes {
            match n.is_empty_tree() {
                Ok(true) => {}
                Ok(false) => return Ok(false),
                Err(s) => return Err(s),
            }
        }
        Ok(true)
    }
}

impl Display for ListNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for n in &self.nodes {
            if let Err(e) = n.fmt(f) {
                return Err(e);
            }
        }
        Ok(())
    }
}

node!(TextNode { text: String });

impl TextNode {
    pub fn new(tr: TreeId, pos: Pos, text: String) -> TextNode {
        TextNode {
            typ: NodeType::Text,
            pos,
            tr,
            text,
        }
    }
}

impl Display for TextNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.text)
    }
}

node!(
    PipeNode {
        decl: Vec<VariableNode>,
        cmds: Vec<CommandNode>
    }
);

impl PipeNode {
    pub fn new(tr: TreeId, pos: Pos, decl: Vec<VariableNode>) -> PipeNode {
        PipeNode {
            typ: NodeType::Pipe,
            tr,
            pos,
            decl,
            cmds: vec![],
        }
    }

    pub fn append(&mut self, cmd: CommandNode) {
        self.cmds.push(cmd);
    }
}

impl Display for PipeNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let decl = if self.decl.is_empty() {
            Ok(())
        } else {
            write!(
                f,
                "{} := ",
                self.decl
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            )
        };
        decl.and_then(|_| {
            write!(
                f,
                "{}",
                self.cmds
                    .iter()
                    .map(|cmd| cmd.to_string())
                    .collect::<Vec<String>>()
                    .join(" | ")
            )
        })
    }
}

node!(ActionNode { pipe: PipeNode });

impl ActionNode {
    pub fn new(tr: TreeId, pos: Pos, pipe: PipeNode) -> ActionNode {
        ActionNode {
            typ: NodeType::Action,
            tr,
            pos,
            pipe,
        }
    }
}

impl Display for ActionNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{{{{{}}}}}", self.pipe)
    }
}

node!(
    CommandNode {
        args: Vec<Nodes>
    }
);

impl CommandNode {
    pub fn new(tr: TreeId, pos: Pos) -> CommandNode {
        CommandNode {
            typ: NodeType::Command,
            pos,
            tr,
            args: vec![],
        }
    }

    pub fn append(&mut self, node: Nodes) {
        self.args.push(node);
    }
}

impl Display for CommandNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let s = self
            .args
            .iter()
            .map(|n|
                // Handle PipeNode.
                n.to_string())
            .collect::<Vec<String>>()
            .join(" ");
        write!(f, "{}", s)
    }
}

node!(IdentifierNode { ident: String });

impl IdentifierNode {
    pub fn new(ident: String) -> IdentifierNode {
        IdentifierNode {
            typ: NodeType::Identifier,
            tr: 0,
            pos: 0,
            ident,
        }
    }

    pub fn set_pos(&mut self, pos: Pos) -> &IdentifierNode {
        self.pos = pos;
        self
    }

    pub fn set_tree(&mut self, tr: TreeId) -> &IdentifierNode {
        self.tr = tr;
        self
    }
}

impl Display for IdentifierNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.ident)
    }
}

node!(
    VariableNode {
        ident: Vec<String>
    }
);

impl VariableNode {
    pub fn new(tr: TreeId, pos: Pos, ident: &str) -> VariableNode {
        VariableNode {
            typ: NodeType::Variable,
            tr,
            pos,
            ident: ident.split('.').map(|s| s.to_owned()).collect(),
        }
    }
}

impl Display for VariableNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.ident.join("."))
    }
}

node!(DotNode {});

impl DotNode {
    pub fn new(tr: TreeId, pos: Pos) -> DotNode {
        DotNode {
            typ: NodeType::Dot,
            tr,
            pos,
        }
    }
}

impl Display for DotNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, ".")
    }
}

node!(NilNode {});

impl Display for NilNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "nil")
    }
}

impl NilNode {
    pub fn new(tr: TreeId, pos: Pos) -> NilNode {
        NilNode {
            typ: NodeType::Nil,
            tr,
            pos,
        }
    }
}

node!(
    FieldNode {
        ident: Vec<String>
    }
);

impl FieldNode {
    pub fn new(tr: TreeId, pos: Pos, ident: &str) -> FieldNode {
        FieldNode {
            typ: NodeType::Field,
            tr,
            pos,
            ident: ident[..]
                .split('.')
                .filter_map(|s| {
                    if s.is_empty() {
                        None
                    } else {
                        Some(s.to_owned())
                    }
                })
                .collect(),
        }
    }
}

impl Display for FieldNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.ident.join("."))
    }
}

node!(
    ChainNode {
        node: Box<Nodes>,
        field: Vec<String>
    }
);

impl ChainNode {
    pub fn new(tr: TreeId, pos: Pos, node: Nodes) -> ChainNode {
        ChainNode {
            typ: NodeType::Chain,
            tr,
            pos,
            node: Box::new(node),
            field: vec![],
        }
    }

    pub fn add(&mut self, val: &str) {
        let val = val.trim_start_matches('.').to_owned();
        self.field.push(val);
    }
}

impl Display for ChainNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        if let Err(e) = {
            // Handle PipeNode.
            write!(f, "{}", self.node)
        } {
            return Err(e);
        }
        for field in &self.field {
            if let Err(e) = write!(f, ".{}", field) {
                return Err(e);
            }
        }
        Ok(())
    }
}

node!(BoolNode { value: Value });

impl BoolNode {
    pub fn new(tr: TreeId, pos: Pos, val: bool) -> BoolNode {
        BoolNode {
            typ: NodeType::Bool,
            tr,
            pos,
            value: Value::from(val),
        }
    }
}

impl Display for BoolNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.value)
    }
}

#[derive(Clone, Debug)]
pub enum NumberType {
    U64,
    I64,
    Float,
    Char,
}

node!(NumberNode {
    is_i64: bool,
    is_u64: bool,
    is_f64: bool,
    text: String,
    number_typ: NumberType,
    value: Value,
});

impl NumberNode {
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::float_cmp))]
    pub fn new(
        tr: TreeId,
        pos: Pos,
        text: String,
        item_typ: &ItemType,
    ) -> Result<NumberNode, NodeError> {
        match *item_typ {
            ItemType::ItemCharConstant => unquote_char(&text, '\'')
                .map(|c| NumberNode {
                    typ: NodeType::Number,
                    tr,
                    pos,
                    is_i64: true,
                    is_u64: true,
                    is_f64: true,
                    text,
                    number_typ: NumberType::Char,
                    value: Value::from(c as u64),
                })
                .ok_or(NodeError::UnquoteError),
            _ => {
                let mut number_typ = NumberType::Float;

                // TODO: Deal with hex.
                let (mut as_i64, mut is_i64) = text
                    .parse::<i64>()
                    .map(|i| (i, true))
                    .unwrap_or((0i64, false));

                if is_i64 {
                    number_typ = NumberType::I64;
                }

                let (mut as_u64, mut is_u64) = text
                    .parse::<u64>()
                    .map(|i| (i, true))
                    .unwrap_or((0u64, false));

                if is_u64 {
                    number_typ = NumberType::U64;
                }

                if is_i64 && as_i64 == 0 {
                    // In case of -0.
                    as_u64 = 0;
                    is_u64 = true;
                }

                let (as_f64, is_f64) = match text.parse::<f64>() {
                    Err(_) => (0.0_f64, false),
                    Ok(f) => {
                        let frac = text.contains(|c| {
                            matches! {
                            c, '.' | 'e' | 'E' }
                        });
                        if frac {
                            (f, true)
                        } else {
                            (f, false)
                        }
                    }
                };
                if !is_i64 && ((as_f64 as i64) as f64) == as_f64 {
                    as_i64 = as_f64 as i64;
                    is_i64 = true;
                }
                if !is_u64 && ((as_f64 as u64) as f64) == as_f64 {
                    as_u64 = as_f64 as u64;
                    is_u64 = true;
                }
                if !is_u64 && !is_i64 && !is_f64 {
                    return Err(NodeError::NaN);
                }

                let value = if is_u64 {
                    Value::from(as_u64)
                } else if is_i64 {
                    Value::from(as_i64)
                } else {
                    Value::from(as_f64)
                };

                Ok(NumberNode {
                    typ: NodeType::Number,
                    tr,
                    pos,
                    is_i64,
                    is_u64,
                    is_f64,
                    text,
                    number_typ,
                    value,
                })
            }
        }
    }
}

impl Display for NumberNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.text)
    }
}

node!(StringNode {
    quoted: String,
    value: Value,
});

impl StringNode {
    pub fn new(tr: TreeId, pos: Pos, orig: String, text: String) -> StringNode {
        StringNode {
            typ: NodeType::String,
            tr,
            pos,
            quoted: orig,
            value: Value::from(text),
        }
    }
}

impl Display for StringNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.quoted)
    }
}

node!(EndNode {});

impl EndNode {
    pub fn new(tr: TreeId, pos: Pos) -> EndNode {
        EndNode {
            typ: NodeType::End,
            tr,
            pos,
        }
    }
}

impl Display for EndNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{{{{end}}}}")
    }
}

node!(ElseNode {});

impl ElseNode {
    pub fn new(tr: TreeId, pos: Pos) -> ElseNode {
        ElseNode {
            typ: NodeType::Else,
            tr,
            pos,
        }
    }
}

impl Display for ElseNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{{{{else}}}}")
    }
}

node!(
    BranchNode {
        pipe: PipeNode,
        list: ListNode,
        else_list: Option<ListNode>
    }
);

pub type IfNode = BranchNode;
pub type WithNode = BranchNode;
pub type RangeNode = BranchNode;

impl BranchNode {
    pub fn new_if(
        tr: TreeId,
        pos: Pos,
        pipe: PipeNode,
        list: ListNode,
        else_list: Option<ListNode>,
    ) -> IfNode {
        IfNode {
            typ: NodeType::If,
            tr,
            pos,
            pipe,
            list,
            else_list,
        }
    }

    pub fn new_with(
        tr: TreeId,
        pos: Pos,
        pipe: PipeNode,
        list: ListNode,
        else_list: Option<ListNode>,
    ) -> WithNode {
        WithNode {
            typ: NodeType::With,
            tr,
            pos,
            pipe,
            list,
            else_list,
        }
    }

    pub fn new_range(
        tr: TreeId,
        pos: Pos,
        pipe: PipeNode,
        list: ListNode,
        else_list: Option<ListNode>,
    ) -> RangeNode {
        RangeNode {
            typ: NodeType::Range,
            tr,
            pos,
            pipe,
            list,
            else_list,
        }
    }
}

impl Display for BranchNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let name = match self.typ {
            NodeType::If => "if",
            NodeType::Range => "range",
            NodeType::With => "with",
            _ => {
                return Err(std::fmt::Error);
            }
        };
        if let Some(ref else_list) = self.else_list {
            return write!(
                f,
                "{{{{{} {}}}}}{}{{{{else}}}}{}{{{{end}}}}",
                name, self.pipe, self.list, else_list
            );
        }
        write!(f, "{{{{{} {}}}}}{}{{{{end}}}}", name, self.pipe, self.list)
    }
}

node!(
    TemplateNode {
        name: PipeOrString,
        pipe: Option<PipeNode>
    }
);

impl TemplateNode {
    pub fn new(tr: TreeId, pos: Pos, name: PipeOrString, pipe: Option<PipeNode>) -> TemplateNode {
        TemplateNode {
            typ: NodeType::Template,
            tr,
            pos,
            name,
            pipe,
        }
    }
}

impl Display for TemplateNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self.pipe {
            Some(ref pipe) => write!(f, "{{{{template {} {}}}}}", self.name, pipe),
            None => write!(f, "{{{{template {}}}}}", self.name),
        }
    }
}

#[derive(Clone, Debug)]
pub enum PipeOrString {
    Pipe(PipeNode),
    String(String),
}

impl Display for PipeOrString {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match *self {
            PipeOrString::Pipe(ref pipe_node) => write!(f, "{}", pipe_node),
            PipeOrString::String(ref s) => write!(f, "{}", s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clone() {
        let t1 = TextNode::new(1, 0, "foo".to_owned());
        let mut t2 = t1.clone();
        t2.text = "bar".to_owned();
        assert_eq!(t1.to_string(), "foo");
        assert_eq!(t2.to_string(), "bar");
    }

    #[test]
    fn test_end() {
        let t1 = EndNode::new(1, 0);
        assert_eq!(t1.to_string(), "{{end}}");
    }
}
