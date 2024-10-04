use super::*;

use expect_test::{expect, Expect};

fn check_lexing(src: &str, expect: Expect) {
    let actual: String = tokenize(src)
        .map(|token| format!("{:?}\n", token))
        .collect();
    expect.assert_eq(&actual)
}

#[test]
fn smoke_test() {
    check_lexing(
        "fn main() { println!(\"zebra\"); } # my source file\n",
        expect![[r#"
            Lexeme { kind: Ident, len: 2 }
            Lexeme { kind: Whitespace, len: 1 }
            Lexeme { kind: Ident, len: 4 }
            Lexeme { kind: OpenParen, len: 1 }
            Lexeme { kind: CloseParen, len: 1 }
            Lexeme { kind: Whitespace, len: 1 }
            Lexeme { kind: OpenBrace, len: 1 }
            Lexeme { kind: Whitespace, len: 1 }
            Lexeme { kind: Ident, len: 7 }
            Lexeme { kind: Bang, len: 1 }
            Lexeme { kind: OpenParen, len: 1 }
            Lexeme { kind: Literal { kind: Str { terminated: true }, suffix_start: 7 }, len: 7 }
            Lexeme { kind: CloseParen, len: 1 }
            Lexeme { kind: Semi, len: 1 }
            Lexeme { kind: Whitespace, len: 1 }
            Lexeme { kind: CloseBrace, len: 1 }
            Lexeme { kind: Whitespace, len: 1 }
            Lexeme { kind: LineComment { doc_style: None }, len: 16 }
            Lexeme { kind: Whitespace, len: 1 }
        "#]],
    )
}

#[test]
fn comment_flavors() {
    check_lexing(
        r"
# line
#:: line as well
#: outer doc line
#! inner doc line
",
        expect![[r#"
            Lexeme { kind: Whitespace, len: 1 }
            Lexeme { kind: LineComment { doc_style: None }, len: 6 }
            Lexeme { kind: Whitespace, len: 1 }
            Lexeme { kind: LineComment { doc_style: None }, len: 16 }
            Lexeme { kind: Whitespace, len: 1 }
            Lexeme { kind: LineComment { doc_style: Some(Outer) }, len: 17 }
            Lexeme { kind: Whitespace, len: 1 }
            Lexeme { kind: LineComment { doc_style: Some(Inner) }, len: 17 }
            Lexeme { kind: Whitespace, len: 1 }
        "#]],
    )
}

#[test]
fn characters() {
    check_lexing(
        "'a' ' ' '\\n'",
        expect![[r#"
            Lexeme { kind: Literal { kind: Char { terminated: true }, suffix_start: 3 }, len: 3 }
            Lexeme { kind: Whitespace, len: 1 }
            Lexeme { kind: Literal { kind: Char { terminated: true }, suffix_start: 3 }, len: 3 }
            Lexeme { kind: Whitespace, len: 1 }
            Lexeme { kind: Literal { kind: Char { terminated: true }, suffix_start: 4 }, len: 4 }
        "#]],
    );
}

#[test]
fn incomplete_char() {
    check_lexing(
        "'abc",
        expect![[r#"
            Lexeme { kind: Literal { kind: Char { terminated: false }, suffix_start: 4 }, len: 4 }
        "#]],
    );
}

#[test]
fn literal_suffixes() {
    check_lexing(
        r#"
'a'
"a"
1234
0b101
0xABC
1.0
1.0e10
2us
"#,
        expect![[r#"
            Lexeme { kind: Whitespace, len: 1 }
            Lexeme { kind: Literal { kind: Char { terminated: true }, suffix_start: 3 }, len: 3 }
            Lexeme { kind: Whitespace, len: 1 }
            Lexeme { kind: Literal { kind: Str { terminated: true }, suffix_start: 3 }, len: 3 }
            Lexeme { kind: Whitespace, len: 1 }
            Lexeme { kind: Literal { kind: Int { base: Decimal, empty_int: false }, suffix_start: 4 }, len: 4 }
            Lexeme { kind: Whitespace, len: 1 }
            Lexeme { kind: Literal { kind: Int { base: Binary, empty_int: false }, suffix_start: 5 }, len: 5 }
            Lexeme { kind: Whitespace, len: 1 }
            Lexeme { kind: Literal { kind: Int { base: Hexadecimal, empty_int: false }, suffix_start: 5 }, len: 5 }
            Lexeme { kind: Whitespace, len: 1 }
            Lexeme { kind: Literal { kind: Float { base: Decimal, empty_exponent: false }, suffix_start: 3 }, len: 3 }
            Lexeme { kind: Whitespace, len: 1 }
            Lexeme { kind: Literal { kind: Float { base: Decimal, empty_exponent: false }, suffix_start: 6 }, len: 6 }
            Lexeme { kind: Whitespace, len: 1 }
            Lexeme { kind: Literal { kind: Int { base: Decimal, empty_int: false }, suffix_start: 1 }, len: 3 }
            Lexeme { kind: Whitespace, len: 1 }
        "#]],
    )
}
