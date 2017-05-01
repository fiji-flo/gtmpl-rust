#[derive(Clone)]
pub enum NodeType {
    ListNode,
    TextNode,
    CommandNode,
    IdentifierNode,
    VariableNode,
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
}

pub trait Node {
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
            $($field: $typ)*,
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

node!(
    VariableNode {
        ident: Vec<String>
    }
);

impl VariableNode {
    fn new(pos: Pos, ident: String) -> VariableNode {
        VariableNode {
            typ: NodeType::VariableNode,
            tr: 0,
            pos,
            ident: ident.split(".").map(|s| s.to_owned()).collect(),
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
        assert!(t2.text != t1.text);
    }
}
