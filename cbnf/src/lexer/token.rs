//! primitive lexing

/// Parsed token.
/// It doesn't contain information about data that has been parsed,
/// only the type of the token and its size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Lexeme {
    pub kind: LexKind,
    pub len: u32,
}

impl Lexeme {
    #[must_use]
    pub const fn new(kind: LexKind, len: u32) -> Self {
        Self { kind, len }
    }
}

/// Enum representing common lexeme types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LexKind {
    // Multi-char tokens:
    /// "// comment"
    LineComment { doc_style: Option<DocStyle> },

    /// `/* block comment */`
    ///
    /// Block comments can be recursive, so a sequence like `/* /* */`
    /// will not be considered terminated and will result in a parsing error.
    BlockComment {
        doc_style: Option<DocStyle>,
        terminated: bool,
    },

    /// Any whitespace character sequence.
    Whitespace,

    /// "ident" or "continue"
    ///
    /// At this step, keywords are also considered identifiers.
    Ident,

    /// Like the above, but containing invalid unicode codepoints.
    InvalidIdent,

    /// Examples: `12u8`, `1.0e-40`, `b"123"`. Note that `_` is an invalid
    /// suffix, but may be present here on string and float literals. Users of
    /// this type will need to check for and reject that case.
    ///
    /// See [`LiteralKind`] for more details.
    Literal {
        kind: LiteralKind,
        suffix_start: u32,
    },

    // One-char tokens:
    /// ";"
    Semi,
    /// ","
    Comma,
    /// "."
    Dot,
    /// "("
    OpenParen,
    /// ")"
    CloseParen,
    /// "{"
    OpenBrace,
    /// "}"
    CloseBrace,
    /// "["
    OpenBracket,
    /// "]"
    CloseBracket,
    /// "@"
    At,
    /// "~"
    Tilde,
    /// "?"
    Question,
    /// ":"
    Colon,
    /// "$"
    Dollar,
    /// "="
    Eq,
    /// "!"
    Bang,
    /// "<"
    Lt,
    /// ">"
    Gt,
    /// "-"
    Minus,
    /// "&"
    And,
    /// "|"
    Or,
    /// "+"
    Plus,
    /// "*"
    Star,
    /// "/"
    Slash,
    /// "^"
    Caret,
    /// "%"
    Percent,

    /// Unknown token, not expected by the lexer, e.g. "№"
    Unknown,

    /// End of input.
    Eof,
}

impl LexKind {
    #[must_use]
    pub const fn name(self) -> &'static str {
        use LexKind::*;

        match self {
            LineComment { .. } => "line comment",
            BlockComment { .. } => "block comment",
            Whitespace => "whitespace",
            Ident => "ident",
            InvalidIdent => "invalid ident",
            Literal { .. } => "literal",
            Semi => "semicolon",
            Comma => "comma",
            Dot => "dot",
            OpenParen => "open parenthesis",
            CloseParen => "close parenthesis",
            OpenBrace => "open brace",
            CloseBrace => "close brace",
            OpenBracket => "open bracket",
            CloseBracket => "close bracket",
            At => "@",
            Tilde => "~",
            Question => "?",
            Colon => ":",
            Dollar => "$",
            Eq => "=",
            Bang => "!",
            Lt => "<",
            Gt => ">",
            Minus => "-",
            And => "&",
            Or => "|",
            Plus => "+",
            Star => "*",
            Slash => "/",
            Caret => "^",
            Percent => "%",
            Unknown => "unknown",
            Eof => "end of file",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DocStyle {
    Outer,
    Inner,
}

/// Enum representing the literal types supported by the lexer.
///
/// Note that the suffix is *not* considered when deciding the `LiteralKind` in
/// this type. This means that float literals like `1f32` are classified by this
/// type as `Int`. (Compare against `rustc_ast::token::LitKind` and
/// `rustc_ast::ast::LitKind`).
#[allow(clippy::doc_markdown)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LiteralKind {
    /// "12_u8", "0o100", "0b120i99", "1f32".
    Int { base: Base, empty_int: bool },
    /// "12.34f32", "1e3", but not "1f32".
    Float { base: Base, empty_exponent: bool },
    /// "'a'", "'\\'", "'''", "';"
    Char { terminated: bool },
    /// ""abc"", ""abc"
    Str { terminated: bool },
}

impl LiteralKind {
    #[must_use]
    pub const fn is_string(self) -> bool {
        !matches!(self, Self::Int { .. } | Self::Float { .. })
    }
    #[must_use]
    pub const fn terminated(self) -> bool {
        use LiteralKind::*;
        match self {
            Int { .. } | Float { .. } => true,
            Char { terminated } | Str { terminated } => terminated,
        }
    }
}

/// Base of numeric literal encoding according to its prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Base {
    /// Literal starts with "0b".
    Binary = 2,
    /// Literal starts with "0o".
    Octal = 8,
    /// Literal doesn't contain a prefix.
    Decimal = 10,
    /// Literal starts with "0x".
    Hexadecimal = 16,
}
