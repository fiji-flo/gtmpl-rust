use lazy_static::lazy_static;
use std::collections::HashMap;
use std::fmt;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

type Pos = usize;

static LEFT_TRIM_MARKER: &str = "- ";
static RIGHT_TRIM_MARKER: &str = " -";
static LEFT_DELIM: &str = "{{";
static RIGHT_DELIM: &str = "}}";
static LEFT_COMMENT: &str = "/*";
static RIGHT_COMMENT: &str = "*/";

lazy_static! {
    static ref KEY: HashMap<&'static str, ItemType> = {
        let mut m = HashMap::new();
        m.insert(".", ItemType::ItemDot);
        m.insert("block", ItemType::ItemBlock);
        m.insert("define", ItemType::ItemDefine);
        m.insert("end", ItemType::ItemEnd);
        m.insert("else", ItemType::ItemElse);
        m.insert("if", ItemType::ItemIf);
        m.insert("range", ItemType::ItemRange);
        m.insert("nil", ItemType::ItemNil);
        m.insert("template", ItemType::ItemTemplate);
        m.insert("with", ItemType::ItemWith);
        m
    };
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, PartialEq)]
pub enum ItemType {
    ItemError,        // error occurred; value is text of error
    ItemBool,         // boolean constant
    ItemChar,         // printable ASCII character; grab bag for comma etc.
    ItemCharConstant, // character constant
    ItemComplex,      // complex constant (1+2i); imaginary is just a number
    ItemColonEquals,  // colon-equals (':=') introducing a declaration
    ItemEOF,
    ItemField,      // alphanumeric identifier starting with '.'
    ItemIdentifier, // alphanumeric identifier not starting with '.'
    ItemLeftDelim,  // left action delimiter
    ItemLeftParen,  // '(' inside action
    ItemNumber,     // simple number, including imaginary
    ItemPipe,       // pipe symbol
    ItemRawString,  // raw quoted string (includes quotes)
    ItemRightDelim, // right action delimiter
    ItemRightParen, // ')' inside action
    ItemSpace,      // run of spaces separating arguments
    ItemString,     // quoted string (includes quotes)
    ItemText,       // plain text
    ItemVariable,   // variable starting with '$', such as '$' or  '$1' or '$hello'
    // Keywords, appear after all the rest.
    ItemKeyword,  // used only to delimit the keywords
    ItemBlock,    // block keyword
    ItemDot,      // the cursor, spelled '.'
    ItemDefine,   // define keyword
    ItemElse,     // else keyword
    ItemEnd,      // end keyword
    ItemIf,       // if keyword
    ItemNil,      // the untyped nil constant, easiest to treat as a keyword
    ItemRange,    // range keyword
    ItemTemplate, // template keyword
    ItemWith,     // with keyword
}

#[derive(Debug)]
pub struct Item {
    pub typ: ItemType,
    pub pos: Pos,
    pub val: String,
    pub line: usize,
}

impl Item {
    pub fn new<T: Into<String>>(typ: ItemType, pos: Pos, val: T, line: usize) -> Item {
        Item {
            typ,
            pos,
            val: val.into(),
            line,
        }
    }
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.typ {
            ItemType::ItemEOF => write!(f, "EOF"),
            ItemType::ItemKeyword => write!(f, "<{}>", self.val),
            _ => write!(f, "{}", self.val),
        }
    }
}

pub struct Lexer {
    last_pos: Pos,                  // position of most recent item returned by nextItem
    items_receiver: Receiver<Item>, // channel of scanned items
    finished: bool,                 // flag if lexer is finished
}

struct LexerStateMachine {
    input: String,              // the string being scanned
    state: State,               // the next lexing function to enter
    pos: Pos,                   // current position in the input
    start: Pos,                 // start position of this item
    width: Pos,                 // width of last rune read from input
    items_sender: Sender<Item>, // channel of scanned items
    paren_depth: usize,         // nesting depth of ( ) exprs
    line: usize,                // 1+number of newlines seen
}

