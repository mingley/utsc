use serde::Serialize;

/// Native lexical diagnostic identifiers. These are not TypeScript TSxxxx codes:
/// matching upstream diagnostic codes, ranges, and messages is a later milestone.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    UnterminatedComment,
    UnterminatedString,
    UnterminatedTemplate,
    InvalidUnicodeEscape,
    HexDigitExpected,
    InvalidNumericSeparator,
    DigitExpected,
    LegacyLeadingZero,
    BigIntMustBeInteger,
    IdentifierAfterNumber,
    InvalidCharacter,
    InvalidPrivateIdentifier,
    BinaryFile,
    MergeConflictMarker,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub start: usize,
    pub length: usize,
    pub message: &'static str,
}
