use std::fmt::Display;

use crate::{
    lexer::LexKind,
    parser::{
        error::{Error::*, InvalidLiteral},
        LIST_EXPECTED, RULE_EXPECTED,
    },
    span::{BSpan, TSpan},
    Cbnf, List, Rule,
};

use pretty_assertions::assert_eq;

// TODO: create better testing system.
//
// TODO: add more testing for error cases.
//
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
            cbnf.terms(l.terms)
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
    cbnf.terms(list.terms).iter().for_each(|term| {
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
    let src = "yeah { }";
    let cbnf = Cbnf::parse(src);
    let out = cbnf_print(src, &cbnf);
    assert_eq!(out, "(0, 8)(0, 4)(5, 8)[0, 0]");
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}
#[test]
fn short_meta() {
    let src = "$yeah;";
    let cbnf = Cbnf::parse(src);
    let out = cbnf_print(src, &cbnf);
    assert_eq!(out, "(0, 5)(0, 5)");
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}
#[test]
fn empty_long_meta() {
    let src = "$yeah { }";
    let cbnf = Cbnf::parse(src);
    let out = cbnf_print(src, &cbnf);
    assert_eq!(out, "(0, 9)(0, 5)(6, 9)[0, 0]");
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
fn metas() {
    let src = "yeah { $one $two $three }";
    let cbnf = Cbnf::parse(src);
    let out = cbnf_print(src, &cbnf);
    assert_eq!(out, "(0, 25)(0, 4)(5, 25)(7, 11)(12, 16)(17, 23)[0, 3]");
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
    let src = r#"yeah { nil a $b "c" 'd' (a $b "c" 'd') nil }"#;
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
    let src = r#"yeah { a $b "c" 'd' | a $b "c" 'd' }"#;
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
    let src = r#"yeah { a | $b | "c" | 'd' | e | $f | "g" | 'h' }"#;
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
    let src = r#"yeah { ((a | $b) | "c") | ((('d') | e) | ($f) | ("g" | 'h')) }"#;
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
            (126, 157)(126, 133)(134, 157)(140, 143)(144, 149)(150, 154)\
            (158, 191)(158, 164)(165, 191)(172, 176)(177, 183)(184, 189)\
            (186, 189)(192, 244)(192, 196)(197, 244)(203, 218)(204, 210)\
            (211, 217)(213, 217)(219, 222)(223, 227)(228, 231)(232, 242)\
            (234, 238)(239, 242)(245, 275)(245, 249)(250, 275)(256, 260)\
            (261, 273)(262, 266)(267, 272)(269, 272)(276, 321)(276, 280)\
            (281, 321)(287, 294)(295, 303)(297, 303)(304, 310)(306, 310)\
            (311, 318)(313, 318)(322, 345)(322, 326)(327, 345)(333, 336)\
            (337, 343)(346, 373)(346, 351)(352, 373)(358, 361)(362, 366)\
            (367, 370)(375, 403)(375, 382)(383, 403)(389, 392)(393, 397)\
            (398, 401)(405, 410)(405, 410)(428, 433)(428, 433)(456, 462)\
            (456, 462)(511, 515)(511, 515)\
            [0, 3][3, 7][5, 7][7, 17][7, 11][9, 11][14, 17][17, 22][18, 22]\
            [20, 22][22, 29][23, 25][25, 27][27, 29][29, 31][31, 34][34, 37]"
    );
    assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
}

// ERROR TESTS -----------------------------------------------------------------

#[test]
fn unclosed_rule() {
    let src = "yeah { ";
    let cbnf = Cbnf::parse(src);
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!([Expected((7, 7).into(), [LexKind::CloseBrace].into())]);
    assert_eq!(actual, expected);
}
#[test]
fn unclosed_group() {
    let src = "yeah { ( }";
    let cbnf = Cbnf::parse(src);
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!([Unterminated((7, 10).into())]);
    assert_eq!(actual, expected);
}
#[test]
fn unclosed_rule_group() {
    let src = "yeah { ( ";
    let cbnf = Cbnf::parse(src);
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!([
        Unterminated((7, 9).into()),
        Expected((9, 9).into(), [LexKind::CloseBrace].into())
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
fn dollar_repeat() {
    let src = "$ $";
    let cbnf = Cbnf::parse(src);
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!(expected![IDENT_1, (2, 3)]);
    assert_eq!(actual, expected);
}
#[test]
fn empty_dollar() {
    let src = "$yeah { $ }";
    let cbnf = Cbnf::parse(src);
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!(expected![IDENT_1, (10, 11)]);
    assert_eq!(actual, expected);
}
#[test]
fn dolllar_after_rule() {
    let src = "$yeah $";
    let cbnf = Cbnf::parse(src);
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!(expected![META_AFTER_IDENT, (6, 7)]);
    assert_eq!(actual, expected);
}
#[test]
fn rule_after_ident() {
    let src = "yeah $";
    let cbnf = Cbnf::parse(src);
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!(expected![RULE_AFTER_IDENT, (5, 6)]);
    assert_eq!(actual, expected);
}
#[test]
fn unterm_char() {
    let src = "yeah { '\n}";
    let cbnf = Cbnf::parse(src);
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!([InvalidLit(InvalidLiteral::Unterminated, (7, 8).into())]);
    assert_eq!(actual, expected);
}
#[test]
fn unterm_string() {
    let src = "yeah { \"}";
    let cbnf = Cbnf::parse(src);
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!([
        InvalidLit(InvalidLiteral::Unterminated, (7, 9).into()),
        Expected((9, 9).into(), [LexKind::CloseBrace].into())
    ]);
    assert_eq!(actual, expected);
}
