use std::fmt::Display;

use crate::{
    lexer::LexKind,
    parser::{
        error::{Error, InvalidLiteral},
        LIST_EXPECTED, RULE_EXPECTED,
    },
    span::BSpan,
    Cbnf, List, Rule,
};

use pretty_assertions::assert_eq;

// TODO: create better testing system.
//
// TODO: add testing for error cases.
//
// TODO: add fuzzing.

fn cbnf_print(src: &str, cbnf: &Cbnf) -> String {
    let mut out = String::with_capacity(src.len());
    cbnf.rules()
        .iter()
        .for_each(|rule| rule_print(&mut out, rule, cbnf));
    cbnf.rules()
        .iter()
        .flat_map(|r| {
            println!("name: {}", r.name.slice(src));
            println!("rule: {}", r.span.slice(src));
            r.expr.clone()
        })
        .flat_map(|l| {
            println!("expr: {}", l.span.slice(src));
            cbnf.terms(l.terms)
        })
        .for_each(|t| println!("term: {}", t.span().slice(src)));
    out
}

fn rule_print(out: &mut String, rule: &Rule, cbnf: &Cbnf) {
    rule.span.write(out);
    rule.name.write(out);
    let Some(list) = &rule.expr else {
        return;
    };
    list_print(out, list, cbnf);
}

fn list_print(out: &mut String, list: &List, cbnf: &Cbnf) {
    list.span.write(out);
    cbnf.terms(list.terms)
        .iter()
        .for_each(|term| term.span().write(out));
}

impl Display for BSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.from.to_string())?;
        f.write_str("..")?;
        f.write_str(&self.to.to_string())?;
        Ok(())
    }
}

impl BSpan {
    fn write(self, out: &mut String) {
        out.push('(');
        out.push_str(&self.from.to_string());
        out.push_str(", ");
        out.push_str(&self.to.to_string());
        out.push(')');
    }
}

const IDENT_1: [LexKind; 1] = [LexKind::Ident];
const RULE_AFTER_IDENT: [LexKind; 1] = [LexKind::OpenBrace];
const META_AFTER_IDENT: [LexKind; 2] = [LexKind::Semi, LexKind::OpenBrace];

macro_rules! expected {
    ($($exp: ident, $(($a: expr, $b: expr)),*),*) => {
        [$($(
            expected!($exp, $a, $b),
        )*)*]

    };
    ($exp: expr, $a: expr, $b: expr) => {
        Expected(($a, $b).into(), $exp.into())
    };
}

use expected;

macro_rules! numeric {
    ($(($a: expr, $b: expr)),*) => {
        [$(
            numeric !($a, $b),
        )*]

    };
    ($a: expr, $b: expr) => {
        InvalidLit(InvalidLiteral::Numeric, ($a, $b).into())
    };
}

macro_rules! debug {
    ($e:expr) => {
        format!("{:#?}", $e)
    };
}

