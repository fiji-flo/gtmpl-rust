use std::any::Any;
use std::collections::{HashMap, VecDeque};

use lexer::{Item, ItemType, Lexer};
use node::*;

pub type Func<'a> = &'a Fn(Option<Box<Any>>) -> Option<Box<Any>>;

pub struct Tree<'a> {
    name: String,
    parse_name: String,
    root: ListNode,
    text: String,
    funcs: HashMap<String, Func<'a>>,
    lex: Option<Lexer>,
    token: VecDeque<Item>,
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
            token: VecDeque::new(),
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
        self.token.push_back(t);
        self.peek_count = 1;
    }

    fn backup2(&mut self, t0: Item, t1: Item) {
        self.token.push_back(t0);
        self.token.push_back(t1);
        self.peek_count = 2;
    }

    fn backup3(&mut self, t0: Item, t1: Item, t2: Item) {
        self.token.push_back(t0);
        self.token.push_back(t1);
        self.token.push_back(t2);
        self.peek_count = 3;
    }

    fn next_non_space(&mut self) -> Option<Item> {
        self.skip_while(|c| c.typ == ItemType::ItemSpace).next()
    }

    fn peek_non_space(&mut self) -> Option<&Item> {
        if let Some(t) = self.next_non_space() {
            self.backup(t);
            return self.token.front();
        }
        None
    }
}

impl<'a> Iterator for Tree<'a> {
    type Item = Item;
    fn next(&mut self) -> Option<Item> {
        if self.peek_count > 0 {
            self.peek_count -= 1;
            self.token.pop_front()
        } else {
            self.next_from_lex()
        }
    }
}

#[cfg(test)]
mod tests_mocked {
    use super::*;
    use lexer::ItemType;

    /*
       ItemText
       ItemLeftDelim
       ItemSpace
       ItemIf
       ItemSpace
       ItemIdentifier
       ItemSpace
       ItemString
       ItemSpace
       ItemString
       ItemSpace
       ItemRightDelim
       ItemEOF
    */
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
            token: VecDeque::new(),
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
    #[test]
    fn test_backup3() {
        let mut t = make_tree();
        assert_eq!(t.next().and_then(|n| Some(n.typ)), Some(ItemType::ItemText));
        let t0 = t.next().unwrap();
        let t1 = t.next().unwrap();
        let t2 = t.next().unwrap();
        assert_eq!(t0.typ, ItemType::ItemLeftDelim);
        assert_eq!(t1.typ, ItemType::ItemSpace);
        assert_eq!(t2.typ, ItemType::ItemIf);
        t.backup3(t0, t1, t2);
        let t0 = t.next().unwrap();
        let t1 = t.next().unwrap();
        let t2 = t.next().unwrap();
        assert_eq!(t0.typ, ItemType::ItemLeftDelim);
        assert_eq!(t1.typ, ItemType::ItemSpace);
        assert_eq!(t2.typ, ItemType::ItemIf);
        assert_eq!(t.last().and_then(|n| Some(n.typ)), Some(ItemType::ItemEOF));
    }


    #[test]
    fn test_next_non_space() {
        let mut t = make_tree();
        t.next();
        let i = t.next().unwrap();
        let typ = i.typ;
        assert_eq!(typ, ItemType::ItemLeftDelim);
        assert_eq!(t.next_non_space().and_then(|n| Some(n.typ)),
                   Some(ItemType::ItemIf));
        assert_eq!(t.last().and_then(|n| Some(n.typ)), Some(ItemType::ItemEOF));
    }

    #[test]
    fn test_peek_non_space() {
        let mut t = make_tree();
        t.next();
        let i = t.next().unwrap();
        let typ = i.typ;
        assert_eq!(typ, ItemType::ItemLeftDelim);
        assert_eq!(t.peek_non_space().and_then(|n| Some(&n.typ)),
                   Some(&ItemType::ItemIf));
        assert_eq!(t.next().and_then(|n| Some(n.typ)), Some(ItemType::ItemIf));
        assert_eq!(t.last().and_then(|n| Some(n.typ)), Some(ItemType::ItemEOF));
    }
}
