use std::borrow::Cow;
use std::iter::FusedIterator;

use serde::Serialize;

use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::kind::{SyntaxKind, keyword};
use crate::unicode::{is_part, is_start};
use DiagnosticCode::*;

/// A token borrows its spelling. Normal scanning allocates no token strings.
/// `start` and `end` are UTF-16 code-unit offsets; byte offsets slice the input.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct Token<'a> {
    pub kind: SyntaxKind,
    pub start: usize,
    pub end: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub text: &'a str,
    pub unterminated: bool,
}

/// Lossless context-free scanner for UTF-8 source. Emits trivia and one EOF.
///
/// `/` and `>` remain individual tokens where TypeScript requires parser-led
/// rescanning. Templates stop at `${`. This interface does not guess syntax.
/// Every non-EOF token consumes at least one Unicode scalar, including errors.
#[derive(Debug)]
pub struct Scanner<'a> {
    source: &'a str,
    byte: usize,
    utf16: usize,
    finished: bool,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Scanner<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            byte: 0,
            utf16: 0,
            finished: false,
            diagnostics: Vec::new(),
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Combine a current `>` token only when the parser is in operator context.
    /// Generic type parsers should keep the uncombined tokens instead.
    pub fn rescan_greater(&mut self, token: Token<'a>) -> Result<Token<'a>, RescanError> {
        self.validate_current(token, SyntaxKind::GreaterThanToken)?;
        use SyntaxKind::*;
        for (suffix, kind) in [
            (">>=", GreaterThanGreaterThanGreaterThanEqualsToken),
            (">>", GreaterThanGreaterThanGreaterThanToken),
            (">=", GreaterThanGreaterThanEqualsToken),
            (">", GreaterThanGreaterThanToken),
            ("=", GreaterThanEqualsToken),
        ] {
            if self.rest().starts_with(suffix) {
                self.ascii(suffix.len());
                return Ok(self.extended_token(token, kind, false));
            }
        }
        Ok(token)
    }

    /// Reinterpret a current `}` as the start of a template continuation.
    /// This must be requested by a parser tracking nested substitution braces.
    pub fn rescan_template(&mut self, token: Token<'a>) -> Result<Token<'a>, RescanError> {
        self.validate_current(token, SyntaxKind::CloseBraceToken)?;
        self.byte = token.byte_start;
        self.utf16 = token.start;
        let (kind, unterminated) = self.template(token.start, true);
        Ok(self.extended_token(token, kind, unterminated))
    }

    fn validate_current(&self, token: Token<'a>, kind: SyntaxKind) -> Result<(), RescanError> {
        if self.finished
            || token.byte_end != self.byte
            || token.end != self.utf16
            || token.start.checked_add(1) != Some(token.end)
        {
            return Err(RescanError::StaleToken);
        }
        let Some(text) = self.source.get(token.byte_start..token.byte_end) else {
            return Err(RescanError::StaleToken);
        };
        if text.as_ptr() != token.text.as_ptr() || text != token.text {
            return Err(RescanError::StaleToken);
        }
        let expected = if kind == SyntaxKind::GreaterThanToken {
            ">"
        } else {
            "}"
        };
        if token.kind != kind || text != expected {
            return Err(RescanError::UnexpectedToken);
        }
        Ok(())
    }

    fn extended_token(&self, token: Token<'a>, kind: SyntaxKind, unterminated: bool) -> Token<'a> {
        Token {
            kind,
            end: self.utf16,
            byte_end: self.byte,
            text: &self.source[token.byte_start..self.byte],
            unterminated,
            ..token
        }
    }

    fn peek(&self) -> Option<u8> {
        self.source.as_bytes().get(self.byte).copied()
    }

    fn rest(&self) -> &'a str {
        &self.source[self.byte..]
    }

    #[inline]
    fn bump(&mut self) -> Option<char> {
        let byte = self.peek()?;
        if byte.is_ascii() {
            self.byte += 1;
            self.utf16 += 1;
            Some(byte as char)
        } else {
            let ch = self.rest().chars().next()?;
            self.byte += ch.len_utf8();
            self.utf16 += ch.len_utf16();
            Some(ch)
        }
    }

    fn ascii(&mut self, count: usize) {
        debug_assert!(self.source.as_bytes()[self.byte..self.byte + count].is_ascii());
        self.byte += count;
        self.utf16 += count;
    }

    fn error(&mut self, code: DiagnosticCode, start: usize, length: usize, message: &'static str) {
        self.diagnostics.push(Diagnostic {
            code,
            start,
            length,
            message,
        });
    }

    fn line_comment(&mut self) {
        while self.rest().chars().next().is_some_and(|ch| !line_break(ch)) {
            self.bump();
        }
    }

    fn block_comment(&mut self) -> bool {
        self.ascii(2);
        while self.peek().is_some() {
            if self.rest().starts_with("*/") {
                self.ascii(2);
                return false;
            }
            self.bump();
        }
        self.error(UnterminatedComment, self.utf16, 0, "'*/' expected.");
        true
    }

    fn quoted(&mut self, quote: u8, start: usize) -> bool {
        self.ascii(1);
        while let Some(byte) = self.peek() {
            if byte == quote {
                self.ascii(1);
                return false;
            }
            if matches!(byte, b'\r' | b'\n') {
                break;
            }
            if byte == b'\\' {
                self.escape(false);
            } else {
                self.bump();
            }
        }
        self.error(
            UnterminatedString,
            start,
            self.utf16 - start,
            "Unterminated string literal.",
        );
        true
    }

    fn template(&mut self, start: usize, continuation: bool) -> (SyntaxKind, bool) {
        self.ascii(1);
        while self.peek().is_some() {
            if self.peek() == Some(b'`') {
                self.ascii(1);
                return (
                    if continuation {
                        SyntaxKind::TemplateTail
                    } else {
                        SyntaxKind::NoSubstitutionTemplateLiteral
                    },
                    false,
                );
            }
            if self.rest().starts_with("${") {
                self.ascii(2);
                return (
                    if continuation {
                        SyntaxKind::TemplateMiddle
                    } else {
                        SyntaxKind::TemplateHead
                    },
                    false,
                );
            }
            if self.peek() == Some(b'\\') {
                // Invalid escapes in tagged templates are parser-sensitive.
                self.escape(true);
            } else {
                self.bump();
            }
        }
        self.error(
            UnterminatedTemplate,
            start,
            self.utf16 - start,
            "Unterminated template literal.",
        );
        (
            if continuation {
                SyntaxKind::TemplateTail
            } else {
                SyntaxKind::NoSubstitutionTemplateLiteral
            },
            true,
        )
    }

    fn escape(&mut self, template: bool) {
        let start = self.utf16;
        self.ascii(1);
        match self.peek() {
            Some(b'\r') => {
                self.ascii(1);
                if self.peek() == Some(b'\n') {
                    self.ascii(1);
                }
            }
            Some(b'x' | b'u') => {
                let unicode = self.peek() == Some(b'u');
                self.ascii(1);
                if unicode && self.peek() == Some(b'{') {
                    self.ascii(1);
                    let mut digits = 0;
                    let mut value = 0_u32;
                    while let Some(ch) = self.peek().filter(u8::is_ascii_hexdigit) {
                        value = value
                            .saturating_mul(16)
                            .saturating_add((ch as char).to_digit(16).unwrap());
                        digits += 1;
                        self.ascii(1);
                    }
                    let closed = self.peek() == Some(b'}');
                    if closed {
                        self.ascii(1);
                    }
                    if (!closed || digits == 0 || value > 0x10ffff) && !template {
                        self.error(
                            InvalidUnicodeEscape,
                            start,
                            self.utf16 - start,
                            "Invalid Unicode escape sequence.",
                        );
                    }
                } else {
                    let expected = if unicode { 4 } else { 2 };
                    let mut digits = 0;
                    while digits < expected && self.peek().is_some_and(|ch| ch.is_ascii_hexdigit())
                    {
                        digits += 1;
                        self.ascii(1);
                    }
                    if digits != expected && !template {
                        self.error(
                            HexDigitExpected,
                            start,
                            self.utf16 - start,
                            "Hexadecimal digit expected.",
                        );
                    }
                }
            }
            Some(_) => {
                self.bump();
            }
            None => {}
        }
    }

    fn unicode_escape(text: &str) -> Option<(char, usize)> {
        let bytes = text.as_bytes();
        if !bytes.starts_with(b"\\u") {
            return None;
        }
        let mut value = 0_u32;
        let mut i = 2;
        if bytes.get(i) == Some(&b'{') {
            i += 1;
            let start = i;
            while let Some(digit) = bytes.get(i).and_then(|&ch| (ch as char).to_digit(16)) {
                value = value.checked_mul(16)?.checked_add(digit)?;
                i += 1;
            }
            if i == start || bytes.get(i) != Some(&b'}') {
                return None;
            }
            i += 1;
        } else {
            for _ in 0..4 {
                value = value * 16 + (*bytes.get(i)? as char).to_digit(16)?;
                i += 1;
            }
        }
        Some((char::from_u32(value)?, i))
    }

    fn identifier(&mut self, private: bool) -> Option<SyntaxKind> {
        let start = self.byte;
        let mut cooked = None::<String>;
        let mut segment = start;
        let mut first = true;
        while let Some(ch) = self.rest().chars().next() {
            let predicate = if first { is_start } else { is_part };
            if ch == '\\' {
                let Some((escaped, bytes)) =
                    Self::unicode_escape(self.rest()).filter(|&(ch, _)| predicate(ch))
                else {
                    break;
                };
                let value = cooked.get_or_insert_with(String::new);
                value.push_str(&self.source[segment..self.byte]);
                value.push(escaped);
                self.ascii(bytes);
                segment = self.byte;
            } else if predicate(ch) {
                self.bump();
            } else {
                break;
            }
            first = false;
        }
        if first {
            return None;
        }
        if private {
            return Some(SyntaxKind::PrivateIdentifier);
        }
        let value = match cooked {
            Some(mut value) => {
                value.push_str(&self.source[segment..self.byte]);
                Cow::Owned(value)
            }
            None => Cow::Borrowed(&self.source[start..self.byte]),
        };
        Some(keyword(&value))
    }

    fn digits(&mut self, radix: u32, check_separators: bool) -> usize {
        let mut count = 0;
        let mut separator_allowed = false;
        let mut last_separator = false;
        while let Some(ch) = self.peek() {
            if ch == b'_' {
                if check_separators && !separator_allowed {
                    self.error(
                        InvalidNumericSeparator,
                        self.utf16,
                        1,
                        "Multiple consecutive numeric separators are not permitted.",
                    );
                }
                separator_allowed = false;
                last_separator = true;
                self.ascii(1);
            } else if (ch as char).is_digit(radix) {
                count += 1;
                separator_allowed = true;
                last_separator = false;
                self.ascii(1);
            } else {
                break;
            }
        }
        if check_separators && last_separator {
            self.error(
                InvalidNumericSeparator,
                self.utf16 - 1,
                1,
                "Numeric separators are not allowed here.",
            );
        }
        count
    }

    fn number(&mut self) -> SyntaxKind {
        let start = self.byte;
        let utf16_start = self.utf16;
        let mut fractional = false;
        let mut leading_zero = false;
        if self.peek() == Some(b'0') {
            let radix = match self.source.as_bytes().get(self.byte + 1) {
                Some(b'x' | b'X') => Some(16),
                Some(b'b' | b'B') => Some(2),
                Some(b'o' | b'O') => Some(8),
                _ => None,
            };
            if let Some(radix) = radix.filter(|_| self.byte + 2 < self.source.len()) {
                self.ascii(2);
                if self.digits(radix, true) == 0 {
                    self.error(DigitExpected, self.utf16, 0, "Digit expected.");
                }
                return self.bigint_suffix();
            }
            let next = self.source.as_bytes().get(start + 1).copied();
            if next.is_some_and(|ch| ch.is_ascii_digit()) {
                self.ascii(1);
                let mut octal = true;
                while let Some(ch) = self.peek().filter(u8::is_ascii_digit) {
                    octal &= ch <= b'7';
                    self.ascii(1);
                }
                self.error(
                    LegacyLeadingZero,
                    utf16_start,
                    self.utf16 - utf16_start,
                    "Octal and decimal literals with leading zeros are not allowed.",
                );
                if octal {
                    return SyntaxKind::NumericLiteral;
                }
                leading_zero = true;
            } else {
                self.digits(10, true);
            }
        } else {
            self.digits(10, true);
        }
        if self.peek() == Some(b'.') {
            fractional = true;
            self.ascii(1);
            self.digits(10, true);
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            fractional = true;
            self.ascii(1);
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.ascii(1);
            }
            if self.digits(10, true) == 0 {
                self.error(DigitExpected, self.utf16, 0, "Digit expected.");
            }
        }
        let kind = if fractional || leading_zero {
            SyntaxKind::NumericLiteral
        } else {
            self.bigint_suffix()
        };
        if !leading_zero {
            self.numeric_identifier();
        }
        kind
    }

    fn bigint_suffix(&mut self) -> SyntaxKind {
        if self.peek() == Some(b'n') {
            self.ascii(1);
            SyntaxKind::BigIntLiteral
        } else {
            SyntaxKind::NumericLiteral
        }
    }

    fn numeric_identifier(&mut self) {
        // Upstream consumes a sole `n` after fractional/exponent literals too.
        if self.peek() == Some(b'n')
            && !self.source[self.byte + 1..]
                .chars()
                .next()
                .is_some_and(is_part)
            && !Self::unicode_escape(&self.source[self.byte + 1..])
                .is_some_and(|(ch, _)| is_part(ch))
        {
            self.error(
                BigIntMustBeInteger,
                self.utf16,
                1,
                "A bigint literal must be an integer.",
            );
            self.ascii(1);
        } else if self.rest().chars().next().is_some_and(is_start) {
            self.error(
                IdentifierAfterNumber,
                self.utf16,
                1,
                "An identifier or keyword cannot immediately follow a numeric literal.",
            );
        }
    }

    fn conflict_marker(&self) -> Option<u8> {
        let first = self.peek()?;
        if !matches!(first, b'<' | b'>' | b'=' | b'|') {
            return None;
        }
        if self.byte != 0
            && !self.source[..self.byte]
                .chars()
                .next_back()
                .is_some_and(line_break)
        {
            return None;
        }
        let prefix = self.source.as_bytes().get(self.byte..self.byte + 7)?;
        if !prefix.iter().all(|&byte| byte == first) {
            return None;
        }
        if matches!(first, b'<' | b'>') && self.source.as_bytes().get(self.byte + 7) != Some(&b' ')
        {
            return None;
        }
        Some(first)
    }

    fn conflict(&mut self, marker: u8, start: usize) {
        if matches!(marker, b'<' | b'>') {
            self.line_comment();
        } else {
            self.ascii(7);
            while self.peek().is_some() {
                if matches!(self.conflict_marker(), Some(b'=' | b'>')) {
                    break;
                }
                self.bump();
            }
        }
        self.error(
            MergeConflictMarker,
            start,
            7,
            "Merge conflict marker encountered.",
        );
    }

    fn punctuation(&mut self) -> SyntaxKind {
        use SyntaxKind::*;
        let first = self.peek().unwrap();
        // Dispatch on the first byte; longest spelling wins within that group.
        let choices: &[(&str, SyntaxKind)] = match first {
            b'=' => &[
                ("===", EqualsEqualsEqualsToken),
                ("==", EqualsEqualsToken),
                ("=>", EqualsGreaterThanToken),
                ("=", EqualsToken),
            ],
            b'!' => &[
                ("!==", ExclamationEqualsEqualsToken),
                ("!=", ExclamationEqualsToken),
                ("!", ExclamationToken),
            ],
            b'+' => &[
                ("++", PlusPlusToken),
                ("+=", PlusEqualsToken),
                ("+", PlusToken),
            ],
            b'-' => &[
                ("--", MinusMinusToken),
                ("-=", MinusEqualsToken),
                ("-", MinusToken),
            ],
            b'*' => &[
                ("**=", AsteriskAsteriskEqualsToken),
                ("**", AsteriskAsteriskToken),
                ("*=", AsteriskEqualsToken),
                ("*", AsteriskToken),
            ],
            b'/' => &[("/=", SlashEqualsToken), ("/", SlashToken)],
            b'%' => &[("%=", PercentEqualsToken), ("%", PercentToken)],
            b'<' => &[
                ("<<=", LessThanLessThanEqualsToken),
                ("<<", LessThanLessThanToken),
                ("<=", LessThanEqualsToken),
                ("<", LessThanToken),
            ],
            b'>' => &[(">", GreaterThanToken)],
            b'&' => &[
                ("&&=", AmpersandAmpersandEqualsToken),
                ("&&", AmpersandAmpersandToken),
                ("&=", AmpersandEqualsToken),
                ("&", AmpersandToken),
            ],
            b'|' => &[
                ("||=", BarBarEqualsToken),
                ("||", BarBarToken),
                ("|=", BarEqualsToken),
                ("|", BarToken),
            ],
            b'^' => &[("^=", CaretEqualsToken), ("^", CaretToken)],
            b'?' => &[
                ("??=", QuestionQuestionEqualsToken),
                ("??", QuestionQuestionToken),
                ("?.", QuestionDotToken),
                ("?", QuestionToken),
            ],
            b'.' => &[("...", DotDotDotToken), (".", DotToken)],
            b'{' => &[("{", OpenBraceToken)],
            b'}' => &[("}", CloseBraceToken)],
            b'(' => &[("(", OpenParenToken)],
            b')' => &[(")", CloseParenToken)],
            b'[' => &[("[", OpenBracketToken)],
            b']' => &[("]", CloseBracketToken)],
            b';' => &[(";", SemicolonToken)],
            b',' => &[(",", CommaToken)],
            b':' => &[(":", ColonToken)],
            b'~' => &[("~", TildeToken)],
            b'@' => &[("@", AtToken)],
            _ => &[],
        };
        for &(text, kind) in choices {
            if self.rest().starts_with(text) {
                if kind == QuestionDotToken
                    && self
                        .source
                        .as_bytes()
                        .get(self.byte + 2)
                        .is_some_and(u8::is_ascii_digit)
                {
                    continue;
                }
                self.ascii(text.len());
                return kind;
            }
        }
        let start = self.utf16;
        self.bump();
        self.error(
            InvalidCharacter,
            start,
            self.utf16 - start,
            "Invalid character.",
        );
        Unknown
    }
}

