use std::any::Any;
use std::collections::{HashMap, VecDeque};

use lexer::{Item, ItemType, Lexer};
use node::*;
use utils::*;

pub type Func<'a> = &'a Fn(Option<Box<Any>>) -> Option<Box<Any>>;

pub struct Parser<'a> {
    name: String,
    text: String,
    funcs: Vec<HashMap<String, Func<'a>>>,
    lex: Option<Lexer>,
    line: usize,
    token: VecDeque<Item>,
    peek_count: usize,
    tree_ids: HashMap<TreeId, String>,
    tree_set: HashMap<String, Tree>,
    tree_id: TreeId,
    tree: Option<Tree>,
    tree_stack: VecDeque<Tree>,
    max_tree_id: TreeId,
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
            funcs: Vec::new(),
            lex: None,
            line: 0,
            token: VecDeque::new(),
            peek_count: 0,
            tree_ids: HashMap::new(),
            tree_set: HashMap::new(),
            tree_id: 0,
            tree: None,
            tree_stack: VecDeque::new(),
            max_tree_id: 0,
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
        self.token.push_front(t);
        self.peek_count += 1;
    }

    fn backup2(&mut self, t0: Item, t1: Item) {
        self.token.push_front(t1);
        self.token.push_front(t0);
        self.peek_count += 2;
    }

    fn backup3(&mut self, t0: Item, t1: Item, t2: Item) {
        self.token.push_front(t2);
        self.token.push_front(t1);
        self.token.push_front(t0);
        self.peek_count += 3;
    }

    fn next_must(&mut self, context: &str) -> Result<Item, String> {
        self.next()
            .ok_or_else(|| self.error_msg(format!("unexpected end in {}", context)))
    }

    fn next_non_space(&mut self) -> Option<Item> {
        self.skip_while(|c| c.typ == ItemType::ItemSpace).next()
    }

    fn next_non_space_must(&mut self, context: &str) -> Result<Item, String> {
        self.next_non_space()
            .ok_or_else(|| self.error_msg(format!("unexpected end in {}", context)))
    }

    fn peek_non_space(&mut self) -> Option<&Item> {
        if let Some(t) = self.next_non_space() {
            self.backup(t);
            return self.token.front();
        }
        None
    }

    fn peek_non_space_must(&mut self, context: &str) -> Result<&Item, String> {
        if let Some(t) = self.next_non_space() {
            self.backup(t);
            return Ok(self.token.front().unwrap());
        }
        self.error(format!("unexpected end in {}", context))
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
            self.tree_by_id(tree_id).map(|t| &t.parse_name)
        } else {
            self.tree.as_ref().map(|t| &t.parse_name)
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

    fn stop_parse(&mut self) -> Result<(), String> {
        self.add_to_tree_set()?;
        self.tree = self.tree_stack.pop_back();
        self.tree_id = self.tree.as_ref().map(|t| t.id).unwrap_or(0);
        Ok(())
    }
    // top level parser
    fn parse_tree(&mut self) -> Result<(), String> {
        let name = self.name.clone();
        let parse_name = self.name.clone();
        self.start_parse(name, 1, parse_name);
        self.parse()?;
        self.stop_parse()?;
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

    fn unexpected<T>(&self, token: &Item, context: &str) -> Result<T, String> {
        self.error(format!("unexpected {} in {}", token, context))
    }

    fn add_var(&mut self, name: String) -> Result<(), String> {
        let mut tree = self.tree
            .take()
            .ok_or_else(|| self.error_msg("no tree".to_owned()))?;
        tree.vars.push(name);
        self.tree = Some(tree);
        Ok(())
    }

    fn add_to_tree_set(&mut self) -> Result<(), String> {
        let tree = self.tree
            .take()
            .ok_or_else(|| self.error_msg("no tree".to_owned()))?;
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

    fn has_func(&self, name: &str) -> bool {
        self.funcs.iter().any(|map| map.contains_key(name))
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
                        self.stop_parse()?;
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

    fn item_list(&mut self) -> Result<(ListNode, Nodes), String> {
        let pos = self.peek_non_space_must("item list")?.pos;
        let mut list = ListNode::new(self.tree_id, pos);
        while self.peek_non_space_must("item list")?.typ != ItemType::ItemEOF {
            let node = self.text_or_action()?;
            match *node.typ() {
                NodeType::End | NodeType::Else => return Ok((list, node)),
                _ => list.append(node),
            }
        }
        self.error("unexpected EOF".to_owned())
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

    fn parse_control(&mut self,
                     allow_else_if: bool,
                     context: &str)
                     -> Result<(Pos, usize, PipeNode, Option<ListNode>), String> {
        Err("doom".to_owned())
    }

    fn if_control(&mut self) -> Result<IfNode, String> {
        //IfNode::new(self.tree_id, self.parse_control(true, "if"))
        Err("doom".to_owned())
    }

    fn block_control(&mut self) -> Result<TemplateNode, String> {
        let context = "block clause";
        let token = self.next_non_space_must(context)?;
        let name = self.parse_template_name(&token, context)?;
        let pipe = self.pipeline(context)?;

        self.max_tree_id += 1;
        let tree_id = self.max_tree_id;
        let parse_name = self.name.clone();
        self.start_parse(name.clone(), tree_id, parse_name);
        let (root, end) = self.item_list()?;
        self.tree.as_mut().map(|t| t.root = Some(root));
        if end.typ() != &NodeType::End {
            return self.error(format!("unexpected {} in {}", end, context));
        }
        self.stop_parse()?;
        Ok(TemplateNode::new(self.tree_id, token.pos, token.line, name, Some(pipe)))
    }

    fn template_control(&mut self) -> Result<Nodes, String> {
        let context = "template clause";
        let token = self.next_non_space_must(context)?;
        let name = self.parse_template_name(&token, context)?;
        let next = self.next_non_space().ok_or(format!("unexpected end"))?;
        let pipe = if next.typ != ItemType::ItemRightDelim {
            self.backup(next);
            Some(self.pipeline(context)?)
        } else {
            None
        };
        Ok(Nodes::Template(TemplateNode::new(self.tree_id, token.line, token.pos, name, pipe)))
    }

    fn pipeline(&mut self, context: &str) -> Result<PipeNode, String> {
        let mut decl = vec![];
        let mut token = self.next_non_space_must("pipeline")?;
        let pos = token.pos;
        // TODO: test this hard!
        while token.typ == ItemType::ItemVariable {
            let token_after_var = self.next_must("variable")?;
            let next = if token_after_var.typ == ItemType::ItemSpace {
                let next = self.next_non_space_must("variable")?;
                if next.typ != ItemType::ItemColonEquals &&
                   !(next.typ == ItemType::ItemChar && next.val == ",") {
                    self.backup3(token, token_after_var, next);
                    break;
                }
                next
            } else {
                token_after_var
            };
            if next.typ == ItemType::ItemColonEquals ||
               (next.typ == ItemType::ItemChar && next.val == ",") {
                self.add_var(next.val.clone())?;
                let variable = VariableNode::new(self.tree_id, next.pos, next.val.clone());
                decl.push(variable);
                if next.typ == ItemType::ItemChar && next.val == "," {
                    if context == "range" && decl.len() < 2 {
                        token = self.next_non_space_must("variable")?;
                        continue;
                    }
                    return self.error(format!("to many decalarations in {}", context));
                }
            } else {
                self.backup2(token, next);
            }
            break;
        }
        let mut pipe = PipeNode::new(self.tree_id, pos, decl);
        let mut token = self.next_non_space_must("pipeline")?;
        loop {
            match token.typ {
                ItemType::ItemRightDelim | ItemType::ItemRightParen => {
                    self.check_pipeline(&mut pipe, context)?;
                    if token.typ == ItemType::ItemRightParen {
                        self.backup(token);
                    }
                    return Ok(pipe);
                }
                ItemType::ItemBool |
                ItemType::ItemCharConstant |
                ItemType::ItemDot |
                ItemType::ItemField |
                ItemType::ItemIdentifier |
                ItemType::ItemNumber |
                ItemType::ItemNil |
                ItemType::ItemRawString |
                ItemType::ItemString |
                ItemType::ItemVariable |
                ItemType::ItemLeftParen => {
                    self.backup(token);
                    pipe.append(self.command()?);
                }
                _ => return self.unexpected(&token, context),
            }
            token = self.next_non_space_must("pipeline")?;
        }
    }

    fn check_pipeline(&mut self, pipe: &mut PipeNode, context: &str) -> Result<(), String> {
        if pipe.cmds.is_empty() {
            return self.error(format!("missing vlaue for {}", context));
        }
        for (i, c) in pipe.cmds.iter().enumerate().skip(1) {
            match c.args.first() {
                Some(n) => {
                    match *n.typ() {
                        NodeType::Bool | NodeType::Dot | NodeType::Nil | NodeType::Number |
                        NodeType::String => {
                            return self.error(format!("non executable command in pipeline stage {}",
                                                      i + 2))
                        }
                        _ => {}
                    }
                }
                None => {
                    return self.error(format!("non executable command in pipeline stage {}", i + 2))
                }
            }
        }
        Ok(())
    }

    fn command(&mut self) -> Result<CommandNode, String> {
        let mut cmd = CommandNode::new(self.tree_id, self.peek_non_space_must("command")?.pos);
        loop {
            self.peek_non_space_must("operand")?;
            if let Some(operand) = self.operand()? {
                cmd.append(operand);
            }
            let token = self.next_must("command")?;
            match token.typ {
                ItemType::ItemSpace => continue,
                ItemType::ItemError => return self.error(format!("{}", token.val)),
                ItemType::ItemRightDelim | ItemType::ItemRightParen => self.backup(token),
                ItemType::ItemPipe => {}
                _ => return self.error(format!("unexpected {} in operand", token)),
            };
            break;
        }
        if cmd.args.is_empty() {
            return self.error("empty command".to_owned());
        }
        Ok(cmd)
    }

    fn operand(&mut self) -> Result<Option<Nodes>, String> {
        let node = self.term()?;
        match node {
            None => Ok(None),
            Some(n) => {
                let next = self.next_must("operand")?;
                if next.typ == ItemType::ItemField {
                    let typ = n.typ().clone();
                    if typ == NodeType::Bool {
                        return self.error(format!("unexpected . after term {}", n.to_string()));
                    }
                    let mut chain = ChainNode::new(self.tree_id, next.pos, n);
                    chain.add(next.val);
                    while self.peek()
                              .map(|p| p.typ == ItemType::ItemField)
                              .unwrap_or(false) {
                        let field = self.next().unwrap();
                        chain.add(field.val);
                    }
                    let n = match typ {
                        NodeType::Field => {
                            Nodes::Field(FieldNode::new(self.tree_id,
                                                        chain.pos(),
                                                        chain.to_string()))
                        }
                        NodeType::Variable => {
                            Nodes::Variable(VariableNode::new(self.tree_id,
                                                              chain.pos(),
                                                              chain.to_string()))
                        }
                        _ => Nodes::Chain(chain),
                    };
                    Ok(Some(n))
                } else {
                    self.backup(next);
                    Ok(Some(n))
                }
            }
        }
    }

    fn term(&mut self) -> Result<Option<Nodes>, String> {
        let token = self.next_non_space_must("token")?;
        let node = match token.typ {
            ItemType::ItemError => return self.error(format!("{}", token.val)),
            ItemType::ItemIdentifier => {
                if !self.has_func(&token.val) {
                    return self.error(format!("function {} not defined", token.val));
                }
                let mut node = IdentifierNode::new(token.val);
                node.set_pos(token.pos);
                node.set_tree(self.tree_id);
                Nodes::Identifier(node)
            }
            ItemType::ItemDot => Nodes::Dot(DotNode::new(self.tree_id, token.pos)),
            ItemType::ItemNil => Nodes::Nil(NilNode::new(self.tree_id, token.pos)),
            ItemType::ItemVariable => {
                Nodes::Variable(self.use_var(self.tree_id, token.pos, token.val)?)
            }
            ItemType::ItemField => Nodes::Field(FieldNode::new(self.tree_id, token.pos, token.val)),
            ItemType::ItemBool => {
                Nodes::Bool(BoolNode::new(self.tree_id, token.pos, token.val == "true"))
            }
            ItemType::ItemCharConstant => {
                match NumberNode::new(self.tree_id, token.pos, token.val, token.typ) {
                    Ok(n) => Nodes::Number(n),
                    Err(e) => return self.error(e.to_string()),
                }
            }
            _ => {
                self.backup(token);
                return Ok(None);
            }
        };
        Ok(Some(node))
    }

    fn use_var(&self, tree_id: TreeId, pos: Pos, name: String) -> Result<VariableNode, String> {
        self.tree
            .as_ref()
            .and_then(|t| {
                          t.vars
                              .iter()
                              .find(|&v| v == &name)
                              .map(|_| VariableNode::new(tree_id, pos, name.clone()))
                      })
            .ok_or_else(|| self.error_msg(format!("undefined variable {}", name)))
    }

    fn parse_template_name(&self, token: &Item, context: &str) -> Result<String, String> {
        match token.typ {
            ItemType::ItemString | ItemType::ItemRawString => {
                unquote_str(&token.val).ok_or(format!("unable to parse string: {}", token.val))
            }
            _ => self.unexpected(token, context),
        }
    }
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
        make_parser_with(s)
    }

    fn make_parser_with<'a>(s: &str) -> Parser<'a> {
        let lex = Lexer::new("foo", s.to_owned());
        Parser {
            name: "foo".to_owned(),
            text: "nope".to_owned(),
            funcs: Vec::new(),
            lex: Some(lex),
            line: 0,
            token: VecDeque::new(),
            peek_count: 0,
            tree_ids: HashMap::new(),
            tree_set: HashMap::new(),
            tree_id: 0,
            tree: None,
            tree_stack: VecDeque::new(),
            max_tree_id: 0,
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

    #[test]
    fn test_pipeline_simple() {
        let mut p = make_parser_with(r#" $foo, $bar := yay | blub "2000" }}"#);
        let pipe = p.pipeline("range");
        // broken for now
        assert!(pipe.is_err());
    }

    #[test]
    fn test_term() {
        let mut p = make_parser_with(r#"{{true}}"#);
        p.next();
        let t = p.term();
        // broken for now
        assert!(t.is_ok());
        let t = t.unwrap();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.typ(), &NodeType::Bool);
        if let Nodes::Bool(n) = t {
            assert_eq!(n.val, true);
        } else {
            assert!(false);
        }

        let mut p = make_parser_with(r#"{{ false }}"#);
        p.next();
        let t = p.term();
        // broken for now
        assert!(t.is_ok());
        let t = t.unwrap();
        assert!(t.is_some());
        let t = t.unwrap();
        assert_eq!(t.typ(), &NodeType::Bool);
        assert_eq!(t.typ(), &NodeType::Bool);
        if let Nodes::Bool(n) = t {
            assert_eq!(n.val, false);
        } else {
            assert!(false);
        }
    }
}
