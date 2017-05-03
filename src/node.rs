use std::fmt::{Display, Error, Formatter};
use itertools::Itertools;
use lexer::ItemType;
use utils::unquote_char;

macro_rules! nodes {
    ($($node:ident, $name:ident),*) => {
        #[derive(Clone)]
        pub enum NodeType {
           $($name,)*
        }

        #[derive(Clone)]
        enum Nodes {
            $($name($node),)*
        }

        impl Display for Nodes {
            fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
                match *self {
                    $(Nodes::$name(ref t) => t.fmt(f),)*
                }
            }
        }
    }
}

nodes!(ListNode, List,
       TextNode, Text,
       CommandNode, Command,
       IdentifierNode, Identifier,
       VariableNode, Variable,
       DotNode, Dot,
       NilNode, Nil,
       FieldNode, Field,
       ChainNode, Chain,
       BoolNode, Bool,
       NumberNode, Number);

type Pos = usize;

type TreeId = usize;


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
            typ: NodeType::List,
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
                return Err(e);
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
            typ: NodeType::List,
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
            typ: NodeType::Command,
            pos,
            tr,
            args: vec![],
        }
    }
}

impl Display for CommandNode {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let s = self.args
            .iter()
            .map(|n| {
                     match n {
                         // Handle PipeNode.
                         _ => n.to_string(),
                     }
                 })
            .join(" ");
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
            typ: NodeType::Identifier,
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
            typ: NodeType::Variable,
            tr,
            pos,
            ident: ident.split('.').map(|s| s.to_owned()).collect(),
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
            typ: NodeType::Dot,
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
    fn new(tr: TreeId, pos: Pos, ident: String) -> FieldNode {
        FieldNode {
            typ: NodeType::Field,
            tr,
            pos,
            ident: ident[1..].split('.').map(|s| s.to_owned()).collect(),
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
            typ: NodeType::Chain,
            tr,
            pos,
            node: Box::new(node),
            field: vec![],
        }
    }
}

impl Display for ChainNode {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        if let Err(e) = match self.node {
               // Handle PipeNode.
               _ => write!(f, "{}", self.node),
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
            typ: NodeType::Bool,
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
        is_i64: bool,
        is_u64: bool,
        is_f64: bool,
        as_i64: i64,
        as_u64: u64,
        as_f64: f64,
        text: String
    }
);

impl NumberNode {
    #[cfg_attr(feature = "cargo-clippy", allow(float_cmp))]
    fn new(tr: TreeId, pos: Pos, text: String, item_typ: ItemType) -> Result<NumberNode, Error> {
        match item_typ {
            ItemType::ItemCharConstant => {
                unquote_char(&text, '\'')
                    .and_then(|c| {
                        Some(NumberNode {
                                 typ: NodeType::Number,
                                 tr,
                                 pos,
                                 is_i64: true,
                                 is_u64: true,
                                 is_f64: true,
                                 as_i64: c as i64,
                                 as_u64: c as u64,
                                 as_f64: (c as i64) as f64,
                                 text,
                             })
                    })
                    .ok_or(Error)
            }
            _ => {
                // TODO: Deal with hex.
                let (mut as_u64, mut is_u64) = text.parse::<u64>()
                    .and_then(|i| Ok((i, true)))
                    .unwrap_or((0u64, false));

                let (mut as_i64, mut is_i64) = text.parse::<i64>()
                    .and_then(|i| Ok((i, true)))
                    .unwrap_or((0i64, false));

                if as_i64 == 0 {
                    // In case of -0.
                    as_u64 = 0;
                    is_u64 = true;
                }

                let (as_f64, is_f64) = if is_u64 {
                    (as_u64 as f64, true)
                } else if is_i64 {
                    (as_i64 as f64, true)
                } else {
                    match text.parse::<f64>() {
                        Err(_) => (0.0 as f64, false),
                        Ok(f) => {
                            if !text.contains(|c| match c {
                                                  '.' | 'e' | 'E' => true,
                                                  _ => false,
                                              }) {
                                return Err(Error);
                            }
                            (f, true)
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
                    return Err(Error);
                }
                Ok(NumberNode {
                       typ: NodeType::Number,
                       tr,
                       pos,
                       is_i64,
                       is_u64,
                       is_f64,
                       as_i64,
                       as_u64,
                       as_f64,
                       text,
                   })
            }
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
