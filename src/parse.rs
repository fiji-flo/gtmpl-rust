use std::any::Any;
use std::collections::{HashMap, VecDeque};

use lexer::{Item, ItemType, Lexer};
use node::*;

pub type Func<'a> = &'a Fn(Option<Box<Any>>) -> Option<Box<Any>>;

pub struct Parser<'a> {
    name: String,
    text: String,
    funcs: HashMap<String, Func<'a>>,
    lex: Option<Lexer>,
    line: usize,
    token: VecDeque<Item>,
    peek_count: usize,
    tree_ids: HashMap<TreeId, String>,
    tree_set: HashMap<String, Tree>,
    tree_id: TreeId,
    tree: Option<Tree>,
    tree_stack: VecDeque<Tree>,
}

pub struct Tree {
    name: String,
    id: TreeId,
    parse_name: String,
    root: Option<ListNode>,
    vars: Vec<String>,
}

impl<'a> Parser<'a> {
    pub fn new(name: String) -> Parser<'a> {
        Parser {
            name,
            text: String::default(),
            funcs: HashMap::new(),
            lex: None,
            line: 0,
            token: VecDeque::new(),
            peek_count: 0,
            tree_ids: HashMap::new(),
            tree_set: HashMap::new(),
            tree_id: 0,
            tree: None,
            tree_stack: VecDeque::new(),
        }
    }
}

impl Tree {
    fn new(name: String, id: TreeId) -> Tree {
        Tree {
            name,
            id,
            parse_name: String::default(),
            root: None,
            vars: vec![],
        }
    }
}

pub fn parse<'a>(name: String,
                 text: String,
                 funcs: HashMap<String, Func<'a>>)
                 -> HashMap<String, Tree> {
    let tree_ids = HashMap::new();
    tree_ids
}

impl<'a> Parser<'a> {
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

    fn peek(&mut self) -> Option<&Item> {
        if let Some(t) = self.next() {
            self.backup(t);
            return self.token.front();
        }
        None
    }

    fn error_context(&mut self, n: Nodes) -> (String, String) {
        let pos = n.pos();
        let tree_id = n.tree();
        let parse_name = if tree_id == 0 && self.tree_ids.contains_key(&tree_id) {
            self.tree_by_id(tree_id).map(|t| { &t.parse_name })
        } else {
            self.tree.as_ref().map(|t| { &t.parse_name })
        };
        let text = &self.text[0..pos];
        let byte_num = match text.rfind('\n') {
            Some(i) => pos - (i + 1),
            None => pos,
        };
        let line_num = text.chars().filter(|c| *c == '\n').count();
        let context = n.to_string();
        (format!("{:?}:{}:{}", parse_name, line_num, byte_num), context)
    }

    fn start_parse(&mut self, name: String, id: TreeId, parse_name: String) {
        if let Some(t) = self.tree.take() {
            self.tree_stack.push_back(t);
        }
        self.tree_id = id;
        let mut t = Tree::new(name, id);
        t.parse_name = parse_name;
        self.tree = Some(t);
    }

    fn stop_parse(&mut self) {
        if let Some(t) = self.tree.take() {
            self.tree_ids.insert(t.id, t.name.clone());
            self.tree_set.insert(t.name.clone(), t);
        }
        self.tree = self.tree_stack.pop_back();
        self.tree_id = self.tree.as_ref().map(|t| {t.id}).unwrap_or(0);
    }
    // top level parser
    fn parse_tree(&mut self) -> Result<(), String> {
        let name = self.name.clone();
        let parse_name = self.name.clone();
        self.start_parse(name, 1, parse_name);
        self.parse()?;
        self.stop_parse();
        Ok(())
    }

    fn tree_by_id(&self, id: TreeId) -> Option<&Tree> {
        self.tree_ids
            .get(&id)
            .and_then(|name| self.tree_set.get(name))
    }
    fn add_tree(&mut self, name: &str, t: Tree) {
        self.tree_ids.insert(t.id, name.to_owned());
        self.tree_set.insert(name.to_owned(), t);
    }

    fn error<T>(&self, msg: String) -> Result<T, String> {
        Err(self.error_msg(msg))
    }

    fn error_msg(&self, msg: String) -> String {
        let name = if let Some(t) = self.tree.as_ref() {
            t.parse_name.clone()
        } else {
            self.name.clone()
        };
        format!("template: {}:{}:{}", name, self.line, msg)
    }

    fn unexpected<T>(&mut self, token: &Item, context: &str) -> Result<T, String> {
        self.error(format!("unexpected {} in {}", token, context))
    }

    fn add_to_tree_set(&mut self) -> Result<(), String> {
        let tree = self.tree.take().ok_or_else(|| {self.error_msg("no tree".to_owned())})?;
        if let Some(t) = self.tree_set.get(&tree.name) {
            if let Some(ref r) = t.root {
                match r.is_empty_tree() {
                    Err(e) => return Err(e),
                    Ok(false) => {
                        let err = format!("template multiple definitions of template {}",
                                          &tree.name);
                        return self.error(err);
                    }
                    Ok(true) => {}
                }
            }
        }
        self.tree_set.insert(tree.name.clone(), tree);
        Ok(())
    }