#[derive(Debug)]
enum State {
    End,
    LexText,
    LexLeftDelim,
    LexComment,
    LexRightDelim,
    LexInsideAction,
    LexSpace,
    LexIdentifier,
    LexField,
    LexVariable,
    LexChar,
    LexNumber,
    LexQuote,
    LexRawQuote,
}

impl Iterator for Lexer {
    type Item = Item;
    fn next(&mut self) -> Option<Item> {
        if self.finished {
            return None;
        }
        let item = match self.items_receiver.recv() {
            Ok(item) => {
                self.last_pos = item.pos;
                if item.typ == ItemType::ItemError || item.typ == ItemType::ItemEOF {
                    self.finished = true;
                }
                item
            }
            Err(e) => {
                self.finished = true;
                Item::new(ItemType::ItemError, 0, format!("{}", e), 0)
            }
        };
        Some(item)
    }
}

impl Lexer {
    pub fn new(input: String) -> Lexer {
        let (tx, rx) = channel();
        let mut l = LexerStateMachine {
            input,
            state: State::LexText,
            pos: 0,
            start: 0,
            width: 0,
            items_sender: tx,
            paren_depth: 0,
            line: 1,
        };
        thread::spawn(move || l.run());
        Lexer {
            last_pos: 0,
            items_receiver: rx,
            finished: false,
        }
    }

    pub fn drain(&mut self) {
        for _ in self.items_receiver.iter() {}
    }
}

impl Drop for Lexer {
    fn drop(&mut self) {
        self.drain();
    }
}

impl Iterator for LexerStateMachine {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        match self.input[self.pos..].chars().next() {
            Some(c) => {
                self.width = c.len_utf8();
                self.pos += self.width;
                if c == '\n' {
                    self.line += 1;
                }
                Some(c)
            }
            None => {
                self.width = 0;
                None
            }
        }
    }
}

impl LexerStateMachine {
    fn run(&mut self) {
        loop {
            self.state = match self.state {
                State::LexText => self.lex_text(),
                State::LexComment => self.lex_comment(),
                State::LexLeftDelim => self.lex_left_delim(),
                State::LexRightDelim => self.lex_right_delim(),
                State::LexInsideAction => self.lex_inside_action(),
                State::LexSpace => self.lex_space(),
                State::LexIdentifier => self.lex_identifier(),
                State::LexField => self.lex_field(),
                State::LexVariable => self.lex_variable(),
                State::LexChar => self.lex_char(),
                State::LexNumber => self.lex_number(),
                State::LexQuote => self.lex_quote(),
                State::LexRawQuote => self.lex_raw_quote(),
                State::End => {
                    return;
                }
            }
        }
    }

    fn backup(&mut self) {
        self.pos -= 1;
        if self.width == 1
            && self.input[self.pos..]
                .chars()
                .next()
                .and_then(|c| if c == '\n' { Some(()) } else { None })
                .is_some()
        {
            self.line -= 1;
        }
    }

    fn peek(&mut self) -> Option<char> {
        let c = self.next();
        self.backup();
        c
    }

    fn emit(&mut self, t: ItemType) {
        let s = &self.input[self.start..self.pos];
        let lines = match t {
            ItemType::ItemText
            | ItemType::ItemRawString
            | ItemType::ItemLeftDelim
            | ItemType::ItemRightDelim => 1,
            _ => s.chars().filter(|c| *c == '\n').count(),
        };
        self.items_sender
            .send(Item::new(t, self.start, s, self.line))
            .unwrap();
        self.line += lines;
        self.start = self.pos;
    }

    fn ignore(&mut self) {
        self.start = self.pos;
    }

    fn accept(&mut self, valid: &str) -> bool {
        if self.next().map(|s| valid.contains(s)).unwrap_or_default() {
            return true;
        }
        self.backup();
        false
    }

    fn accept_run(&mut self, valid: &str) {
        while self.accept(valid) {}
    }

