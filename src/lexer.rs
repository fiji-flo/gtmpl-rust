use std::fmt;

type Pos = usize;

#[derive(Debug)]
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

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.typ {
            ItemEOF => write!(f, "EOF"),
            ItemError => write!(f, "{}", self.val),
            ItemKeyword => write!(f, "<{}>", self.val),
            _ => write!(f, "{}", self.val),
        }
    }
}

struct Lexer {
    name: String, // the name of the input; used only for error reports
    input: Vec<char>, // the string being scanned
    left_delim: String, // start of action
    right_delim: String, // end of action
    state: State, // the next lexing function to enter
    pos: Pos, // current position in the input
    start: Pos, // start position of this item
    width: Pos, // width of last rune read from input
    last_pos: Pos, // position of most recent item returned by nextItem
    items: Vec<Item>, // channel of scanned items
    paren_depth: usize, // nesting depth of ( ) exprs
    line: usize, // 1+number of newlines seen
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
    LexFieldOrVariable,
    LexChar,
    LexNumber,
    LexQuote,
    LexRawQuote,
}

impl Lexer {
    fn next(&mut self) -> Option<char> {
        match self.input.get(self.pos) {
            Some(c) => {
                self.width = c.len_utf8();
                self.pos += 1;
                if *c == '\n' {
                    self.line += 1;
                }
                Some(*c)
            }
            None => {
                self.width = 0;
                None
            }
        }
    }

    fn backup(&mut self) {
        self.pos -= 1;
        if self.width == 1 &&
            self.input
            .get(self.pos)
            .and_then(|c| if *c == '\n' { Some(()) } else { None })
            .is_some() {
                self.line -= 1;
            }
    }

    fn peek(&mut self) -> Option<char> {
        let c = self.next();
        self.backup();
        c
    }
}
