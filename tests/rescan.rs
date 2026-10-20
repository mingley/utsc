use utsc::{RescanError, Scanner, SyntaxKind};

#[test]
fn greater_operators_are_combined_only_on_request() {
    for (text, kind) in [
        (">", SyntaxKind::GreaterThanToken),
        (">=", SyntaxKind::GreaterThanEqualsToken),
        (">>", SyntaxKind::GreaterThanGreaterThanToken),
        (">>>", SyntaxKind::GreaterThanGreaterThanGreaterThanToken),
        (">>=", SyntaxKind::GreaterThanGreaterThanEqualsToken),
        (
            ">>>=",
            SyntaxKind::GreaterThanGreaterThanGreaterThanEqualsToken,
        ),
    ] {
        let mut scanner = Scanner::new(text);
        let token = scanner.next().unwrap();
        assert_eq!(token.text, ">");
        let combined = scanner.rescan_greater(token).unwrap();
        assert_eq!(combined.kind, kind);
        assert_eq!(combined.text, text);
        assert_eq!(combined.end, text.len());
        assert_eq!(scanner.next().unwrap().kind, SyntaxKind::EndOfFile);
    }
    assert_eq!(Scanner::new(">>>").count(), 4);
}
#[test]
fn template_middle_and_tail_preserve_utf16_spans() {
    let mut scanner = Scanner::new("`😀${x}中${y}end`");
    assert_eq!(scanner.next().unwrap().kind, SyntaxKind::TemplateHead);
    assert_eq!(scanner.next().unwrap().text, "x");
    let close = scanner.next().unwrap();
    let middle = scanner.rescan_template(close).unwrap();
    assert_eq!(
        (middle.kind, middle.text, middle.start, middle.end),
        (SyntaxKind::TemplateMiddle, "}中${", 6, 10)
    );
    assert_eq!(scanner.next().unwrap().text, "y");
    let close = scanner.next().unwrap();
    let tail = scanner.rescan_template(close).unwrap();
    assert_eq!(
        (tail.kind, tail.text, tail.start, tail.end),
        (SyntaxKind::TemplateTail, "}end`", 11, 16)
    );
    assert!(scanner.diagnostics().is_empty());
}
#[test]
fn stale_foreign_and_wrong_kind_rescans_are_rejected_without_corrupting_state() {
    let mut scanner = Scanner::new("> x");
    let previous = scanner.next().unwrap();
    let whitespace = scanner.next().unwrap();
    assert_eq!(
        scanner.rescan_greater(previous),
        Err(RescanError::StaleToken)
    );
    assert_eq!(
        scanner.rescan_greater(whitespace),
        Err(RescanError::UnexpectedToken)
    );
    assert_eq!(scanner.next().unwrap().text, "x");
    let eof = scanner.next().unwrap();
    assert!(scanner.rescan_template(eof).is_err());
    assert!(scanner.next().is_none());
    let source = String::from(">");
    let other = String::from(">");
    let mut a = Scanner::new(&source);
    let mut b = Scanner::new(&other);
    a.next();
    assert_eq!(
        a.rescan_greater(b.next().unwrap()),
        Err(RescanError::StaleToken)
    );
}