    fn errorf(&mut self, msg: &str) -> State {
        self.items_sender
            .send(Item::new(ItemType::ItemError, self.start, msg, self.line))
            .unwrap();
        State::End
    }

    fn lex_text(&mut self) -> State {
        self.width = 0;
        let x = self.input[self.pos..].find(&LEFT_DELIM);
        match x {
            Some(x) => {
                self.pos += x;
                let ld = self.pos + LEFT_DELIM.len();
                let trim = if self.input[ld..].starts_with(LEFT_TRIM_MARKER) {
                    rtrim_len(&self.input[self.start..self.pos])
                } else {
                    0
                };
                self.pos -= trim;
                if self.pos > self.start {
                    self.emit(ItemType::ItemText);
                }
                self.pos += trim;
                self.ignore();
                State::LexLeftDelim
            }
            None => {
                self.pos = self.input.len();
                if self.pos > self.start {
                    self.emit(ItemType::ItemText);
                }
                self.emit(ItemType::ItemEOF);
                State::End
            }
        }
    }

    fn at_right_delim(&mut self) -> (bool, bool) {
        if self.input[self.pos..].starts_with(&RIGHT_DELIM) {
            return (true, false);
        }
        if self.input[self.pos..].starts_with(&format!("{}{}", RIGHT_TRIM_MARKER, RIGHT_DELIM)) {
            return (true, true);
        }
        (false, false)
    }

    fn lex_left_delim(&mut self) -> State {
        self.pos += LEFT_DELIM.len();
        let trim = self.input[self.pos..].starts_with(LEFT_TRIM_MARKER);
        let after_marker = if trim { LEFT_TRIM_MARKER.len() } else { 0 };
        if self.input[(self.pos + after_marker)..].starts_with(LEFT_COMMENT) {
            self.pos += after_marker;
            self.ignore();
            State::LexComment
        } else {
            self.emit(ItemType::ItemLeftDelim);
            self.pos += after_marker;
            self.ignore();
            self.paren_depth = 0;
            State::LexInsideAction
        }
    }

    fn lex_comment(&mut self) -> State {
        self.pos += LEFT_COMMENT.len();
        let i = match self.input[self.pos..].find(RIGHT_COMMENT) {
            Some(i) => i,
            None => {
                return self.errorf("unclosed comment");
            }
        };

        self.pos += i + RIGHT_COMMENT.len();
        let (delim, trim) = self.at_right_delim();

        if !delim {
            return self.errorf("comment end before closing delimiter");
        }

        if trim {
            self.pos += RIGHT_TRIM_MARKER.len();
        }

        self.pos += RIGHT_DELIM.len();

        if trim {
            self.pos += ltrim_len(&self.input[self.pos..]);
        }

        self.ignore();
        State::LexText
    }

    fn lex_right_delim(&mut self) -> State {
        let trim = self.input[self.pos..].starts_with(RIGHT_TRIM_MARKER);
        if trim {
            self.pos += RIGHT_TRIM_MARKER.len();
            self.ignore();
        }
        self.pos += RIGHT_DELIM.len();
        self.emit(ItemType::ItemRightDelim);
        if trim {
            self.pos += ltrim_len(&self.input[self.pos..]);
            self.ignore();
        }
        State::LexText
    }

