use std::fmt;

type Pos = usize;

enum ItemType {
    ItemError,                        // error occurred; value is text of error
    ItemBool,                         // boolean constant
    ItemChar,                         // printable ASCII character; grab bag for comma etc.
    ItemCharConstant,                 // character constant
    ItemComplex,                      // complex constant (1+2i); imaginary is just a number
    ItemColonEquals,                  // colon-equals (':=') introducing a declaration
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
    name:       String,    // the name of the input; used only for error reports
    input:      Vec<char>, // the string being scanned
    leftDelim:  String,    // start of action
    rightDelim: String,    // end of action
    state:      State,     // the next lexing function to enter
    pos:        Pos,       // current position in the input
    start:      Pos,       // start position of this item
    width:      Pos,       // width of last rune read from input
    lastPos:    Pos,       // position of most recent item returned by nextItem
    items:      Vec<Item>, // channel of scanned items
    parenDepth: usize,     // nesting depth of ( ) exprs
    line:       usize,     // 1+number of newlines seen

}

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

fn next(l: &mut Lexer) -> Option<char> {
    match l.input.get(l.pos) {
        Some(c) => {
            l.width = c.len_utf8();
            l.pos += 1;
            if *c == '\n' {
                l.line += 1;
            }
            Some(*c)
        },
        None => {
            l.width = 0;
            None
        }
    }
}

fn backup(l: &mut Lexer) {
    l.pos -= 1;
    if l.width == 1 && l.input.get(l.pos).and_then(|c| { if *c == '\n' { Some(()) } else { None }}).is_some() {
        l.line -= 1;
    }
}
