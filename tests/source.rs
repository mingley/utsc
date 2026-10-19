use utsc::{LineColumn, SourceFile};

#[test]
fn maps_astral_characters_without_confusing_bytes_and_code_units() {
    let file = SourceFile::new("a😀b\r\n变量\u{2028}c\u{2029}".into());
    assert_eq!(file.line_count(), 4);
    assert_eq!(file.position(5), Some(LineColumn { line: 0, column: 3 }));
    assert_eq!(file.position(8), Some(LineColumn { line: 1, column: 0 }));
    assert_eq!(file.position(14), Some(LineColumn { line: 1, column: 2 }));
    assert_eq!(file.position(2), None);
    assert_eq!(file.position(file.text().len() + 1), None);
    assert_eq!(file.byte_offset(LineColumn { line: 0, column: 2 }), None);
    assert_eq!(file.byte_offset(LineColumn { line: 0, column: 3 }), Some(5));
    assert_eq!(
        file.byte_offset(LineColumn {
            line: 0,
            column: 99
        }),
        None
    );
    assert_eq!(file.byte_offset(LineColumn { line: 4, column: 0 }), None);
}

#[test]
fn roundtrips_every_character_boundary_outside_line_terminators() {
    let file = SourceFile::new("😀hello\r\n变量\ncafé\r\u{2028}end".into());
    for (byte, ch) in file.text().char_indices() {
        if matches!(ch, '\r' | '\n' | '\u{2028}' | '\u{2029}') {
            continue;
        }
        let position = file.position(byte).unwrap();
        assert_eq!(file.byte_offset(position), Some(byte));
    }
    let eof = file.text().len();
    assert_eq!(file.byte_offset(file.position(eof).unwrap()), Some(eof));
}

#[test]
fn empty_and_trailing_lines() {
    for text in ["", "\n", "\r\n", "\r", "\u{2028}", "\u{2029}"] {
        let file = SourceFile::new(text.into());
        let eof = file.position(text.len()).unwrap();
        assert_eq!(eof.column, 0);
        assert_eq!(file.byte_offset(eof), Some(text.len()));
    }
}