    fn lex_inside_action(&mut self) -> State {
        let (delim, _) = self.at_right_delim();
        if delim {
            if self.paren_depth == 0 {
                return State::LexRightDelim;
            }
            return self.errorf("unclosed left paren");
        }

        match self.next() {
            None | Some('\r') | Some('\n') => self.errorf("unclosed action"),
            Some(c) => {
                match c {
                    '"' => State::LexQuote,
                    '`' => State::LexRawQuote,
                    '$' => State::LexVariable,
                    '\'' => State::LexChar,
                    '(' => {
                        self.emit(ItemType::ItemLeftParen);
                        self.paren_depth += 1;
                        State::LexInsideAction
                    }
                    ')' => {
                        self.emit(ItemType::ItemRightParen);
                        if self.paren_depth == 0 {
                            return self.errorf(&format!("unexpected right paren {}", c));
                        }
                        self.paren_depth -= 1;
                        State::LexInsideAction
                    }
                    ':' => match self.next() {
                        Some('=') => {
                            self.emit(ItemType::ItemColonEquals);
                            State::LexInsideAction
                        }
                        _ => self.errorf("expected :="),
                    },
                    '|' => {
                        self.emit(ItemType::ItemPipe);
                        State::LexInsideAction
                    }
                    '.' => match self.input[self.pos..].chars().next() {
                        Some('0'..='9') => {
                            self.backup();
                            State::LexNumber
                        }
                        _ => State::LexField,
                    },
                    '+' | '-' | '0'..='9' => {
                        self.backup();
                        State::LexNumber
                    }
                    _ if c.is_whitespace() => State::LexSpace,
                    _ if c.is_alphanumeric() || c == '_' => {
                        self.backup();
                        State::LexIdentifier
                    }
                    _ if c.is_ascii() => {
                        // figure out a way to check for unicode.isPrint ?!
                        self.emit(ItemType::ItemChar);
                        State::LexInsideAction
                    }
                    _ => self.errorf(&format!("unrecognized character in action {}", c)),
                }
            }
        }
    }

    fn lex_space(&mut self) -> State {
        while self.peek().map(|c| c.is_whitespace()).unwrap_or_default() {
            self.next();
        }
        self.emit(ItemType::ItemSpace);
        State::LexInsideAction
    }

    fn lex_identifier(&mut self) -> State {
        let c = self.find(|c| !(c.is_alphanumeric() || *c == '_'));
        self.backup();
        if !self.at_terminator() {
            return self.errorf(&format!("bad character {}", c.unwrap_or_default()));
        }
        let item_type = match &self.input[self.start..self.pos] {
            "true" | "false" => ItemType::ItemBool,
            word if KEY.contains_key(word) => (*KEY.get(word).unwrap()).clone(),
            word if word.starts_with('.') => ItemType::ItemField,
            _ => ItemType::ItemIdentifier,
        };
        self.emit(item_type);
        State::LexInsideAction
    }

    fn lex_field(&mut self) -> State {
        self.lex_field_or_variable(ItemType::ItemField)
    }

    fn lex_variable(&mut self) -> State {
        self.lex_field_or_variable(ItemType::ItemVariable)
    }

    fn lex_field_or_variable(&mut self, typ: ItemType) -> State {
        if self.at_terminator() {
            self.emit(match typ {
                ItemType::ItemVariable => ItemType::ItemVariable,
                _ => ItemType::ItemDot,
            });
            return State::LexInsideAction;
        }
        let c = self.find(|c| !(c.is_alphanumeric() || *c == '_'));
        self.backup();

        if !self.at_terminator() {
            return self.errorf(&format!("bad character {}", c.unwrap_or_default()));
        }
        self.emit(typ);
        State::LexInsideAction
    }

    fn at_terminator(&mut self) -> bool {
        match self.peek() {
            Some(c) => {
                match c {
                    '.' | ',' | '|' | ':' | ')' | '(' | ' ' | '\t' | '\r' | '\n' => true,
                    // this is what golang does to detect a delimiter
                    _ => RIGHT_DELIM.starts_with(c),
                }
            }
            None => false,
        }
    }

    fn lex_char(&mut self) -> State {
        let mut escaped = false;
        loop {
            let c = self.next();
            match c {
                Some('\\') => {
                    escaped = true;
                    continue;
                }
                Some('\n') | None => {
                    return self.errorf("unterminated character constant");
                }
                Some('\'') if !escaped => {
                    break;
                }
                _ => {}
            };
            escaped = false;
        }
        self.emit(ItemType::ItemCharConstant);
        State::LexInsideAction
    }

