use std::fmt::Display;

use crate::{
    lexer::LexKind,
    parser::{
        error::{Error, ErrorKind::*, InvalidLiteral},
        LIST_EXPECTED, RULE_EXPECTED,
    },
    span::{BSpan, TSpan},
    Cbnf, List, Rule,
};

use pretty_assertions::assert_eq;

// TODO: create better testing system.
// hopefully break tests (bytes,  terms) into multiple strings at least

// TODO: add more testing for error cases.

// TODO: add fuzzing.

fn cbnf_print(src: &str, cbnf: &Cbnf) -> String {
    let mut bytes = String::new();
    let mut terms = String::new();
    cbnf.rules()
        .iter()
        .for_each(|(_, rule)| rule_print(&mut bytes, &mut terms, rule, cbnf));
    cbnf.rules()
        .iter()
        .flat_map(|(name, r)| {
            println!("name: {name}");
            println!("rule: {}", r.span.slice(src));
            r.expr.clone()
        })
        .flat_map(|l| {
            println!("expr: {}", l.span.slice(src));
            cbnf.terms_at(l.terms)
        })
        .for_each(|t| println!("term: {}", t.span().slice(src)));
    bytes + &terms
}

fn rule_print(bytes: &mut String, terms: &mut String, rule: &Rule, cbnf: &Cbnf) {
    rule.span.write(bytes);
    rule.name.write(bytes);
    let Some(list) = &rule.expr else {
        return;
    };
    list_print(bytes, terms, list, cbnf);
}

