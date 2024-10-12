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

// TODO: add more testing for newly added stuff

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
            (28, 72)(28, 35)(36, 72)(42, 54)(55, 70)(57, 70)(73, 115)(73, 85)\
            (86, 115)(92, 96)(97, 101)(102, 113)(103, 106)(107, 112)(109, 112)\
            (116, 168)(116, 129)(130, 168)(136, 140)(141, 154)(155, 159)\
            (160, 166)(162, 166)(169, 202)(169, 175)(176, 202)(183, 187)\
            (188, 194)(195, 200)(197, 200)(203, 234)(203, 207)(208, 234)\
            (214, 219)(220, 223)(224, 228)(229, 232)(235, 265)(235, 239)\
            (240, 265)(246, 250)(251, 263)(252, 256)(257, 262)(259, 262)\
            (266, 316)(266, 270)(271, 316)(277, 283)(284, 290)(286, 290)\
            (291, 298)(293, 298)(299, 306)(301, 306)(307, 314)(309, 314)\
            (317, 343)(317, 322)(323, 343)(329, 332)(333, 337)(338, 341)\
            (344, 371)(344, 350)(351, 371)(357, 360)(361, 365)(366, 369)\
            (372, 399)(372, 376)(377, 399)(383, 387)(388, 392)(393, 397)\
            (400, 431)(400, 405)(406, 431)(412, 415)(416, 429)(432, 484)\
            (432, 445)(446, 484)(452, 455)(456, 469)(470, 473)(474, 482)\
            (476, 482)(486, 493)(486, 490)(491, 493)(494, 502)(494, 499)\
            (500, 502)(503, 509)(503, 506)(507, 509)(510, 516)(510, 513)\
            (514, 516)[0, 3][1, 3][3, 9][5, 9][7, 9][9, 14][12, 14][14, 18]\
            [16, 18][18, 22][22, 27][23, 27][25, 27][27, 36][28, 30][30, 32]\
            [32, 34][34, 36][36, 39][39, 42][42, 45][45, 47][47, 52][50, 52]\
            [52, 52][52, 52][52, 52][52, 52]"
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
    let src = "yeah { \\ #\\@// \\ #\\\n}\n\\ #\\@// \\ #\\\n";
    let cbnf = Cbnf::parse(src);
    let actual = format!("{:#?}", cbnf.errors);
    let expected = debug!(expected![
        LIST_EXPECTED,
        (7, 8),
        (9, 12),
        RULE_EXPECTED,
        (22, 23),
        (24, 27)
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
