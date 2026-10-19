use utsc::{Scanner, SyntaxKind};

#[test]
fn borrows_text_and_preserves_utf16_offsets() {
    let source = "const 变量 = '😀';";
    let mut scanner = Scanner::new(source);
    let tokens: Vec<_> = scanner.by_ref().collect();
    let name = &tokens[2];
    assert_eq!(name.kind, SyntaxKind::Identifier);
    assert_eq!(
        (name.start, name.end, name.byte_start, name.byte_end),
        (6, 8, 6, 12)
    );
    let string = tokens
        .iter()
        .find(|t| t.kind == SyntaxKind::StringLiteral)
        .unwrap();
    assert_eq!((string.start, string.end), (11, 15));
    assert_eq!(name.text.as_ptr(), source[6..].as_ptr());
    assert!(scanner.diagnostics().is_empty());
    assert!(scanner.next().is_none());
    assert!(scanner.next().is_none());
}

#[test]
fn escaped_keywords_and_private_names() {
    let tokens: Vec<_> = Scanner::new(r"\u0069f #\u{61}bc café a\u200cb").collect();
    assert_eq!(tokens[0].kind, SyntaxKind::IfKeyword);
    assert_eq!(tokens[2].kind, SyntaxKind::PrivateIdentifier);
    assert_eq!(tokens[4].text, "café");
    assert_eq!(tokens[6].kind, SyntaxKind::Identifier);
}

#[test]
fn bare_hash_makes_progress_instead_of_hanging_like_upstream_api() {
    let mut scanner = Scanner::new("# #\\u0020");
    let tokens: Vec<_> = scanner.by_ref().collect();
    assert_eq!(
        tokens.iter().map(|t| t.text).collect::<String>(),
        "# #\\u0020"
    );
    assert_eq!(tokens.last().unwrap().kind, SyntaxKind::EndOfFile);
    assert!(!scanner.diagnostics().is_empty());
}

#[test]
fn lexical_errors_have_bounded_spans() {
    for source in [
        "'bad", "/* bad", "`bad", "😀", "1e+", "1__0", "0b2", "\\uZZZZ",
    ] {
        let mut scanner = Scanner::new(source);
        let tokens: Vec<_> = scanner.by_ref().collect();
        assert_eq!(tokens.iter().map(|t| t.text).collect::<String>(), source);
        assert!(
            !scanner.diagnostics().is_empty(),
            "missing diagnostic: {source:?}"
        );
        for diagnostic in scanner.diagnostics() {
            assert!(
                diagnostic.start + diagnostic.length <= source.encode_utf16().count(),
                "{source:?}: {diagnostic:?}"
            );
        }
    }
}

#[test]
fn all_ecmascript_line_separators_are_preserved() {
    let text = "a\r\nb\rc\nd\u{2028}e\u{2029}f";
    let tokens: Vec<_> = Scanner::new(text).collect();
    assert_eq!(
        tokens
            .iter()
            .filter(|t| t.kind == SyntaxKind::NewLineTrivia)
            .count(),
        5
    );
    assert_eq!(tokens.iter().map(|t| t.text).collect::<String>(), text);
}

#[test]
fn decimal_optional_chain_and_generic_closers_are_not_greedily_combined() {
    let tokens: Vec<_> = Scanner::new("a?.3 >> >=")
        .filter(|t| !t.kind.is_trivia())
        .collect();
    assert_eq!(tokens[1].kind, SyntaxKind::QuestionToken);
    assert_eq!(tokens[2].text, ".3");
    assert_eq!(tokens[3].kind, SyntaxKind::GreaterThanToken);
    assert_eq!(tokens[4].kind, SyntaxKind::GreaterThanToken);
    assert_eq!(tokens[5].kind, SyntaxKind::GreaterThanToken);
    assert_eq!(tokens[6].kind, SyntaxKind::EqualsToken);
}

#[test]
fn seeded_unicode_inputs_always_terminate_and_roundtrip() {
    let alphabet = [
        "a", "0", "变量", "𐐀", "😀", "\u{fffd}", "\u{2028}", "\u{2029}", " ", "\r\n", "'", "\"",
        "`", "\\", "/", "*", "$", "{", "}", "#", "e", ".", "+", "_", "\u{200c}",
    ];
    let mut seed = 0x735463_u32;
    for _ in 0..10_000 {
        let mut source = String::new();
        for _ in 0..32 {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            source.push_str(alphabet[seed as usize % alphabet.len()]);
        }
        let mut byte = 0;
        let mut utf16 = 0;
        let mut eof = 0;
        for token in Scanner::new(&source) {
            assert_eq!((token.byte_start, token.start), (byte, utf16));
            assert_eq!(token.text, &source[token.byte_start..token.byte_end]);
            byte = token.byte_end;
            utf16 += token.text.encode_utf16().count();
            assert_eq!(token.end, utf16);
            if token.kind == SyntaxKind::EndOfFile {
                eof += 1;
            } else {
                assert!(!token.text.is_empty());
            }
        }
        assert_eq!(byte, source.len());
        assert_eq!(eof, 1);
    }
}
