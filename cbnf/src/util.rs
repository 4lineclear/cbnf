use unicode_normalization::{is_nfc_quick, IsNormalized, UnicodeNormalization};

use crate::lexer::Lexeme;
use crate::span::BSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AsStr<'a> {
    Span(AsBSpan),
    Str(&'a str),
}

impl<'a> From<&'a str> for AsStr<'a> {
    fn from(value: &'a str) -> Self {
        Self::Str(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AsBSpan {
    // Current span used as start
    Len(u32),
    Lex(Lexeme),
    // Uses given
    Span(BSpan),
}

macro_rules! impl_from {
    ($($ty:ty, $name:ident),*) => { $(
        impl From<$ty> for AsBSpan {
            fn from(value: $ty) -> Self {
                Self::$name(value)
            }
        }
        impl<'a> From<$ty> for AsStr<'a> {
            fn from(value: $ty) -> Self {
                Self::Span(AsBSpan::$name(value))
            }
        }
    )*};
}

impl_from!(u32, Len, Lexeme, Lex, BSpan, Span);

#[must_use]
pub const fn is_whitespace(c: char) -> bool {
    matches!(
        c,
        // Usual ASCII suspects
        '\u{0009}'   // \t
        | '\u{000A}' // \n
        | '\u{000B}' // vertical tab
        | '\u{000C}' // form feed
        | '\u{000D}' // \r
        | '\u{0020}' // space

        // NEXT LINE from latin1
        | '\u{0085}'

        // Bidi markers
        | '\u{200E}' // LEFT-TO-RIGHT MARK
        | '\u{200F}' // RIGHT-TO-LEFT MARK

        // Dedicated whitespace characters from Unicode
        | '\u{2028}' // LINE SEPARATOR
        | '\u{2029}' // PARAGRAPH SEPARATOR
    )
}

#[must_use]
pub fn valid_id(s: &str) -> bool {
    s.chars().next().is_some_and(is_id_start) && s[1..].chars().all(is_id_continue)
}

#[must_use]
pub fn is_id_start(c: char) -> bool {
    c == '_' || unicode_ident::is_xid_start(c)
}

#[must_use]
pub fn is_id_continue(c: char) -> bool {
    unicode_ident::is_xid_continue(c)
}

#[must_use]
pub fn nfc_normalize(string: &str) -> String {
    match is_nfc_quick(string.chars()) {
        IsNormalized::Yes => string.to_owned(),
        _ => string.chars().nfc().collect::<String>(),
    }
}