    fn parse(&mut self) -> Result<(), String> {
        if self.tree.is_none() {
            return self.error("no tree".to_owned());
        }
        let mut tree = self.tree.take().unwrap();
        let id = self.tree_id;
        let mut t = match self.next() {
            None => return self.error(format!("unable to peek for tree {}", id)),
            Some(t) => t,
        };
        tree.root = Some(ListNode::new(id, t.pos));
        while t.typ != ItemType::ItemEOF {
            if t.typ == ItemType::ItemLeftDelim {
                let nns = self.next_non_space();
                match nns {
                    Some(ref item) if item.typ == ItemType::ItemDefine => {
                        self.start_parse("definition".to_owned(), id + 1, tree.parse_name.clone());
                        self.parse()?;
                        self.stop_parse();
                        continue;
                    }
                    _ => {}
                };
                if let Some(t2) = nns {
                    self.backup2(t, t2);
                } else {
                    self.backup(t);
                }
            } else {
                self.backup(t);
            }
            let node = match self.text_or_action() {
                Ok(Nodes::Else(node)) => return self.error(format!("unexpected {}", node)),
                Ok(Nodes::End(node)) => return self.error(format!("unexpected {}", node)),
                Ok(node) => node,
                Err(e) => return Err(e),
            };
            tree.root.as_mut().map(|mut r| r.append(node));

            t = match self.next() {
                None => return self.error(format!("unable to peek for tree {}", id)),
                Some(t) => t,
            };
        }
        self.backup(t);
        Ok(())
    }

    fn text_or_action(&mut self) -> Result<Nodes, String> {
        match self.next_non_space() {
            Some(ref item) if item.typ == ItemType::ItemText => {
                Ok(Nodes::Text(TextNode::new(self.tree_id, item.pos, item.val.clone())))
            }
            Some(ref item) if item.typ == ItemType::ItemLeftDelim => self.action(),
            Some(ref item) => self.unexpected(item, "input"),
            _ => self.error(format!("unexpected end of input")),
        }
    }

    fn action(&mut self) -> Result<Nodes, String> {
        Err("doom".to_owned())
    }

    fn block_control(&mut self) -> Result<(), String> {

        Err("doom".to_owned())
    }

//    fn template_control(&mut self, token: Item, context: String) -> Result<String, String> {
//        match token.typ {
//            ItemType::ItemString | ItemType::ItemRawString => {
//
//            }
//
//        }
//    }
}

impl<'a> Iterator for Parser<'a> {
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
            }
            _ => None,
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
    fn make_parser<'a>() -> Parser<'a> {
        let s = r#"something {{ if eq "foo" "bar" }}"#;
        let lex = Lexer::new("foo", s.to_owned());
        Parser {
            name: "foo".to_owned(),
            text: "nope".to_owned(),
            funcs: HashMap::new(),
            lex: Some(lex),
            line: 0,
            token: VecDeque::new(),
            peek_count: 0,
            tree_ids: HashMap::new(),
            tree_set: HashMap::new(),
            tree_id: 0,
            tree: None,
            tree_stack: VecDeque::new(),
        }
    }

    #[test]
    fn test_iter() {
        let mut p = make_parser();
        assert_eq!(p.next().and_then(|n| Some(n.typ)), Some(ItemType::ItemText));
        assert_eq!(p.collect::<Vec<_>>().len(), 12);
    }

    #[test]
    fn test_backup() {
        let mut t = make_parser();
        assert_eq!(t.next().and_then(|n| Some(n.typ)), Some(ItemType::ItemText));
        let i = t.next().unwrap();
        let s = i.to_string();
        t.backup(i);
        assert_eq!(t.next().and_then(|n| Some(n.to_string())), Some(s));
        assert_eq!(t.last().and_then(|n| Some(n.typ)), Some(ItemType::ItemEOF));
    }
    #[test]
    fn test_backup3() {
        let mut t = make_parser();
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
        let mut t = make_parser();
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
        let mut t = make_parser();
        t.next();
        let i = t.next().unwrap();
        let typ = i.typ;
        assert_eq!(typ, ItemType::ItemLeftDelim);
        assert_eq!(t.peek_non_space().and_then(|n| Some(&n.typ)),
                   Some(&ItemType::ItemIf));
        assert_eq!(t.next().and_then(|n| Some(n.typ)), Some(ItemType::ItemIf));
        assert_eq!(t.last().and_then(|n| Some(n.typ)), Some(ItemType::ItemEOF));
    }

    #[test]
    fn parse_basic_tree() {
        let mut p = make_parser();
        let r = p.parse_tree();

        assert!(r.is_err());
        assert_eq!(&r.err().unwrap(), "doom")
    }
}