    fn lex_number(&mut self) -> State {
        if self.scan_number() {
            // Let's ingnore complex numbers here.
            self.emit(ItemType::ItemNumber);
            State::LexInsideAction
        } else {
            let msg = &format!("bad number syntax: {}", &self.input[self.start..self.pos]);
            self.errorf(msg)
        }
    }

    fn scan_number(&mut self) -> bool {
        self.accept("+-");
        if self.accept("0") && self.accept("xX") {
            let digits = "0123456789abcdefABCDEF";
            self.accept_run(digits);
        } else {
            let digits = "0123456789";
            self.accept_run(digits);
            if self.accept(".") {
                self.accept_run(digits);
            }
            if self.accept("eE") {
                self.accept("+-");
                self.accept_run(digits);
            }
        }
        // Let's ignore imaginary numbers for now.
        if self.peek().map(|c| c.is_alphanumeric()).unwrap_or(true) {
            self.next();
            return false;
        }
        true
    }

    fn lex_quote(&mut self) -> State {
        let mut escaped = false;
        loop {
            let c = self.next();
            match c {
                Some('\\') => {
                    escaped = true;
                    continue;
                }
                Some('\n') | None => {
                    return self.errorf("unterminated quoted string");
                }
                Some('"') if !escaped => {
                    break;
                }
                _ => {}
            };
            escaped = false;
        }
        self.emit(ItemType::ItemString);
        State::LexInsideAction
    }

    fn lex_raw_quote(&mut self) -> State {
        let start_line = self.line;
        if !self.any(|c| c == '`') {
            self.line = start_line;
            return self.errorf("unterminated raw quoted string");
        }
        self.emit(ItemType::ItemRawString);
        State::LexInsideAction
    }
}

fn rtrim_len(s: &str) -> usize {
    match s.rfind(|c: char| !c.is_whitespace()) {
        Some(i) => s.len() - 1 - i,
        None => s.len(),
    }
}

fn ltrim_len(s: &str) -> usize {
    let l = s.len();
    s.find(|c: char| !c.is_whitespace()).unwrap_or(l)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexer_run() {
        let mut l = Lexer::new("abc".to_owned());
        let i1 = l.next().unwrap();
        assert_eq!(i1.typ, ItemType::ItemText);
        assert_eq!(&i1.val, "abc");
    }

    #[test]
    fn lex_simple() {
        let s = r#"something {{ if eq "foo" "bar" }}"#;
        let l = Lexer::new(s.to_owned());
        assert_eq!(l.count(), 13);
    }

    #[test]
    fn test_whitespace() {
        let s = r#"something {{  .foo  }}"#;
        let l = Lexer::new(s.to_owned());
        let s_ = l.map(|i| i.val).collect::<Vec<String>>().join("");
        assert_eq!(s_, s);
    }

    #[test]
    fn test_input() {
        let s = r#"something {{ .foo }}"#;
        let l = Lexer::new(s.to_owned());
        let s_ = l.map(|i| i.val).collect::<Vec<String>>().join("");
        assert_eq!(s_, s);
    }

    #[test]
    fn test_underscore() {
        let s = r#"something {{ .foo_bar }}"#;
        let l = Lexer::new(s.to_owned());
        let s_ = l.map(|i| i.val).collect::<Vec<String>>().join("");
        assert_eq!(s_, s);
    }

    #[test]
    fn test_trim() {
        let s = r#"something {{- .foo -}} 2000"#;
        let l = Lexer::new(s.to_owned());
        let s_ = l.map(|i| i.val).collect::<Vec<String>>().join("");
        assert_eq!(s_, r#"something{{.foo}}2000"#);
    }

    #[test]
    fn test_comment() {
        let s = r#"something {{- /* foo */ -}} 2000"#;
        let l = Lexer::new(s.to_owned());
        let s_ = l.map(|i| i.val).collect::<Vec<String>>().join("");
        assert_eq!(s_, r#"something2000"#);
    }
}
