use biturbo_spiral::highlight::{highlight, is_spiral_path};
use biturbo_spiral::lexer::Style::{self, *};
use std::string::String as Text;

fn styled(text: &str) -> Vec<(Text, Style)> {
    let units: Vec<u16> = text.encode_utf16().collect();
    highlight(text, &[(0, units.len() as u32)])
        .into_iter()
        .map(|span| (Text::from_utf16_lossy(&units[span.start as usize..span.end as usize]), span.style))
        .collect()
}

fn pairs(expected: &[(&str, Style)]) -> Vec<(Text, Style)> {
    expected.iter().map(|(text, style)| (text.to_string(), *style)).collect()
}

#[test]
fn edition_2_definition() {
    assert_eq!(
        styled("inl f (x : i32) = if x > 0i32 then Some x else None"),
        pairs(&[("inl", Keyword), ("if", Keyword), ("0", Number), ("i32", Keyword), ("then", Keyword),
            ("Some", Type), ("else", Keyword), ("None", Type)])
    );
}

#[test]
fn edition_3_symbol_keywords() {
    assert_eq!(
        styled(":= f x = ? x > 1 ?> !+ |: !-"),
        pairs(&[(":=", Keyword), ("?", Keyword), ("1", Number), ("?>", Keyword), ("!+", Number), ("|:", Keyword),
            ("!-", Number)])
    );
}

#[test]
fn edition_3_keyword_before_comma() {
    assert_eq!(styled("(a :~, b)"), pairs(&[(":~", Keyword)]));
}

#[test]
fn strings_and_escapes() {
    assert_eq!(
        styled(r#"let s = "a\nb" ++ 'c' ++ @"raw\""#),
        pairs(&[("let", Keyword), ("\"a", String), ("\\n", Attribute), ("b\"", String), ("'c'", String),
            ("@\"raw\\\"", String)])
    );
}

#[test]
fn macro_splices() {
    assert_eq!(
        styled(r#"$"std::mem::drop(!x) as `t @n \v !!""#),
        pairs(&[("$\"std::mem::drop(", String), ("!x", Variable), (") as ", String), ("`t", Type),
            (" ", String), ("@n", Type), (" ", String), ("\\v", Keyword), (" ", String), ("!!", Attribute),
            ("\"", String)])
    );
}

#[test]
fn macro_expression_splice() {
    assert_eq!(
        styled(r#"$'f(!(g 1i32))'"#),
        pairs(&[("$'f(", String), ("1", Number), ("i32", Keyword), (")'", String)])
    );
}

#[test]
fn symbols_and_unary_operators() {
    assert_eq!(
        styled("sm'.contains .(+) (-x) a - b"),
        pairs(&[(".contains", Value), (".(+)", Value), ("-", Command)])
    );
}

#[test]
fn numbers() {
    assert_eq!(
        styled("f -1 0xFF 1.5f64 2u8 x-1"),
        pairs(&[("-1", Number), ("0xFF", Number), ("1.5", Number), ("f64", Keyword), ("2", Number), ("u8", Keyword),
            ("1", Number)])
    );
}

#[test]
fn line_comments_and_directives() {
    assert_eq!(
        styled("///> rust -d regex\n/// doc\nlet x = 1 // note\n///\na // b"),
        pairs(&[("///> rust -d regex", Attribute), ("/// doc", Comment), ("let", Keyword), ("1", Number),
            ("// note", Comment), ("///", Comment), ("// b", Comment)])
    );
}

#[test]
fn block_comment_spans_lines() {
    assert_eq!(
        styled("let (* a\n (* nested *) b\nc *) inl\n2 (*) 3"),
        pairs(&[("let", Keyword), ("(* a", Comment), (" (* nested *) b", Comment), ("c *)", Comment),
            ("inl", Keyword), ("2", Number), ("3", Number)])
    );
}

#[test]
fn triple_string_spans_lines() {
    assert_eq!(
        styled("let s = \"\"\"a\n\"\" b\nc\"\"\" inl\n\"\"\"\"\"\" x"),
        pairs(&[("let", Keyword), ("\"\"\"a", String), ("\"\" b", String), ("c\"\"\"", String), ("inl", Keyword),
            ("\"\"\"\"\"\"", String)])
    );
}

#[test]
fn macro_continues_on_next_line() {
    assert_eq!(
        styled("$\"a \\\n  !b\" inl"),
        pairs(&[("$\"a \\", String), ("!b", Variable), ("\"", String), ("inl", Keyword)])
    );
}

#[test]
fn test_headers() {
    assert_eq!(styled("test add_one = 1\n  test"), pairs(&[("test", Keyword), ("1", Number)]));
}

#[test]
fn utf16_offsets_and_crlf() {
    assert_eq!(styled("\"😀\" inl\r\nlet"), pairs(&[("\"😀\"", String), ("inl", Keyword), ("let", Keyword)]));
}

#[test]
fn ranges_restart_state() {
    let text = "(* open\nlet\n@@ -1 +1 @@\nlet";
    let units: Vec<u16> = text.encode_utf16().collect();
    let first_end = text.find("@@").unwrap() as u32;
    let second_start = text.rfind("let").unwrap() as u32;
    let spans = highlight(text, &[(0, first_end), (second_start, units.len() as u32)]);
    let styles: Vec<(u32, Style)> = spans.iter().map(|span| (span.start, span.style)).collect();
    assert_eq!(styles, vec![(0, Comment), (8, Comment), (second_start, Keyword)]);
}

#[test]
fn spiral_paths() {
    assert!(is_spiral_path("lib/spiral/sm'.spi"));
    assert!(is_spiral_path("C:/x/main.SPIR"));
    assert!(!is_spiral_path("src/lib.rs"));
    assert!(!is_spiral_path("package.spiproj"));
}
