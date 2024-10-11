//! pre-lexing
//!
//! At this level, error handling and keywords don't exist
#![allow(clippy::unnested_or_patterns)]

use unicode_properties::UnicodeEmoji;

use super::util::{is_id_continue, is_id_start, is_whitespace};

#[cfg(test)]
pub mod test;

pub mod cursor;
pub mod token;
pub mod unescape;

pub use cursor::{Cursor, EOF_CHAR};
pub use token::{
    Base, DocStyle,
    LexKind::{self, *},
    Lexeme,
    LiteralKind::{self, *},
};

macro_rules! dassert {
    ($($t:tt)*) => {
        #[cfg(debug_assertions)]
        assert!($($t)*);
    };
}

pub fn tokenize(input: &str) -> impl Iterator<Item = Lexeme> + '_ {
    Cursor::new(input)
}

#[must_use]
pub fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|start| is_id_start(start) && chars.all(is_id_continue))
}

impl Cursor<'_> {
    /// Parses a token from the input string.
    pub fn advance(&mut self) -> Lexeme {
        self.token_pos = self.pos();
        let Some(first_char) = self.bump() else {
            return Lexeme::new(LexKind::Eof, 0);
        };
        let token_kind = match first_char {
            // Whitespace sequence.
            c if is_whitespace(c) => self.whitespace(),

            // Identifier (this should be checked after other variant that can
            // start as identifier).
            c if is_id_start(c) => self.ident(),

            // Numeric literal.
            c @ '0'..='9' => {
                let literal_kind = self.number(c);
                let suffix_start = self.pos_within_token();
                self.eat_literal_suffix();
                LexKind::Literal {
                    kind: literal_kind,
                    suffix_start,
                }
            }

            // comment or block comment.
            '#' => self.line_comment(),

            // One-symbol tokens.
            ';' => Semi,
            ',' => Comma,
            '.' => Dot,
            '(' => OpenParen,
            ')' => CloseParen,
            '{' => OpenBrace,
            '}' => CloseBrace,
            '[' => OpenBracket,
            ']' => CloseBracket,
            '@' => At,
            '~' => Tilde,
            '?' => Question,
            ':' => Colon,
            '$' => Dollar,
            '=' => Eq,
            '!' => Bang,
            '<' => Lt,
            '>' => Gt,
            '-' => Minus,
            '&' => And,
            '|' => Or,
            '+' => Plus,
            '*' => Star,
            '^' => Caret,
            '%' => Percent,

            // Character literal.
            '\'' => self.char(),

            // String literal.
            '"' => self.string(),
            // Identifier starting with an emoji. Only lexed for graceful error recovery.
            c if !c.is_ascii() && c.is_emoji_char() => self.fake_ident(),
            _ => Unknown,
        };
        let res = Lexeme::new(token_kind, self.pos_within_token());
        self.reset_pos_within_token();
        res
    }

    fn line_comment(&mut self) -> LexKind {
        dassert!(self.prev() == '#');

        let doc_style = match self.first() {
            // `#!` is an inner line doc comment.
            '!' => Some(DocStyle::Inner),
            // `###` (more than 2 slashes) is not considered a doc comment.
            ':' if self.second() != ':' => Some(DocStyle::Outer),
            _ => None,
        };

        self.eat_while(|c| c != '\n');
        LineComment { doc_style }
    }

    fn whitespace(&mut self) -> LexKind {
        dassert!(is_whitespace(self.prev()));
        self.eat_while(is_whitespace);
        Whitespace
    }

    fn ident(&mut self) -> LexKind {
        dassert!(is_id_start(self.prev()));
        // Start is already eaten, eat the rest of identifier.
        self.eat_while(is_id_continue);
        // Known prefixes must have been handled earlier. So if
        // we see a prefix here, it is definitely an unknown prefix.
        match self.first() {
            c if !c.is_ascii() && c.is_emoji_char() => self.fake_ident(),
            _ => Ident,
        }
    }

    fn fake_ident(&mut self) -> LexKind {
        // Start is already eaten, eat the rest of identifier.
        self.eat_while(|c| {
            unicode_ident::is_xid_continue(c)
                || (!c.is_ascii() && c.is_emoji_char())
                || c == '\u{200d}'
        });

        InvalidIdent
    }

    pub fn string(&mut self) -> LexKind {
        let terminated = self.double_quoted_string();
        let suffix_start = self.pos_within_token();
        if terminated {
            self.eat_literal_suffix();
        }
        let kind = Str { terminated };
        Literal { kind, suffix_start }
    }

    fn number(&mut self, first_digit: char) -> LiteralKind {
        dassert!('0' <= self.prev() && self.prev() <= '9');
        let mut base = Base::Decimal;
        if first_digit == '0' {
            // Attempt to parse encoding base.
            match self.first() {
                'b' => {
                    base = Base::Binary;
                    self.bump();
                    if !self.eat_decimal_digits() {
                        return Int {
                            base,
                            empty_int: true,
                        };
                    }
                }
                'o' => {
                    base = Base::Octal;
                    self.bump();
                    if !self.eat_decimal_digits() {
                        return Int {
                            base,
                            empty_int: true,
                        };
                    }
                }
                'x' => {
                    base = Base::Hexadecimal;
                    self.bump();
                    if !self.eat_hexadecimal_digits() {
                        return Int {
                            base,
                            empty_int: true,
                        };
                    }
                }
                // Not a base prefix; consume additional digits.
                '0'..='9' | '_' => {
                    self.eat_decimal_digits();
                }

                // Also not a base prefix; nothing more to do here.
                '.' | 'e' | 'E' => {}

                // Just a 0.
                _ => {
                    return Int {
                        base,
                        empty_int: false,
                    }
                }
            }
        } else {
            // No base prefix, parse number in the usual way.
            self.eat_decimal_digits();
        };

        match self.first() {
            // Don't be greedy if this is actually an
            // integer literal followed by field/method access or a range pattern
            // (`0..2` and `12.foo()`)
            '.' if self.second() != '.' && !is_id_start(self.second()) => {
                // might have stuff after the ., and if it does, it needs to start
                // with a number
                self.bump();
                let mut empty_exponent = false;
                if self.first().is_ascii_digit() {
                    self.eat_decimal_digits();
                    match self.first() {
                        'e' | 'E' => {
                            self.bump();
                            empty_exponent = !self.eat_float_exponent();
                        }
                        _ => (),
                    }
                }
                Float {
                    base,
                    empty_exponent,
                }
            }
            'e' | 'E' => {
                self.bump();
                let empty_exponent = !self.eat_float_exponent();
                Float {
                    base,
                    empty_exponent,
                }
            }
            _ => Int {
                base,
                empty_int: false,
            },
        }
    }

    fn char(&mut self) -> LexKind {
        dassert!(self.prev() == '\'');
        let terminated = self.single_quoted_string();
        let suffix_start = self.pos_within_token();
        if terminated {
            self.eat_literal_suffix();
        }
        let kind = Char { terminated };
        Literal { kind, suffix_start }
    }

    fn single_quoted_string(&mut self) -> bool {
        dassert!(self.prev() == '\'');
        // Check if it's a one-symbol literal.
        if self.second() == '\'' && self.first() != '\\' {
            self.bump();
            self.bump();
            return true;
        }

        // Literal has more than one symbol.

        // Parse until either quotes are terminated or error is detected.
        loop {
            match self.first() {
                // Quotes are terminated, finish parsing.
                '\'' => {
                    self.bump();
                    return true;
                }
                // Probably beginning of the comment, which we don't want to include
                // to the error report.
                '/' => break,
                // Newline without following '\'' means unclosed quote, stop parsing.
                '\n' if self.second() != '\'' => break,
                // End of file, stop parsing.
                EOF_CHAR if self.is_eof() => break,
                // Escaped slash is considered one character, so bump twice.
                '\\' => {
                    self.bump();
                    self.bump();
                }
                // Skip the character.
                _ => {
                    self.bump();
                }
            }
        }
        // String was not terminated.
        false
    }

    /// Eats double-quoted string and returns true
    /// if string is terminated.
    fn double_quoted_string(&mut self) -> bool {
        dassert!(self.prev() == '"');
        while let Some(c) = self.bump() {
            match c {
                '"' => {
                    return true;
                }
                '\\' if self.first() == '\\' || self.first() == '"' => {
                    // Bump again to skip escaped character.
                    self.bump();
                }
                _ => (),
            }
        }
        // End of file reached.
        false
    }

    fn eat_decimal_digits(&mut self) -> bool {
        let mut has_digits = false;
        loop {
            match self.first() {
                '_' => {
                    self.bump();
                }
                '0'..='9' => {
                    has_digits = true;
                    self.bump();
                }
                _ => break,
            }
        }
        has_digits
    }

    fn eat_hexadecimal_digits(&mut self) -> bool {
        let mut has_digits = false;
        loop {
            match self.first() {
                '_' => {
                    self.bump();
                }
                '0'..='9' | 'a'..='f' | 'A'..='F' => {
                    has_digits = true;
                    self.bump();
                }
                _ => break,
            }
        }
        has_digits
    }

    /// Eats the float exponent. Returns true if at least one digit was met,
    /// and returns false otherwise.
    fn eat_float_exponent(&mut self) -> bool {
        dassert!(self.prev() == 'e' || self.prev() == 'E');
        if self.first() == '-' || self.first() == '+' {
            self.bump();
        }
        self.eat_decimal_digits()
    }

    // Eats the suffix of the literal, e.g. "u8".
    fn eat_literal_suffix(&mut self) {
        self.eat_identifier();
    }

    // Eats the identifier. Note: succeeds on `_`, which isn't a valid
    // identifier.
    fn eat_identifier(&mut self) {
        if !is_id_start(self.first()) {
            return;
        }
        self.bump();

        self.eat_while(is_id_continue);
    }
}

impl Iterator for Cursor<'_> {
    type Item = Lexeme;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.advance();
        (token.kind != LexKind::Eof).then_some(token)
    }
}