impl<'a> Iterator for Scanner<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        let byte_start = self.byte;
        let start = self.utf16;
        let mut unterminated = false;
        let kind = if let Some(ch) = self.rest().chars().next() {
            if whitespace(ch) {
                while self.rest().chars().next().is_some_and(whitespace) {
                    self.bump();
                }
                SyntaxKind::WhitespaceTrivia
            } else if line_break(ch) {
                self.bump();
                if ch == '\r' && self.peek() == Some(b'\n') {
                    self.ascii(1);
                }
                SyntaxKind::NewLineTrivia
            } else if self.byte == 0 && self.rest().starts_with("#!") {
                self.line_comment();
                SyntaxKind::Unknown // TypeScript 7 no longer has ShebangTrivia.
            } else if let Some(marker) = self.conflict_marker() {
                self.conflict(marker, start);
                SyntaxKind::ConflictMarkerTrivia
            } else if self.rest().starts_with("//") {
                self.line_comment();
                SyntaxKind::SingleLineCommentTrivia
            } else if self.rest().starts_with("/*") {
                unterminated = self.block_comment();
                SyntaxKind::MultiLineCommentTrivia
            } else if matches!(self.peek(), Some(b'\'' | b'"')) {
                unterminated = self.quoted(self.peek().unwrap(), start);
                SyntaxKind::StringLiteral
            } else if ch == '`' {
                let result = self.template(start, false);
                unterminated = result.1;
                result.0
            } else if ch.is_ascii_digit()
                || (ch == '.'
                    && self
                        .source
                        .as_bytes()
                        .get(self.byte + 1)
                        .is_some_and(u8::is_ascii_digit))
            {
                self.number()
            } else if ch == '#' && self.rest().starts_with("#!") {
                self.ascii(1);
                self.error(InvalidCharacter, start, 1, "Invalid character.");
                SyntaxKind::Unknown
            } else if ch == '#' {
                self.ascii(1);
                if self.identifier(true).is_none() {
                    self.error(
                        InvalidPrivateIdentifier,
                        start,
                        1,
                        "A private identifier requires an identifier after '#'.",
                    );
                }
                SyntaxKind::PrivateIdentifier
            } else if let Some(kind) = self.identifier(false) {
                kind
            } else if ch == '\u{fffd}' {
                while self.bump().is_some() {}
                self.error(
                    BinaryFile,
                    start,
                    self.utf16 - start,
                    "File appears to be binary.",
                );
                SyntaxKind::NonTextFileMarkerTrivia
            } else {
                self.punctuation()
            }
        } else {
            self.finished = true;
            SyntaxKind::EndOfFile
        };
        debug_assert!(self.byte > byte_start || self.finished);
        Some(Token {
            kind,
            start,
            end: self.utf16,
            byte_start,
            byte_end: self.byte,
            text: &self.source[byte_start..self.byte],
            unterminated,
        })
    }
}

impl FusedIterator for Scanner<'_> {}

fn line_break(ch: char) -> bool {
    matches!(ch, '\r' | '\n' | '\u{2028}' | '\u{2029}')
}

fn whitespace(ch: char) -> bool {
    matches!(
        ch,
        '\t' | '\u{b}' | '\u{c}' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200b}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

/// Rescans require the current token from this scanner, before advancing again.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RescanError {
    StaleToken,
    UnexpectedToken,
}

impl std::fmt::Display for RescanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::StaleToken => "rescan requires the current token from this scanner",
            Self::UnexpectedToken => "token kind cannot be rescanned in this context",
        })
    }
}

impl std::error::Error for RescanError {}
