//! Native TypeScript compiler infrastructure.
//!
//! This first milestone provides source positions and a context-free scanner.
//! It is not yet a parser, type checker, or JavaScript emitter.

mod diagnostic;
mod kind;
mod scanner;
mod source;
mod unicode;

pub use diagnostic::{Diagnostic, DiagnosticCode};
pub use kind::SyntaxKind;
pub use scanner::{Scanner, Token};
pub use source::{LineColumn, SourceFile};

pub const TYPESCRIPT_VERSION: &str = "7.0.2";
