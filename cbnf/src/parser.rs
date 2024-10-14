#![allow(clippy::cast_possible_truncation)]

use crate::{
    lexer::{Base, Cursor, LexKind, LiteralKind, *},
    parser::error::{Error, InvalidLiteral},
    span::{BSpan, TSpan},
    util::*,
    Comment, DocComment, List, Rule, Term,
};

use self::error::ErrorKind;

// TODO: flesh out errors, especially the 'Expected' class of errors

pub mod error;

#[cfg(test)]
mod test;

pub struct Parser<'a> {
    curr: Option<(Lexeme, BSpan)>,
    pub(crate) cursor: Cursor<'a>,
    pub(crate) comments: Vec<Comment>,
    pub(crate) docs: Vec<DocComment>,
    pub(crate) errors: Vec<Error>,
    pub(crate) terms: Vec<Term>,
}

impl<'a> Parser<'a> {
    fn advance(&mut self) -> (Lexeme, BSpan) {
        self.curr.take().unwrap_or_else(|| {
            let token = self.cursor.advance();
            (token, self.span(token))
        })
    }
    fn reverse(&mut self, token: Lexeme) -> Option<(Lexeme, BSpan)> {
        self.curr.replace((token, self.span(token)))
    }
    fn until_non_wc(&mut self) -> (Lexeme, BSpan) {
        loop {
            let (token, span) = self.advance();
            if !self.handle_wc(token) {
                break (token, span);
            }
        }
    }
    fn push_comment(&mut self, style: Option<DocStyle>, content: impl Into<AsBSpan>) {
        let span = self.span(content);
        match style {
            Some(style) => self.docs.push(DocComment(style, span)),
            None => self.comments.push(Comment(span)),
        }
    }
    fn handle_wc(&mut self, token: Lexeme) -> bool {
        if let LineComment { doc_style } = token.kind {
            self.push_comment(doc_style, token);
        }
        if let BlockComment {
            doc_style,
            terminated,
        } = token.kind
        {
            if !terminated {
                self.push_err(Error {
                    span: self.span(token),
                    kind: ErrorKind::Unterminated,
                });
            }
            self.push_comment(doc_style, token);
        }
        matches!(
            token.kind,
            LineComment { .. } | BlockComment { .. } | Whitespace
        )
    }
    fn err_expected(&mut self, span: impl Into<AsBSpan>, expected: impl Into<Box<[LexKind]>>) {
        self.push_err(Error {
            span: self.span(span),
            kind: ErrorKind::Expected(expected.into()),
        });
    }
    fn push_err(&mut self, err: impl Into<Error>) {
        let err: Error = err.into();
        let Some(prev) = self.errors.last_mut() else {
            self.errors.push(err);
            return;
        };
        if let Some(err) = prev.congregate(err) {
            self.errors.push(err);
        }
    }
    #[must_use]
    const fn src(&self) -> &str {
        self.cursor.src()
    }
    #[must_use]
    pub fn slice(&self, span: impl Into<AsBSpan>) -> &str {
        self.span(span).slice(self.src())
    }
    pub fn span(&self, span: impl Into<AsBSpan>) -> BSpan {
        match span.into() {
            AsBSpan::Len(len) => self.token_span(len),
            AsBSpan::Lex(token) => self.token_span(token.len),
            AsBSpan::Span(span) => span,
        }
    }
    #[must_use]
    const fn token_pos(&self) -> u32 {
        self.cursor.lex_pos()
    }
    #[must_use]
    const fn token_span(&self, len: u32) -> BSpan {
        BSpan::new(self.token_pos(), self.token_pos() + len)
    }
}

pub const LIST_EXPECTED: [LexKind; 5] = [OpenParen, Ident, Or, LITERAL, CloseBrace];
pub const RULE_EXPECTED: [LexKind; 1] = [Ident];

impl<'a> Parser<'a> {
    #[must_use]
    pub fn new(input: &'a str) -> Self {
        Self {
            curr: None,
            cursor: Cursor::new(input),
            comments: Vec::new(),
            docs: Vec::new(),
            errors: Vec::new(),
            terms: Vec::new(),
        }
    }

    #[must_use]
    pub fn next_rule(&mut self) -> Option<Rule> {
        let (name, open) = loop {
            let span = self.until_ident()?;
            if let Some(open) = self.rule_opener(span) {
                break (span, open);
            }
        };
        let (close, terms) = self.list(open);
        Some(Rule {
            name,
            expr: Some(List {
                span: (open, close).into(),
                terms,
            }),
            span: name.to(close),
        })
    }

