//! Typed, flat expression storage. Node IDs avoid recursive ownership/drop and
//! keep AST lifetime bounded by the parse result; spellings borrow the source.
use serde::Serialize;

use crate::{Diagnostic, Token};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ExprId(pub(crate) usize);

impl ExprId {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub byte_start: usize,
    pub byte_end: usize,
}

impl Span {
    pub(crate) fn token(token: Token<'_>) -> Self {
        Self {
            start: token.start,
            end: token.end,
            byte_start: token.byte_start,
            byte_end: token.byte_end,
        }
    }
    pub(crate) fn through(self, end: Self) -> Self {
        Self {
            end: end.end,
            byte_end: end.byte_end,
            ..self
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Expr<'a> {
    pub span: Span,
    #[serde(flatten)]
    pub kind: ExprKind<'a>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind")]
pub enum ExprKind<'a> {
    Identifier {
        token: Token<'a>,
    },
    Literal {
        token: Token<'a>,
    },
    Parenthesized {
        expression: ExprId,
    },
    Unary {
        operator: Token<'a>,
        operand: ExprId,
        postfix: bool,
    },
    Binary {
        operator: Token<'a>,
        left: ExprId,
        right: ExprId,
    },
    Conditional {
        condition: ExprId,
        when_true: ExprId,
        when_false: ExprId,
    },
    Call {
        callee: ExprId,
        arguments: Vec<ExprId>,
        optional: bool,
    },
    Property {
        object: ExprId,
        name: Token<'a>,
        optional: bool,
    },
    Element {
        object: ExprId,
        index: ExprId,
        optional: bool,
    },
    Array {
        elements: Vec<Option<ExprId>>,
    },
    Spread {
        expression: ExprId,
    },
    Template {
        parts: Vec<Token<'a>>,
        expressions: Vec<ExprId>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseDiagnosticCode {
    ExpressionExpected,
    TokenExpected,
    UnexpectedToken,
    UnsupportedSyntax,
    NestingLimit,
    InvalidAssignmentTarget,
    InvalidExponentiation,
    MixedNullishAndLogical,
}

#[derive(Debug, Serialize)]
pub struct ParseDiagnostic {
    pub code: ParseDiagnosticCode,
    pub span: Span,
    pub message: &'static str,
}

/// No parser success is implied when either diagnostics list is nonempty.
#[derive(Debug, Serialize)]
pub struct ParsedExpression<'a> {
    pub root: Option<ExprId>,
    pub nodes: Vec<Expr<'a>>,
    pub diagnostics: Vec<ParseDiagnostic>,
    pub lexical_diagnostics: Vec<Diagnostic>,
}

impl<'a> ParsedExpression<'a> {
    pub fn is_valid(&self) -> bool {
        self.root.is_some() && self.diagnostics.is_empty() && self.lexical_diagnostics.is_empty()
    }
    pub fn node(&self, id: ExprId) -> Option<&Expr<'a>> {
        self.nodes.get(id.0)
    }
}
