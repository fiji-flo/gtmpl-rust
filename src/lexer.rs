use std::ascii::AsciiExt;
use std::collections::HashMap;
use std::fmt;
use std::thread;
use std::sync::mpsc::{channel, Receiver, Sender};

type Pos = usize;

static LEFT_TRIM_MARKER: &str  = "- ";
static RIGHT_TRIM_MARKER: &str  = " -";
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
        m.insert("else", ItemType::ItemElse);
        m.insert("if", ItemType::ItemIf);
        m.insert("range", ItemType::ItemRange);
        m.insert("nil", ItemType::ItemNil);
        m.insert("template", ItemType::ItemTemplate);
        m.insert("with", ItemType::ItemWith);
        m
    };
}

#[derive(Debug)]
#[derive(Clone)]
#[derive(PartialEq)]
enum ItemType {
    ItemError, // error occurred; value is text of error
    ItemBool, // boolean constant
    ItemChar, // printable ASCII character; grab bag for comma etc.
    ItemCharConstant, // character constant
    ItemComplex, // complex constant (1+2i); imaginary is just a number
    ItemColonEquals, // colon-equals (':=') introducing a declaration
    ItemEOF,
    ItemField, // alphanumeric identifier starting with '.'
    ItemIdentifier, // alphanumeric identifier not starting with '.'
    ItemLeftDelim, // left action delimiter
    ItemLeftParen, // '(' inside action
    ItemNumber, // simple number, including imaginary
    ItemPipe, // pipe symbol
    ItemRawString, // raw quoted string (includes quotes)
    ItemRightDelim, // right action delimiter
    ItemRightParen, // ')' inside action
    ItemSpace, // run of spaces separating arguments
    ItemString, // quoted string (includes quotes)
    ItemText, // plain text
    ItemVariable, // variable starting with '$', such as '$' or  '$1' or '$hello'
    // Keywords, appear after all the rest.
    ItemKeyword, // used only to delimit the keywords
    ItemBlock, // block keyword
    ItemDot, // the cursor, spelled '.'
    ItemDefine, // define keyword
    ItemElse, // else keyword
    ItemEnd, // end keyword
    ItemIf, // if keyword
    ItemNil, // the untyped nil constant, easiest to treat as a keyword
    ItemRange, // range keyword
    ItemTemplate, // template keyword
    ItemWith, // with keyword
}

struct Item {
    pub typ: ItemType,
    pub pos: Pos,
    pub val: String,
    pub line: usize,
}

impl Item {
    pub fn new(typ: ItemType, pos: Pos, val: &str, line: usize) -> Item {
        Item {
            typ: typ,
            pos: pos,
            val: val.to_owned(),
            line: line,
        }
    }
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.typ {
            ItemType::ItemEOF => write!(f, "EOF"),
            ItemType::ItemError => write!(f, "{}", self.val),
            ItemType::ItemKeyword => write!(f, "<{}>", self.val),
            _ => write!(f, "{}", self.val),
        }
    }
}

struct Lexer {
    name: String, // the name of the input; used only for error reports
    last_pos: Pos, // position of most recent item returned by nextItem
    items_receiver: Receiver<Item>, // channel of scanned items
}

struct LexerStateMachine {
    name: String, // the name of the input; used only for error reports
    input: String, // the string being scanned
    state: State, // the next lexing function to enter
    pos: Pos, // current position in the input
    start: Pos, // start position of this item
    width: Pos, // width of last rune read from input
    items_sender: Sender<Item>, // channel of scanned items
    paren_depth: usize, // nesting depth of ( ) exprs
    line: usize, // 1+number of newlines seen
}

#[derive(Debug)]
enum State {
    Start,
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
    LexFieldOrVariable,
    LexChar,
    LexNumber,
    LexQuote,
    LexRawQuote,
}

impl Lexer {
    pub fn new(name: &str, input: String) -> Lexer {
        let (tx, rx) = channel();
        let mut l = LexerStateMachine {
            name: name.to_owned(),
            input: input,
            state: State::Start,
            pos: 0,
            start: 0,
            width: 0,
            items_sender: tx,
            paren_depth: 0,
            line: 1,
        };
        thread::spawn(move || l.run());
        Lexer {
            name: name.to_owned(),
            last_pos: 0,
            items_receiver: rx,
        }
    }

    pub fn next_item(&mut self) -> Item {
        match self.items_receiver.recv() {
            Ok(item) => {
                self.last_pos = item.pos;
                item
            }
            Err(e) => Item::new(ItemType::ItemError, 0, &format!("{}", e), 0),
        }
    }