    fn list(&mut self, open: u32) -> (u32, TSpan) {
        let first = self.terms.len() as u32;
        let mut groups = Vec::new();
        let mut ors = Vec::new();
        let (span, eof) = loop {
            let (token, span) = self.until_non_wc();
            match token.kind {
                CloseBrace => break (span, false),
                Eof => break (span, true),
                OpenParen => {
                    groups.push(self.terms.len() as u32);
                    self.terms.push(Term::Group(List::new(
                        span,
                        TSpan::empty(self.terms.len() as u32),
                    )));
                }
                CloseParen if !groups.is_empty() => {
                    self.pop_group(&mut ors, &mut groups, span.to);
                }
                Or => {
                    self.handle_or(&mut ors, &groups);
                    ors.push(self.terms.len() as u32);
                    self.terms.push(Term::Or(List::new(
                        span,
                        TSpan::empty(self.terms.len() as u32),
                    )));
                }
                Ident => self.terms.push(Term::Ident(span)),
                Literal { kind, .. } if kind.is_string() => {
                    if !kind.terminated() {
                        self.push_err(Error {
                            span: self.span(span),
                            kind: ErrorKind::InvalidLit(InvalidLiteral::Unterminated),
                        });
                    }
                    self.terms.push(Term::Literal(span));
                }
                Literal { .. } => self.push_err((InvalidLiteral::Numeric, span)),
                // TODO: also add a CloseParen item to the EXPECTED when there
                // are unclosed groups
                _ => self.err_expected(span, LIST_EXPECTED),
            }
        };
        self.handle_or(&mut ors, &groups);
        self.handle_unclosed(groups, span);
        if eof {
            self.push_err(Error {
                span: span.from(open),
                kind: ErrorKind::UnclosedRule,
            });
        }
        assert!(ors.is_empty(), "or backlog not empty: {ors:#?}");
        (span.to, (first, self.terms.len() as u32).into())
    }

    fn handle_or(&mut self, ors: &mut Vec<u32>, groups: &[u32]) -> bool {
        let or = match (ors.last(), groups.last()) {
            (Some(&o), Some(&g)) if o > g => o,
            (Some(&o), None) => o,
            _ => return false,
        };
        let len = self.terms.len();
        let to = self.terms[len - 1].span().to;
        let Term::Or(list) = &mut self.terms[or as usize] else {
            unreachable!("non 'or' found at index {or}")
        };
        list.terms.to = len as u32;
        list.span.to = to;
        ors.pop();
        true
    }

    fn pop_group(&mut self, ors: &mut Vec<u32>, groups: &mut Vec<u32>, to: u32) {
        let Some(&group) = groups.last() else { return };
        let len = self.terms.len();
        if let Term::Group(list) = &mut self.terms[group as usize] {
            list.terms.to = len as u32;
            list.span.to = to;
        } else {
            unreachable!("group 'or' found at index {group}")
        };
        groups.pop();
        while self.handle_or(ors, groups) {}
        if let Some(Term::Or(or)) = self.terms.get_mut(group.saturating_sub(1) as usize) {
            or.span.to = to;
        }
    }

    fn handle_unclosed(&mut self, groups: Vec<u32>, span: BSpan) {
        for group in groups {
            let err_span;
            let len = self.terms.len();
            if let Term::Group(group) = &mut self.terms[group as usize] {
                group.span.to = span.to;
                group.terms.to = len as u32;
                err_span = group.span;
            } else {
                unreachable!("group 'or' found at index {group}")
            };
            self.push_err(Error {
                span: self.span(err_span),
                kind: ErrorKind::Unterminated,
            });
        }
    }

    fn rule_opener(&mut self, err_span: BSpan) -> Option<u32> {
        let (token, span) = self.until_non_wc();
        if OpenBrace == token.kind {
            Some(span.from)
        } else {
            self.push_err(Error {
                span: err_span,
                kind: ErrorKind::UnopenedRule,
            });
            self.reverse(token);
            None
        }
    }

    fn until_ident(&mut self) -> Option<BSpan> {
        loop {
            let (token, span) = self.until_non_wc();
            match token.kind {
                Ident => break Some(span),
                Eof => return None,
                _ => self.err_expected(token, [Ident]),
            };
        }
    }
}

const LITERAL: LexKind = LexKind::Literal {
    kind: LiteralKind::Int {
        base: Base::Binary,
        empty_int: false,
    },
    suffix_start: 0,
};

#[allow(dead_code)]
const LINE_COMMENT: LexKind = LexKind::LineComment { doc_style: None };
