//! Native TypeScript compiler infrastructure.
//!
//! Provides source positions, contextual token rescans, and a typed expression
//! parser for a documented subset. No source-file parser, checker, or emitter yet.

pub mod ast;
mod diagnostic;
mod kind;
mod parser;
mod scanner;
mod source;
mod unicode;

pub use diagnostic::{Diagnostic, DiagnosticCode};
pub use kind::SyntaxKind;
pub use parser::parse_expression;
pub use scanner::{RescanError, Scanner, Token};
pub use source::{LineColumn, SourceFile};

pub const TYPESCRIPT_VERSION: &str = "7.0.2";
