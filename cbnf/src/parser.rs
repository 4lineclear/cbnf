use crate::{
    lexer::{Base, Cursor, LexKind, LiteralKind, *},
    span::BSpan,
    util::*,
    Comment, Delim, Expression, List, Rule, Term,
};

pub mod error;
#[cfg(test)]
mod test;

pub struct Parser<'a> {
    cursor: Cursor<'a>,
    comments: Vec<Comment>,
    errors: Vec<Error>,
}

impl<'a> Parser<'a> {
    fn err_eof(&mut self) {
        self.push_err(Error::Eof(self.token_pos()));
    }

    fn lex_non_wc(&mut self) -> Option<Lexeme> {
        let token = self.cursor.advance();
        (!self.filter_comment_or_whitespace(token)).then_some(token)
    }
    fn push_comment(&mut self, style: Option<DocStyle>, content: impl Into<AsBSpan>) {
        self.comments.push(Comment {
            style,
            content: self.span(content),
        });
    }

    // TODO: consider also having a flag for parsing when there is a doc comment
    fn filter_comment_or_whitespace(&mut self, token: Lexeme) -> bool {
        if let LineComment { doc_style } = token.kind {
            self.push_comment(doc_style, token.len);
        }
        matches!(token.kind, LineComment { .. } | Whitespace)
    }

    fn err_unterminated(&mut self, span: impl Into<AsBSpan>) {
        self.push_err(Error::Unterminated(self.span(span)));
    }

    fn err_expected(&mut self, span: impl Into<AsBSpan>, expected: impl Into<Box<[LexKind]>>) {
        self.push_err(Error::Expected(self.span(span), expected.into()));
    }