fn list_print(bytes: &mut String, terms: &mut String, list: &List, cbnf: &Cbnf) {
    list.span.write(bytes);
    list.terms.write(terms);
    cbnf.terms_at(list.terms).iter().for_each(|term| {
        term.span().write(bytes);
        term.terms().inspect(|t| t.write(terms));
    });
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

impl TSpan {
    fn write(self, out: &mut String) {
        out.push('[');
        out.push_str(&self.from.to_string());
        out.push_str(", ");
        out.push_str(&self.to.to_string());
        out.push(']');
    }
}

macro_rules! expected {
    ($($exp: ident, $(($a: expr, $b: expr)),*),*) => {
        [$($(
            expected!($exp, $a, $b),
        )*)*]

    };
    ($exp: expr, $a: expr, $b: expr) => {
        Error { span: ($a, $b).into(), kind: Expected($exp.into()) }
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
        Error { span: ($a, $b).into(), kind: InvalidLit(InvalidLiteral::Numeric) }
    };
}

macro_rules! debug {
    ($e:expr) => {
        format!("{:#?}", $e)
    };
}

#[test]
fn empty() {
    let src = "yeah { }";
    let cbnf = Cbnf::parse(src);
    let out = cbnf_print(src, &cbnf);
    assert_eq!(out, "(0, 8)(0, 4)(5, 8)[0, 0]");
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}
#[test]
fn strings() {
    let src = r#"yeah { "one" "two" "three" }"#;
    let cbnf = Cbnf::parse(src);
    let out = cbnf_print(src, &cbnf);
    assert_eq!(out, "(0, 28)(0, 4)(5, 28)(7, 12)(13, 18)(19, 26)[0, 3]");
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}
#[test]
fn chars() {
    let src = "yeah { 'o' 't' 'h' }";
    let cbnf = Cbnf::parse(src);
    let out = cbnf_print(src, &cbnf);
    assert_eq!(out, "(0, 20)(0, 4)(5, 20)(7, 10)(11, 14)(15, 18)[0, 3]");
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}
#[test]
fn idents() {
    let src = "yeah { one two three }";
    let cbnf = Cbnf::parse(src);
    let out = cbnf_print(src, &cbnf);
    assert_eq!(out, "(0, 22)(0, 4)(5, 22)(7, 10)(11, 14)(15, 20)[0, 3]");
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}
#[test]
fn group() {
    let src = "yeah { ( ) }";
    let cbnf = Cbnf::parse(src);
    let out = cbnf_print(src, &cbnf);
    assert_eq!(out, "(0, 12)(0, 4)(5, 12)(7, 10)[0, 1][0, 1]");
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}
#[test]
fn mixed() {
    let src = r#"yeah { nil a bb "c" 'd' (a bb "c" 'd') nil }"#;
    let cbnf = Cbnf::parse(src);
    let out = cbnf_print(src, &cbnf);
    assert_eq!(
        out,
        "(0, 44)(0, 4)(5, 44)(7, 10)(11, 12)(13, 15)(16, 19)\
         (20, 23)(24, 38)(25, 26)(27, 29)(30, 33)(34, 37)(39, 42)\
         [0, 11][5, 10]"
    );
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}
#[test]
fn single_or() {
    let src = r#"yeah { a bb "c" 'd' | a bb "c" 'd' }"#;
    let cbnf = Cbnf::parse(src);
    let out = cbnf_print(src, &cbnf);
    assert_eq!(
        out,
        "(0, 36)(0, 4)(5, 36)(7, 8)(9, 11)(12, 15)(16, 19)\
         (20, 34)(22, 23)(24, 26)(27, 30)(31, 34)[0, 9][4, 9]"
    );
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}
#[test]
fn many_or() {
    let src = r#"yeah { a | bb | "c" | 'd' | e | ff | "g" | 'h' }"#;
    let cbnf = Cbnf::parse(src);
    let out = cbnf_print(src, &cbnf);
    assert_eq!(
        out,
        "(0, 48)(0, 4)(5, 48)(7, 8)(9, 13)(11, 13)(14, 19)(16, 19)(20, 25)\
         (22, 25)(26, 29)(28, 29)(30, 34)(32, 34)(35, 40)(37, 40)(41, 46)\
         (43, 46)\
         [0, 15][1, 3][3, 5][5, 7][7, 9][9, 11][11, 13][13, 15]"
    );
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}
#[test]
fn many_group_or() {
    let src = r#"yeah { ((a | bb) | "c") | ((('d') | e) | (ff) | ("g" | 'h')) }"#;
    let cbnf = Cbnf::parse(src);
    let out = cbnf_print(src, &cbnf);
    assert_eq!(
        out,
        "\
            (0, 62)(0, 4)(5, 62)(7, 23)(8, 16)(9, 10)(11, 15)(13, 15)(17, 22)\
            (19, 22)(24, 60)(26, 60)(27, 38)(28, 33)(29, 32)(34, 37)(36, 37)\
            (39, 45)(41, 45)(42, 44)(46, 59)(48, 59)(49, 52)(53, 58)(55, 58)\
            [0, 22][0, 7][1, 5][3, 5][5, 7][7, 22][8, 22][9, 14][10, 12]\
            [12, 14][14, 17][15, 17][17, 22][18, 22][20, 22]"
    );
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}
#[test]
fn cbnf() {
    let src = include_str!("../../../cbnf.cbnf");
    let cbnf = Cbnf::parse(src);
    let out = cbnf_print(src, &cbnf);
    assert_eq!(
        out,
        "\
            (27, 63)(27, 34)(35, 63)(41, 44)(45, 49)(50, 61)(51, 54)(55, 60)\
            (57, 60)(64, 97)(64, 70)(71, 97)(78, 82)(83, 89)(90, 95)(92, 95)\
            (98, 129)(98, 102)(103, 129)(109, 114)(115, 118)(119, 123)\
            (124, 127)(130, 160)(130, 134)(135, 160)(141, 145)(146, 158)\
            (147, 151)(152, 157)(154, 157)(161, 212)(161, 165)(166, 212)\
            (172, 179)(180, 186)(182, 186)(187, 194)(189, 194)(195, 202)\
            (197, 202)(203, 210)(205, 210)(213, 239)(213, 218)(219, 239)\
            (225, 228)(229, 233)(234, 237)(240, 268)(240, 247)(248, 268)\
            (254, 257)(258, 262)(263, 266)(269, 296)(269, 273)(274, 296)\
            (280, 284)(285, 289)(290, 294)(298, 305)(298, 302)(303, 305)\
            (306, 314)(306, 311)(312, 314)(315, 321)(315, 318)(319, 321)\
            (322, 328)(322, 325)(326, 328)[0, 6][2, 6][4, 6][6, 10][8, 10]\
            [10, 14][14, 19][15, 19][17, 19][19, 28][20, 22][22, 24][24, 26]\
            [26, 28][28, 31][31, 34][34, 37][37, 37][37, 37][37, 37][37, 37]"
    );
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}

// ERROR TESTS -----------------------------------------------------------------

#[test]
fn unclosed_rule() {
    let src = "yeah { ";
    let cbnf = Cbnf::parse(src);
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!([Error::from(((5, 7).into(), UnclosedRule)),]);
    assert_eq!(actual, expected);
}
#[test]
fn unclosed_group() {
    let src = "yeah { ( }";
    let cbnf = Cbnf::parse(src);
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!([Error::from(((7, 10).into(), Unterminated))]);
    assert_eq!(actual, expected);
}
#[test]
fn unclosed_rule_group() {
    let src = "yeah { ( ";
    let cbnf = Cbnf::parse(src);
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!([
        Error::from(((7, 9).into(), Unterminated)),
        Error::from(((5, 9).into(), UnclosedRule)),
    ]);
    assert_eq!(actual, expected);
}
#[test]
fn int_or_float() {
    let src = "yeah { 12_u8 0o100 0b120i99 1f32 12.34f32 1e3 }";
    let cbnf = Cbnf::parse(src);
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
    let src = "yeah { \\ //\\@# \\ //\\\n}\n\\ //\\@# \\ //\\\n";
    let cbnf = Cbnf::parse(src);
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
fn dollar_after_rule() {
    let src = "yeah $";
    let cbnf = Cbnf::parse(src);
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!([
        Error::from(((0, 4).into(), UnopenedRule)),
        Error::from(((5, 6).into(), Expected([LexKind::Ident].into())))
    ]);
    assert_eq!(actual, expected);
}
#[test]
fn unterm_char() {
    let src = "yeah { '\n}";
    let cbnf = Cbnf::parse(src);
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!([Error::from((
        (7, 8).into(),
        InvalidLit(InvalidLiteral::Unterminated)
    )),]);
    assert_eq!(actual, expected);
}
#[test]
fn unterm_string() {
    let src = "yeah { \"}";
    let cbnf = Cbnf::parse(src);
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!([
        Error::from(((7, 9).into(), InvalidLit(InvalidLiteral::Unterminated))),
        Error::from(((5, 9).into(), UnclosedRule)),
    ]);
    assert_eq!(actual, expected);
}
