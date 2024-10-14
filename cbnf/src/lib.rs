#![allow(clippy::option_option)]

use indexmap::IndexMap;

use crate::{
    parser::{error::Error, Parser},
    span::{BSpan, TSpan},
};

pub use indexmap;

// TODO: consider using string interning (or a ton of refs) to
// allow for partial compilation

pub mod lexer;
pub mod parser;
pub mod span;
pub mod util;

// TODO: consider renaming to convenient bnf

// TODO: consider moving to c or rust style comments
// if that is done, add raw strings back in too.

// TODO: add test coverage

// TODO: add whitespace aware syntax
// something such as a ',' denoting an immediate token

// TODO: add another byte

// TODO: add regex && UnicodeSet notation

// TODO: consider moving back to an AST with Rc<T> everywhere to allow for
// partial recompiling

// TODO: generate a parser generator from this file that is tested by
// parsing this file

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
    // TODO: create a double key map where either a BSpan or a String can
    // index the map
    pub rules: IndexMap<String, Rule>,
    pub extras: Vec<Rule>,
    pub comments: Vec<Comment>,
    pub docs: Vec<DocComment>,
    pub errors: Vec<Error>,
    pub terms: Vec<Term>,
}

impl From<Parser<'_>> for Cbnf {
    fn from(mut value: Parser<'_>) -> Self {
        let mut extras = Vec::new();
        let mut rules = IndexMap::new();
        while let Some(rule) = value.next_rule() {
            if rules.contains_key(value.slice(rule.name)) {
                extras.push(rule);
            } else {
                rules.insert(value.slice(rule.name).to_owned(), rule);
            }
        }
        Self {
            rules,
            extras,
            comments: value.comments,
            docs: value.docs,
            errors: value.errors,
            terms: value.terms,
        }
    }
}

impl Cbnf {
    #[must_use]
    pub const fn rules(&self) -> &IndexMap<String, Rule> {
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
    pub fn terms(&self) -> &[Term] {
        &self.terms
    }
    #[must_use]
    pub fn terms_at(&self, span: TSpan) -> &[Term] {
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
            Literal(span) | Ident(span) => *span,
            Or(list) | Group(list) => list.span,
        }
    }
    #[must_use]
    pub const fn terms(&self) -> Option<TSpan> {
        use Term::*;
        match self {
            Or(list) | Group(list) => Some(list.terms),
            Literal(_) | Ident(_) => None,
        }
    }
}