    pub fn push_err(&mut self, err: impl Into<Error>) {
        self.errors.push(err.into());
    }
    #[must_use]
    fn src(&self) -> &str {
        self.cursor.src()
    }
    #[must_use]
    pub fn slice(&self, span: impl Into<AsBSpan>) -> &str {
        self.span(span).slice(self.src())
    }
    #[must_use]
    pub fn symbol(&self, span: impl Into<AsBSpan>) -> Symbol {
        self.slice(span).into()
    }
    pub fn span(&self, span: impl Into<AsBSpan>) -> BSpan {
        match span.into() {
            AsBSpan::Len(len) => self.token_span(len),
            AsBSpan::Lex(token) => self.token_span(token.len),
            AsBSpan::Span(span) => span,
        }
    }
    #[must_use]
    pub const fn token_pos(&self) -> usize {
        self.cursor.lex_pos()
    }
    #[must_use]
    const fn token_span(&self, len: usize) -> BSpan {
        BSpan::new(self.token_pos(), self.token_pos() + len)
    }
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let cursor = Cursor::new(input);
        let comments = Vec::new();
        let errors = Vec::new();
        Self {
            cursor,
            comments,
            errors,
        }
    }

    pub fn parts(self) -> (Cursor<'a>, Vec<Comment>, Vec<Error>) {
        (self.cursor, self.comments, self.errors)
    }

    pub fn next_rule(&mut self) -> Option<Rule> {
        let (name, open_brace) = match self.until_dollar_or_ident() {
            X(()) => return None,
            Y(pos) => {
                let ident = self.ident().opt()?;
                let semi = self.semi_or_open_brace().opt()?;
                let span = ident.from(pos);
                if semi {
                    return Some(Rule {
                        name: span,
                        expr: None,
                        span,
                    });
                }
                (span, self.token_pos())
            }
            Z(span) => (span, self.open_brace().opt().map(|()| self.token_pos())?),
        };
        let expr = self.expr(open_brace, 0).opt()?;
        let span = name.to(expr.span.to);
        Some(Rule {
            name,
            expr: Some(expr),
            span,
        })
    }

    fn expr(&mut self, open: usize, parens: usize) -> Filtered<Expression> {
        use LexKind::*;
        const EXPECTED: [LexKind; 5] = [OpenParen, Ident, RawIdent, LITERAL, CloseBrace];
        let mut parts = Vec::new();
        let mut delim = Delim::Or;
        let mut list = List::new(BSpan::new(open, open));
        let (span, paren) = loop {
            let Some(token) = self.lex_non_wc() else {
                continue;
            };
            let span = self.span(token);
            match token.kind {
                OpenParen => {
                    let expr = self.expr(self.token_pos(), parens + 1);
                    match expr {
                        Correct(expr) => {
                            list.terms.push(Term::Group(expr));
                        }
                        Other(_) => (),
                        InputEnd => return Filtered::InputEnd,
                    }
                }
                CloseParen if parens != 0 => break (span, true),
                Ident | RawIdent if self.slice(span) == "or" => {
                    list.reset_list(BSpan::new(open, span.to));
                    parts.push((delim, list));
                    list = List::new(BSpan::new(open, open));
                    delim = Delim::Or;
                }
                Ident | RawIdent => list.terms.push(Term::Ident(span)),
                Dollar => {
                    if let Correct(ident) = self.ident() {
                        list.terms.push(Term::Meta(span.to(ident.to)));
                    }
                }
                Literal { kind, .. } if !kind.is_string() || !kind.terminated() => {
                    self.push_err((InvalidLiteral::Numeric, span))
                }
                Literal { .. } => list.terms.push(Term::Literal(span)),
                CloseBrace => break (span, false),
                _ => return self.handle_other(token, EXPECTED),
            }
        };
        if !paren && parens != 0 {
            self.err_unterminated(span);
        }
        list.reset_list(BSpan::new(open, span.to));
        parts.push((Delim::Or, list));
        let close = self.token_pos();
        let span = BSpan::new(open, close + 1);
        Expression { span, parts }.into()
    }

    fn ident(&mut self) -> Filtered<BSpan> {
        look_for!(match (self, token, [Ident, RawIdent]) {
            Ident | RawIdent => break self.span(token).into(),
        })
    }

    /// `X` = Eof, `Y` = Dollar, `Z` = Ident
    ///
    /// Runs until one of the above is found
    fn until_dollar_or_ident(&mut self) -> Either3<(), usize, BSpan> {
        loop {
            match look_for!(match (self, token, [Dollar, Ident, RawIdent]) {
                Dollar => break Y(self.token_pos()).into(),
                Ident | RawIdent => break Z(self.span(token)).into(),
                Eof => break InputEnd,
            }) {
                InputEnd => return X(()),
                Correct(span) => break span,
                Other(_) => (),
            }
        }
    }
    /// `A` = Semi, `B` = Brace
    fn open_brace(&mut self) -> Filtered<()> {
        look_for!(match (self, token, [OpenBrace]) {
            OpenBrace => break ().into(),
        })
    }

    /// `true` if semi else `false`
    fn semi_or_open_brace(&mut self) -> Filtered<bool> {
        look_for!(match (self, token, [Semi, OpenBrace]) {
            Semi => break true.into(),
            OpenBrace => break false.into(),
        })
    }

    fn handle_other<T>(
        &mut self,
        token: Lexeme,
        expected: impl Into<Box<[LexKind]>>,
    ) -> Filtered<T> {
        match token.kind {
            Eof => {
                self.err_eof();
                return InputEnd;
            }
            _ => {
                self.err_expected(token, expected);
                return Other(token);
            }
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

#[allow(unused)]
use Either::*;
use Either3::*;
use Filtered::*;

macro_rules! look_for {
    (match ($this:ident, $lex:ident, $expected: expr $(, $span:ident)?) {
        $($matcher:pat $(if $pred:expr)? => $result:expr $(,)?)*
    }) => {{
        use LexKind::*;
        loop {
            let Some($lex) = $this.lex_non_wc() else {
                continue;
            };
            #[allow(unused)]
            $(let $span = $this.span($lex);)?
            match $lex.kind {
                $($matcher $(if $pred)? => $result,)*
                #[allow(unreachable_patterns)]
                _ => break $this.handle_other($lex, $expected)
            }
        }
    }};
}

use look_for;

use self::error::{Error, InvalidLiteral};