    pub fn drain(&mut self) {
        for _ in self.items_receiver.iter() {}
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
        self.state = State::LexText;
        loop {
            match self.state {
                State::End => {
                    return;
                }
                State::LexText => {
                    self.state = self.lex_text();
                }
                _ => {
                    return;
                }
                /*
                State::End,
                State::LexComment,
                State::LexRightDelim,
                State::LexInsideAction,
                State::LexSpace,
                State::LexIdentifier,
                State::LexField,
                State::LexVariable,
                State::LexFieldOrVariable,
                State::LexChar,
                State::LexNumber,
                State::LexQuote,
                State::LexRawQuote,
                 */
            }
        }
    }

    fn backup(&mut self) {
        self.pos -= 1;
        if self.width == 1 &&
           self.input[self.pos..]
               .chars()
               .next()
               .and_then(|c| if c == '\n' { Some(()) } else { None })
               .is_some() {
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
            ItemType::ItemText | ItemType::ItemRawString | ItemType::ItemLeftDelim |
            ItemType::ItemRightDelim => 1,
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
        if self.next()
               .and_then(|s| Some(valid.contains(s)))
               .is_some() {
            return true;
        }
        self.backup();
        false
    }

    fn accpet_run(&mut self, valid: &str) {
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
                let ld = self.pos + LEFT_DELIM.len();
                self.pos += x;
                let mut trim = 0;
                if self.input[ld..].starts_with(LEFT_TRIM_MARKER) {
                    trim = rtrim_len(&self.input[self.start..self.pos]);
                }
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
        self.pos = LEFT_DELIM.len();
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
        self.pos = LEFT_COMMENT.len();
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
            self.pos = ltrim_len(&self.input[self.pos..]);
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
        self.emit(ItemType::ItemLeftDelim);
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
            None | Some('\r') | Some('\n') => {
                return self.errorf("unclosed action");
            },
            Some(c) => match c {
                '"' => State::LexQuote,
                '`' => State::LexRawQuote,
                '$' => State::LexVariable,
                '\'' => State::LexChar,
                '(' => {
                    self.emit(ItemType::ItemLeftParen);
                    self.paren_depth += 1;
                    State::LexInsideAction
                },
                ')' => {
                    self.emit(ItemType::ItemRightParen);
                    if self.paren_depth == 0 {
                        return self.errorf(&format!("unexpected right paren {}", c));
                    }
                    self.paren_depth -= 1;
                    State::LexInsideAction
                }
                ':' => {
                    match self.next() {
                        Some('=') => {
                            self.emit(ItemType::ItemColonEquals);
                            State::LexInsideAction
                        },
                        _ => {
                            return self.errorf("expected :=");
                        }
                    }
                },
                '|' => {
                    self.emit(ItemType::ItemPipe);
                    State::LexInsideAction
                },
                '.' => {
                    match self.input[self.pos..].chars().next() {
                        Some('0' ... '9') => {
                            self.backup();
                            State::LexNumber
                        },
                        _ => State::LexField,
                    }
                },
                '+' | '-' | '0' ... '9' => {
                    self.backup();
                    State::LexNumber
                },
                _ if c.is_whitespace() => State::LexSpace,
                _ if c.is_alphanumeric() => {
                    self.backup();
                    State::LexIdentifier
                },
                _ if c.is_ascii() => { // figure out a way to check for unicode.isPrint ?!
                    self.emit(ItemType::ItemChar);
                    State::LexInsideAction
                }
                _ => {
                    return self.errorf(&format!("unrecognized character in action {}", c));
                },
            }
        }
    }

    fn lex_identifier(&mut self) -> State {
        let c = self.skip_while(|c| c.is_alphanumeric()).next();
        self.backup();
        if self.at_terminator() {
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

    fn at_terminator(&mut self) -> bool {
        match self.peek() {
            Some(c) => {
                match c {
                    '.' | ',' | '|' | ':' | ')' | '('  => true,
                    ' ' | '\t' | '\r' | '\n' => true,
                    // this is what golang does to detect a delimiter
                    _  => {
                        LEFT_DELIM.starts_with(c)
                    }
                }
            },
            None => false,
        }
    }
}

fn rtrim_len(s: &str) -> usize {
    let l = s.len();
    l - s.rfind(|c: char| !c.is_whitespace()).unwrap_or(l)
}

fn ltrim_len(s: &str) -> usize {
    let l = s.len();
    l - s.find(|c: char| !c.is_whitespace()).unwrap_or(l)
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lexer_run() {
        let mut l = Lexer::new("foo", "abc".to_owned());
        let i1 = l.next_item();
        assert_eq!(i1.typ, ItemType::ItemText);
        assert_eq!(&i1.val, "abc");
        l.drain();
    }
}
