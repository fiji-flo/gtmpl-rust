use std::fmt::{Display, Error, Formatter};
use itertools::Itertools;

#[derive(Clone)]
enum Num {
    Int(i64),
    Uint(u64),
    Float(f64),
}

#[derive(Clone)]
pub enum NodeType {
    ListNode,
    TextNode,
    CommandNode,
    IdentifierNode,
    VariableNode,
    DotNode,
    NilNode,
    FieldNode,
    ChainNode,
    BoolNode,
    NumberNode,
}

type Pos = usize;

type TreeId = usize;

#[derive(Clone)]
enum Nodes {
    ListNode(ListNode),
    TextNode(TextNode),
    CommandNode(CommandNode),
    IdentifierNode(IdentifierNode),
    VariableNode(VariableNode),
    DotNode(DotNode),
    NilNode(NilNode),
    FieldNode(FieldNode),
    ChainNode(ChainNode),
    BoolNode(BoolNode),
    NumberNode(NumberNode),
}

impl Display for Nodes {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            &Nodes::TextNode(ref t) => t.fmt(f),
            _=> Ok(())
        }
    }
}

pub trait Node: Display {
    fn typ(&self) -> &NodeType;
    fn pos(&self) -> Pos;
    fn tree(&self) -> TreeId;
}

macro_rules! node {
    ($name:ident {
        $($field:ident : $typ:ty),*
    }) => {
        #[derive(Clone)]
        struct $name {
            typ: NodeType,
            pos: Pos,
            tr: TreeId,
            $($field: $typ,)*
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

node!(
    ListNode {
        nodes: Vec<Nodes>
    }
);

impl ListNode {
    fn append(&mut self, n: Nodes) {
        self.nodes.push(n);
    }
    fn new(tr: TreeId, pos: Pos) -> ListNode {
        ListNode {
            typ: NodeType::ListNode,
            pos,
            tr,
            nodes: vec![],
        }
    }
}

impl Display for ListNode {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        for n in &self.nodes {
            if let Err(e) = n.fmt(f) {
                return Err(e)
            }
        }
        Ok(())
    }
}

node!(
    TextNode {
        text: String
    }
);

impl TextNode {
    fn new(tr: TreeId, pos: Pos, text: String) -> TextNode {
        TextNode {
            typ: NodeType::ListNode,
            pos,
            tr,
            text,
        }
    }
}

impl Display for TextNode {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{}", self.text)
    }
}

node!(
    CommandNode {
        args: Vec<Nodes>
    }
);

impl CommandNode {
    fn new(tr: TreeId, pos: Pos) -> CommandNode {
        CommandNode {
            typ: NodeType::CommandNode,
            pos,
            tr,
            args: vec![],
        }
    }
}

impl Display for CommandNode {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let s = self.args.iter().map(|n| {
            match n {
                // Handle PipeNode.
                _ => n.to_string()
            }
        }).join(" ");
        write!(f, "{}", s)
    }
}

node!(
    IdentifierNode {
        ident: String
    }
);

impl IdentifierNode {
    fn new(ident: String) -> IdentifierNode {
        IdentifierNode {
            typ: NodeType::IdentifierNode,
            tr: 0,
            pos: 0,
            ident,
        }
    }

    fn set_pos(&mut self, pos: Pos) -> &IdentifierNode {
        self.pos = pos;
        self
    }

    fn set_tree(&mut self, tr: TreeId) -> &IdentifierNode {
        self.tr = tr;
        self
    }
}

impl Display for IdentifierNode {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{}", self.ident)
    }
}

node!(
    VariableNode {
        ident: Vec<String>
    }
);

impl VariableNode {
    fn new(tr: TreeId, pos: Pos, ident: String) -> VariableNode {
        VariableNode {
            typ: NodeType::VariableNode,
            tr,
            pos,
            ident: ident.split(".").map(|s| s.to_owned()).collect(),
        }
    }
}

impl Display for VariableNode {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{}", self.ident.join("."))
    }
}

node!(
    DotNode {}
);

impl DotNode {
    fn new(tr: TreeId, pos: Pos) -> DotNode {
        DotNode {
            typ: NodeType::DotNode,
            tr,
            pos,
        }
    }
}

impl Display for DotNode {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, ".")
    }
}

node!(
    NilNode {}
);

impl Display for NilNode {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "nil")
    }
}

impl NilNode {
    fn new(tr: TreeId, pos: Pos) -> NilNode {
        NilNode {
            typ: NodeType::NilNode,
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
    fn new(tr: TreeId, pos: Pos, ident: String) -> FieldNode {
        FieldNode {
            typ: NodeType::FieldNode,
            tr,
            pos,
            ident: ident[1..].split(".").map(|s| s.to_owned()).collect(),
        }
    }
}

impl Display for FieldNode {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
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
    fn new(tr: TreeId, pos: Pos, node: Nodes) -> ChainNode {
        ChainNode {
            typ: NodeType::ChainNode,
            tr,
            pos,
            node: Box::new(node),
            field: vec!(),
        }
    }
}

impl Display for ChainNode {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        if let Err(e) = match self.node {
            // Handle PipeNode.
            _ => write!(f, "{}", self.node)
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

node!(
    BoolNode {
        val: bool
    }
);

impl BoolNode {
    fn new(tr: TreeId, pos: Pos, val: bool) -> BoolNode {
        BoolNode {
            typ: NodeType::BoolNode,
            tr,
            pos,
            val,
        }
    }
}

impl Display for BoolNode {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{}", self.val)
    }
}

node!(
    NumberNode {
        num: Num,
        text: String
    }
);

impl NumberNode {
    fn new(tr: TreeId, pos: Pos, text: String) -> NumberNode {
        NumberNode {
            typ: NodeType::NumberNode,
            tr,
            pos,
            num: Num::Int(0),
            text,
        }
    }
}

impl Display for NumberNode {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "{}", self.text)
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
}
