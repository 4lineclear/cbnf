use crate::lexer;
use crate::span::BSpan;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Error {
    Eof(usize),
    InvalidLit(InvalidLiteral, BSpan),
    Unterminated(BSpan),
    Expected(BSpan, Box<[lexer::LexKind]>),
}

impl From<(InvalidLiteral, BSpan)> for Error {
    fn from(value: (InvalidLiteral, BSpan)) -> Self {
        Self::InvalidLit(value.0, value.1)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InvalidLiteral {
    /// Numeric Literal Found
    Numeric,
    /// Unterminated Literal Found
    Unterminated,
}
