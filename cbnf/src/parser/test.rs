use crate::span::BSpan;
use crate::{parser::error::Error, Cbnf, Expression, List, Rule};

use pretty_assertions::assert_eq;

// TODO: create better testing system.
//
// TODO: add testing for error cases.
//
// TODO: add fuzzing.
macro_rules! test_success {
    (
        $($name:ident, $cbnf:expr, $expected: expr),*
    ) => {
        $(
        #[test]
        fn $name() {
            let cbnf = Cbnf::parse($cbnf);
            let out = cbnf_print($cbnf, &cbnf);
            assert_eq!(out, $expected);
            assert!(cbnf.errors.is_empty(), "{:#?}", cbnf.errors);
        }
        )*
    };
}

macro_rules! test_error {
    (
        $($name:ident, $cbnf:expr, $expected: expr),*
    ) => {
        $(
        #[test]
        fn $name() {
            let cbnf = Cbnf::parse($cbnf);
            assert_eq!(cbnf.errors, Vec::<Error>::from($expected));
        }
        )*
    };
}

test_success!(
    empty,
    "yeah { }",
    "(0, 8)(5, 8)(5, 8)",
    short_meta,
    "$yeah;",
    "(0, 5)",
    empty_long_meta,
    "$yeah { }",
    "(0, 9)(6, 9)(6, 9)",
    strings,
    r#"yeah { "one" "two" "three" }"#,
    "(0, 28)(5, 28)(7, 26)(7, 12)(13, 18)(19, 26)",
    chars,
    "yeah { 'o' 't' 'h' }",
    "(0, 20)(5, 20)(7, 18)(7, 10)(11, 14)(15, 18)",
    idents,
    "yeah { one two three }",
    "(0, 22)(5, 22)(7, 20)(7, 10)(11, 14)(15, 20)",
    metas,
    "yeah { $one $two $three }",
    "(0, 25)(5, 25)(7, 23)(7, 11)(12, 16)(17, 23)",
    group,
    "yeah { ( ) }",
    "(0, 12)(5, 12)(7, 10)(7, 10)",
    mixed,
    r#"yeah { nil a $b "c" 'd' (a $b "c" 'd') nil }"#,
    "(0, 44)(5, 44)(7, 42)(7, 10)(11, 12)(13, 15)(16, 19)(20, 23)(24, 38)(39, 42)",
    cbnf,
    include_str!("../../../cbnf.cbnf"),
    "\
        (126, 157)(139, 157)(141, 155)(141, 144)(145, 150)(151, 155)(158, 192)\
        (170, 192)(172, 183)(172, 176)(177, 183)(187, 190)(187, 190)(193, 248)\
        (203, 248)(205, 234)(205, 221)(222, 225)(226, 230)(231, 234)(238, 246)\
        (238, 242)(243, 246)(249, 286)(259, 286)(261, 284)(261, 265)(266, 284)\
        (287, 319)(297, 319)(299, 317)(299, 303)(304, 317)(320, 369)(330, 369)\
        (332, 339)(332, 339)(343, 349)(343, 349)(353, 357)(353, 357)(361, 366)\
        (361, 366)(370, 394)(380, 394)(382, 392)(382, 385)(386, 392)(395, 422)\
        (406, 422)(408, 420)(408, 411)(412, 416)(417, 420)(424, 453)(437, 453)\
        (439, 451)(439, 442)(443, 447)(448, 451)(455, 460)(478, 483)(506, 512)\
        (561, 565)"
);

test_error!(
    unclosed_rule,
    "yeah { ",
    [Error::Eof(7)],
    unclosed_group,
    "yeah { ( }",
    [Error::Unterminated(BSpan::new(9, 10,),), Error::Eof(10)]
);

fn cbnf_print(src: &str, cbnf: &Cbnf) -> String {
    let mut out = String::with_capacity(src.len());
    cbnf.rules()
        .iter()
        .for_each(|rule| rule_print(&mut out, rule));
    out
}

fn rule_print(out: &mut String, rule: &Rule) {
    rule.span.write(out);
    let Some(expr) = &rule.expr else {
        return;
    };
    expr.span.write(out);
    expr_print(out, expr);
}

fn expr_print(out: &mut String, expr: &Expression) {
    expr.parts
        .iter()
        .for_each(|(_, list)| list_print(out, list));
}

fn list_print(out: &mut String, list: &List) {
    list.span.write(out);
    list.terms.iter().for_each(|term| term.span().write(out));
}

trait SpanWrite {
    fn write(self, out: &mut String);
}

impl SpanWrite for crate::span::BSpan {
    fn write(self, out: &mut String) {
        out.push('(');
        out.push_str(&self.from.to_string());
        out.push_str(", ");
        out.push_str(&self.to.to_string());
        out.push(')');
    }
}
