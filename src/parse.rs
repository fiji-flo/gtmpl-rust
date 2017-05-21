use std::any::Any;
use std::collections::{HashMap, VecDeque};

use lexer::{Item, ItemType, Lexer};
use node::*;

pub type Func<'a> = &'a Fn(Option<Box<Any>>) -> Option<Box<Any>>;

pub struct Tree<'a> {
    name: String,
    id: TreeId,
    parse_name: String,
    root: Option<ListNode>,
    text: String,
    funcs: HashMap<String, Func<'a>>,
    lex: Option<Lexer>,
    token: VecDeque<Item>,
    peek_count: usize,
    vars: Vec<String>,
    tree_ids: HashMap<TreeId, String>,
    tree_set: HashMap<String, Tree<'a>>,
    line: usize,
}

impl<'a, 'b> Tree<'a> {
    fn new(name: String, id: TreeId, funcs: HashMap<String, Func<'a>>) -> Tree<'a> {
        Tree {
            name,
            id,
            parse_name: String::default(),
            root: None,
            text: String::default(),
            funcs,
            lex: None,
            token: VecDeque::new(),
            peek_count: 0,
            vars: vec![],
            tree_ids: HashMap::new(),
            tree_set: HashMap::new(),
            line: 0,
        }
    }
    fn clone_new(t: &Tree) -> Tree<'b> {
        Tree {
            name: t.name.clone(),
            id: t.id.clone(),
            parse_name: t.parse_name.clone(),
            root: t.root.clone(),
            text: t.text.clone(),
            funcs: HashMap::new(),
            lex: None,
            token: VecDeque::new(),
            peek_count: 0,
            vars: vec![],
            tree_ids: HashMap::new(),
            tree_set: HashMap::new(),
            line: 0,
        }
    }
}

pub fn parse<'a>(name: String,
                 text: String,
                 funcs: HashMap<String, Func<'a>>)
                 -> HashMap<String, Tree<'a>> {
    let tree_ids = HashMap::new();
    tree_ids
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

    fn error_context(&mut self, n: Nodes) -> (String, String) {
        let pos = n.pos();
        let tree_id = n.tree();
        let tree = if tree_id == 0 && self.tree_ids.contains_key(&tree_id) {
            self.tree_by_id(tree_id).unwrap()
        } else {
            self
        };
        let text = &tree.text[0..pos];
        let byte_num = match text.rfind('\n') {
            Some(i) => pos - (i + 1),
            None => pos,
        };
        let line_num = text.chars().filter(|c| *c == '\n').count();
        let context = n.to_string();
        (format!("{}:{}:{}", tree.parse_name, line_num, byte_num), context)
    }

    fn start_parse(&mut self,
                   funcs: HashMap<String, Func<'a>>,
                   lex: Lexer,
                   tree_ids: HashMap<TreeId, String>,
                   tree_set: HashMap<String, Tree<'a>>) {
        self.root = None;
        self.lex = Some(lex);
        self.vars = vec!["$".to_owned()];
        self.funcs = funcs;
        self.tree_ids = tree_ids;
        self.tree_set = tree_set;
    }

    fn stop_parse(&mut self) {
        self.lex = None;
        self.vars = vec![];
        self.funcs = HashMap::new();
        self.tree_ids = HashMap::new();
        self.tree_set = HashMap::new();
    }

    fn parse(&mut self,
             text: String,
             tree_ids: HashMap<TreeId, String>,
             tree_set: HashMap<String, Tree<'a>>,
             funcs: HashMap<String, Func<'a>>)
             -> Result<(), String> {
        self.parse_name = self.name.clone();
        let lex_name = self.name.clone();
        self.start_parse(funcs,
                         Lexer::new(&lex_name, text.clone()),
                         tree_ids,
                         tree_set);
        self.text = text;
        //self.do_parse();
        self.stop_parse();
        Ok(())
    }

    fn tree_by_id(&self, id: TreeId) -> Option<&Tree<'a>> {
        self.tree_ids
            .get(&id)
            .and_then(|name| self.tree_set.get(name))
    }
    fn add_tree(&mut self, name: &str, t: Tree<'a>) {
        self.tree_ids.insert(t.id, name.to_owned());
        self.tree_set.insert(name.to_owned(), t);
    }

    fn error(&mut self, msg: String) -> Result<(), String> {
        self.root = None;
        Err(format!("template: {}:{}:{}", self.parse_name, self.line, msg))
    }

    fn add_to_tree_set(mut tree: Tree<'a>,
                       mut tree_set: HashMap<String, Tree<'a>>)
                       -> Result<(), String> {
        if let Some(t) = tree_set.get(&tree.name) {
            if let Some(ref r) = t.root {
                match r.is_empty_tree() {
                    Err(e) => return Err(e),
                    Ok(false) => {
                        let err = format!("template multiple definitions of template {}",
                                          &tree.name);
                        return tree.error(err);
                    }
                    Ok(true) => {}
                }
            }
        }
        tree_set.insert(tree.name.clone(), tree);
        Ok(())
    }
}

impl<'a> Iterator for Tree<'a> {
    type Item = Item;
    fn next(&mut self) -> Option<Item> {
        let item = if self.peek_count > 0 {
            self.peek_count -= 1;
            self.token.pop_front()
        } else {
            self.next_from_lex()
        };
        match item {
            Some(item) => {
                self.line = item.line;
                Some(item)
            },
            _ => None
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
            id: 1,
            parse_name: "bar".to_owned(),
            root: None,
            text: "nope".to_owned(),
            funcs: HashMap::new(),
            lex: Some(lex),
            token: VecDeque::new(),
            peek_count: 0,
            vars: vec![],
            tree_ids: HashMap::new(),
            tree_set: HashMap::new(),
            line: 0,
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
