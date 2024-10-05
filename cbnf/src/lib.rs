#![allow(clippy::option_option)]
use crate::{
    parser::{error::Error, Parser},
    span::{BSpan, TSpan},
};

pub mod lexer;
pub mod parser;
pub mod span;
pub mod util;

// TODO: resolve rule names.
//
// TODO: add test coverage
//
// TODO: test multi or

#[derive(Clone, Debug)]
pub struct Comment(BSpan);

impl Comment {
    #[must_use]
    pub const fn span(&self) -> BSpan {
        self.0
    }
}

#[derive(Clone, Debug)]
pub struct DocComment(DocStyle, BSpan);

pub use crate::lexer::DocStyle;

impl DocComment {
    #[must_use]
    pub const fn style(&self) -> DocStyle {
        self.0
    }
    #[must_use]
    pub const fn span(&self) -> BSpan {
        self.1
    }
}

/// Complex Bachus-Naur Form
#[derive(Default, Clone, Debug)]
pub struct Cbnf {
    // TODO: consider having this be a hashmap
    rules: Vec<Rule>,
    comments: Vec<Comment>,
    docs: Vec<DocComment>,
    errors: Vec<Error>,
    terms: Vec<Term>,
}

impl From<Parser<'_>> for Cbnf {
    fn from(mut value: Parser<'_>) -> Self {
        let rules = core::iter::from_fn(|| value.next_rule()).collect();
        Self {
            rules,
            comments: value.comments,
            docs: value.docs,
            errors: value.errors,
            terms: value.terms,
        }
    }
}

impl Cbnf {
    #[must_use]
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }
    #[must_use]
    pub fn comments(&self) -> &[Comment] {
        &self.comments
    }
    #[must_use]
    pub fn docs(&self) -> &[DocComment] {
        &self.docs
    }
    #[must_use]
    pub fn errors(&self) -> &[Error] {
        &self.errors
    }
    #[must_use]
    pub fn parse(input: &str) -> Self {
        Self::from(Parser::new(input))
    }
    #[must_use]
    pub fn terms(&self, span: TSpan) -> &[Term] {
        &self.terms[span.range()]
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Rule {
    pub name: BSpan,
    pub expr: Option<List>,
    /// The span of the entire `Rule`
    ///
    /// This should extend either to the closing brace or semicolon
    pub span: BSpan,
}

/// An list is a set of Terms seperated by whitespace
#[derive(Debug, Default, Clone, Copy)]
pub struct List {
    span: BSpan,
    terms: TSpan,
}

impl List {
    pub(crate) const fn new(span: BSpan, terms: TSpan) -> Self {
        Self { span, terms }
    }

    #[must_use]
    pub const fn span(&self) -> BSpan {
        self.span
    }

    #[must_use]
    pub const fn terms(&self) -> TSpan {
        self.terms
    }
}

/// A single item within a list
#[derive(Debug, Clone, Copy)]
pub enum Term {
    /// ..
    Ident(BSpan),
    /// ".."
    Literal(BSpan),
    /// $ ..
    Meta(BSpan),
    /// Or
    Or(List),
    /// ( .. )
    Group(List),
}

impl Term {
    #[must_use]
    pub const fn span(&self) -> BSpan {
        use Term::*;
        match self {
            Literal(span) | Ident(span) | Meta(span) => *span,
            Or(list) | Group(list) => list.span,
        }
    }
}
