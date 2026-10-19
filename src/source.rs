/// A zero-based line and UTF-16 column, as used by TypeScript and LSP.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineColumn {
    pub line: usize,
    pub column: usize,
}

/// Immutable UTF-8 text with an index of line starts. No global source storage.
#[derive(Debug)]
pub struct SourceFile {
    text: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn new(text: String) -> Self {
        let mut line_starts = vec![0];
        let mut chars = text.char_indices().peekable();
        while let Some((offset, ch)) = chars.next() {
            match ch {
                '\r' => {
                    let end = if let Some(&(next, '\n')) = chars.peek() {
                        chars.next();
                        next + 1
                    } else {
                        offset + 1
                    };
                    line_starts.push(end);
                }
                '\n' | '\u{2028}' | '\u{2029}' => line_starts.push(offset + ch.len_utf8()),
                _ => {}
            }
        }
        Self { text, line_starts }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Returns None for an out-of-bounds offset or the middle of a UTF-8 scalar.
    /// A column never points into the middle of a UTF-16 surrogate pair.
    pub fn position(&self, byte_offset: usize) -> Option<LineColumn> {
        if !self.text.is_char_boundary(byte_offset) {
            return None;
        }
        let line = self
            .line_starts
            .partition_point(|&start| start <= byte_offset)
            - 1;
        let column = self.text[self.line_starts[line]..byte_offset]
            .encode_utf16()
            .count();
        Some(LineColumn { line, column })
    }

    /// Convert an LSP position back to a UTF-8 byte offset, rejecting columns
    /// inside surrogate pairs, line terminators, or past the end of the line.
    pub fn byte_offset(&self, position: LineColumn) -> Option<usize> {
        let start = *self.line_starts.get(position.line)?;
        let end = self
            .line_starts
            .get(position.line + 1)
            .copied()
            .unwrap_or(self.text.len());
        let mut column = 0;
        for (relative, ch) in self.text[start..end].char_indices() {
            if column == position.column {
                return Some(start + relative);
            }
            if matches!(ch, '\r' | '\n' | '\u{2028}' | '\u{2029}') {
                return None;
            }
            column += ch.len_utf16();
        }
        (column == position.column).then_some(end)
    }
}
