use crate::lexer;
use crate::span::BSpan;

// TODO: consider having expected be a nested list of Lexemes

// TODO: add more values, etc, to errors

// TODO: create ErrorKind && add span to error

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Error {
    pub span: BSpan,
    pub kind: ErrorKind,
}

// TODO: UnnamedRule

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorKind {
    InvalidLit(InvalidLiteral),
    UnclosedRule,
    UnopenedRule,
    Unterminated,
    Expected(Box<[lexer::LexKind]>),
}

impl From<(BSpan, ErrorKind)> for Error {
    fn from((span, kind): (BSpan, ErrorKind)) -> Self {
        Self { span, kind }
    }
}

use ErrorKind::*;

impl Error {
    /// `Some(error)` means a non congregated error
    #[must_use]
    pub const fn span(&self) -> BSpan {
        self.span
    }

    #[must_use]
    pub fn message(&self) -> String {
        match &self.kind {
            InvalidLit(lit) => match lit {
                InvalidLiteral::Numeric => "Numbers not allowed",
                InvalidLiteral::Unterminated => "Unterminated terminal found",
            }
            .into(),
            UnclosedRule => "Unclosed rule found".into(),
            UnopenedRule => "Unopened rule found".into(),
            Unterminated => "Group not terminated".into(),
            Expected(acc) => {
                if acc.len() == 0 {
                    return "Token not expected".into();
                }
                let mut o = String::from("Token not expected, expected one of: [ ");
                o.push_str(acc[0].name());
                for l in &acc[1..] {
                    o.push_str(", ");
                    o.push_str(l.name());
                }
                o.push_str(" ]");
                o
            }
        }
    }
    /// `Some(error)` means a non congregated error
    pub fn congregate(&mut self, other: Self) -> Option<Self> {
        let Expected(exp) = &mut self.kind else {
            return Some(other);
        };
        let Expected(other_exp) = &other.kind else {
            return Some(other);
        };
        #[expect(clippy::suspicious_operation_groupings)]
        if self.span.to != other.span.from || exp != other_exp {
            return Some(other);
        }
        self.span.to = other.span.to;
        None
    }
}

impl From<(InvalidLiteral, BSpan)> for Error {
    fn from((kind, span): (InvalidLiteral, BSpan)) -> Self {
        Self {
            span,
            kind: InvalidLit(kind),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum InvalidLiteral {
    /// Numeric Literal Found
    Numeric,
    /// Unterminated Literal Found
    Unterminated,
}
