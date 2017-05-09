use std::any::Any;
use std::collections::HashMap;

use lexer::{Item, Lexer};
use node::*;

pub type Func<'a> = &'a Fn(Option<Box<Any>>) -> Option<Box<Any>>;

pub struct Tree<'a> {
    name: String,
    parse_name: String,
    root: ListNode,
    text: String,
    funcs: HashMap<String, Func<'a>>,
    lex: Option<Lexer>,
    token: Vec<Item>,
    peek_count: usize,
    vars: Vec<String>,
    tree_set: HashMap<String, TreeId>,
}

impl<'a, 'b> Tree<'a> {
    fn clone_new(t: &Tree) -> Tree<'b> {
        Tree {
            name: t.name.clone(),
            parse_name: t.parse_name.clone(),
            root: t.root.clone(),
            text: t.text.clone(),
            funcs: HashMap::new(),
            lex: None,
            token: vec![],
            peek_count: 0,
            vars: vec![],
            tree_set: HashMap::new(),
        }
    }
}

pub fn parse<'a>(name: String,
                 text: String,
                 funcs: HashMap<String, Func<'a>>)
                 -> HashMap<String, Tree<'a>> {
    let tree_set = HashMap::new();
    tree_set
}

impl<'a> Tree<'a> {
    fn next_from_lex(&mut self) -> Option<Item> {
        match &mut self.lex {
            &mut Some(ref mut l) => l.next(),
            &mut None => None,
        }
    }

    fn backup(&mut self, t: Item) {
        self.token.push(t);
        self.peek_count = 1;
    }

    fn backup2(&mut self, t0: Item, t1: Item) {
        self.token.push(t0);
        self.token.push(t1);
        self.peek_count = 2;
    }

    fn backup3(&mut self, t0: Item, t1: Item, t2: Item) {
        self.token.push(t0);
        self.token.push(t1);
        self.token.push(t2);
        self.peek_count = 3;
    }
}

impl<'a> Iterator for Tree<'a> {
    type Item = Item;
    fn next(&mut self) -> Option<Item> {
        if self.peek_count > 0 {
            self.peek_count -= 1;
            self.token.pop()
        } else {
            self.next_from_lex()
        }
    }
}

#[cfg(test)]
mod tests_mocked {
    use super::*;
    use lexer::ItemType;

    fn make_tree<'a>() -> Tree<'a> {
        let s = r#"something {{ if eq "foo" "bar" }}"#;
        let lex = Lexer::new("foo", s.to_owned());
        Tree {
            name: "foo".to_owned(),
            parse_name: "bar".to_owned(),
            root: ListNode::new(0, 0),
            text: "nope".to_owned(),
            funcs: HashMap::new(),
            lex: Some(lex),
            token: vec![],
            peek_count: 0,
            vars: vec![],
            tree_set: HashMap::new(),
        }
    }

    #[test]
    fn test_iter() {
        let mut t = make_tree();
        assert_eq!(t.next().and_then(|n| Some(n.typ)), Some(ItemType::ItemText));
        assert_eq!(t.collect::<Vec<_>>().len(), 12);
    }

    #[test]
    fn test_backup() {
        let mut t = make_tree();
        assert_eq!(t.next().and_then(|n| Some(n.typ)), Some(ItemType::ItemText));
        let i = t.next().unwrap();
        let s = i.to_string();
        t.backup(i);
        assert_eq!(t.next().and_then(|n| Some(n.to_string())), Some(s));
        assert_eq!(t.last().and_then(|n| Some(n.typ)), Some(ItemType::ItemEOF));
    }
}