#[test]
fn empty() {
    let cbnf = Cbnf::parse("yeah { }");
    let out = cbnf_print("yeah { }", &cbnf);
    assert_eq!(out, "(0, 8)(0, 4)(5, 8)");
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}
#[test]
fn short_meta() {
    let cbnf = Cbnf::parse("$yeah;");
    let out = cbnf_print("$yeah;", &cbnf);
    assert_eq!(out, "(0, 5)(0, 5)");
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}
#[test]
fn empty_long_meta() {
    let cbnf = Cbnf::parse("$yeah { }");
    let out = cbnf_print("$yeah { }", &cbnf);
    assert_eq!(out, "(0, 9)(0, 5)(6, 9)");
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}
#[test]
fn strings() {
    let cbnf = Cbnf::parse(r#"yeah { "one" "two" "three" }"#);
    let out = cbnf_print(r#"yeah { "one" "two" "three" }"#, &cbnf);
    assert_eq!(out, "(0, 28)(0, 4)(5, 28)(7, 12)(13, 18)(19, 26)");
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}
#[test]
fn chars() {
    let cbnf = Cbnf::parse("yeah { 'o' 't' 'h' }");
    let out = cbnf_print("yeah { 'o' 't' 'h' }", &cbnf);
    assert_eq!(out, "(0, 20)(0, 4)(5, 20)(7, 10)(11, 14)(15, 18)");
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}
#[test]
fn idents() {
    let cbnf = Cbnf::parse("yeah { one two three }");
    let out = cbnf_print("yeah { one two three }", &cbnf);
    assert_eq!(out, "(0, 22)(0, 4)(5, 22)(7, 10)(11, 14)(15, 20)");
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}
#[test]
fn metas() {
    let cbnf = Cbnf::parse("yeah { $one $two $three }");
    let out = cbnf_print("yeah { $one $two $three }", &cbnf);
    assert_eq!(out, "(0, 25)(0, 4)(5, 25)(7, 11)(12, 16)(17, 23)");
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}
#[test]
fn group() {
    let cbnf = Cbnf::parse("yeah { ( ) }");
    let out = cbnf_print("yeah { ( ) }", &cbnf);
    assert_eq!(out, "(0, 12)(0, 4)(5, 12)(7, 10)");
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}
#[test]
fn mixed() {
    let cbnf = Cbnf::parse(r#"yeah { nil a $b "c" 'd' (a $b "c" 'd') nil }"#);
    let out = cbnf_print(r#"yeah { nil a $b "c" 'd' (a $b "c" 'd') nil }"#, &cbnf);
    assert_eq!(
        out,
        "(0, 44)(0, 4)(5, 44)(7, 10)(11, 12)(13, 15)(16, 19)\
         (20, 23)(24, 38)(25, 26)(27, 29)(30, 33)(34, 37)(39, 42)"
    );
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}
#[test]
fn cbnf() {
    let cbnf = Cbnf::parse(include_str!("../../../cbnf.cbnf"));
    let out = cbnf_print(include_str!("../../../cbnf.cbnf"), &cbnf);
    assert_eq!(
        out,
        "\
            (126, 157)(126, 133)(139, 157)(141, 144)(145, 150)(151, 155)\
            (158, 192)(158, 164)(170, 192)(172, 176)(177, 183)(184, 186)\
            (187, 190)(193, 248)(193, 197)(203, 248)(205, 221)(206, 212)\
            (213, 215)(216, 220)(222, 225)(226, 230)(231, 234)(235, 237)\
            (238, 242)(243, 246)(249, 281)(249, 253)(259, 281)(261, 265)\
            (266, 279)(267, 271)(272, 274)(275, 278)(282, 331)(282, 286)\
            (292, 331)(294, 301)(302, 304)(305, 311)(312, 314)(315, 319)\
            (320, 322)(323, 328)(332, 356)(332, 336)(342, 356)(344, 347)\
            (348, 354)(357, 384)(357, 362)(368, 384)(370, 373)(374, 378)\
            (379, 382)(386, 415)(386, 393)(399, 415)(401, 404)(405, 409)\
            (410, 413)(417, 422)(417, 422)(440, 445)(440, 445)(468, 474)\
            (468, 474)(523, 527)(523, 527)"
    );
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}

use Error::*;

#[test]
fn unclosed_rule() {
    let cbnf = Cbnf::parse("yeah { ");
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!([Expected((7, 7).into(), [LexKind::CloseBrace].into())]);
    assert_eq!(actual, expected);
}
#[test]
fn unclosed_group() {
    let cbnf = Cbnf::parse("yeah { ( }");
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!([Unterminated((7, 10).into())]);
    assert_eq!(actual, expected);
}
#[test]
fn unclosed_rule_group() {
    let cbnf = Cbnf::parse("yeah { ( ");
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!([
        Unterminated((7, 9).into()),
        Expected((9, 9).into(), [LexKind::CloseBrace].into())
    ]);
    assert_eq!(actual, expected);
}
#[test]
fn int_or_float() {
    let cbnf = Cbnf::parse("yeah { 12_u8 0o100 0b120i99 1f32 12.34f32 1e3 }");
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!(numeric![
        (7, 12),
        (13, 18),
        (19, 27),
        (28, 32),
        (33, 41),
        (42, 45)
    ]);
    assert_eq!(actual, expected);
}
#[test]
fn not_rule_or_ident() {
    let cbnf = Cbnf::parse("yeah { \\ //\\@# \\ //\\\n}\n\\ //\\@# \\ //\\\n");
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!(expected![
        LIST_EXPECTED,
        (7, 8),
        (9, 13),
        RULE_EXPECTED,
        (23, 24),
        (25, 29)
    ]);
    assert_eq!(actual, expected);
}
#[test]
fn dollar_repeat() {
    let cbnf = Cbnf::parse("$ $");
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!(expected![IDENT_1, (2, 3)]);
    assert_eq!(actual, expected);
}
#[test]
fn empty_dollar() {
    let cbnf = Cbnf::parse("$yeah { $ }");
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!(expected![IDENT_1, (10, 11)]);
    assert_eq!(actual, expected);
}
#[test]
fn dolllar_after_rule() {
    let cbnf = Cbnf::parse("$yeah $");
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!(expected![META_AFTER_IDENT, (6, 7)]);
    assert_eq!(actual, expected);
}
#[test]
fn rule_after_ident() {
    let cbnf = Cbnf::parse("yeah $");
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!(expected![RULE_AFTER_IDENT, (5, 6)]);
    assert_eq!(actual, expected);
}
#[test]
fn unterm_char() {
    let cbnf = Cbnf::parse("yeah { '\n}");
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!([InvalidLit(InvalidLiteral::Unterminated, (7, 8).into())]);
    assert_eq!(actual, expected);
}
#[test]
fn unterm_string() {
    let cbnf = Cbnf::parse("yeah { \"}");
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!([
        InvalidLit(InvalidLiteral::Unterminated, (7, 9).into()),
        Expected((9, 9).into(), [LexKind::CloseBrace].into())
    ]);
    assert_eq!(actual, expected);
}
