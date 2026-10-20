use crate::ast::*;
use crate::{Scanner, SyntaxKind, Token};
use ParseDiagnosticCode::*;
use SyntaxKind::*;

/// Parse one expression, rejecting unsupported syntax and unconsumed tokens.
/// This is not a source-file parser, semantic checker, or compilation API.
/// Nesting is bounded to protect callers from stack exhaustion on hostile input.
pub fn parse_expression(source: &str) -> ParsedExpression<'_> {
    let mut scanner = Scanner::new(source);
    let (current, line_break) = next_significant(&mut scanner);
    let mut parser = Parser {
        scanner,
        current,
        line_break,
        nodes: Vec::new(),
        diagnostics: Vec::new(),
        depth: 0,
    };
    let root = parser.expression(1);
    if parser.current.kind != EndOfFile {
        parser.error(
            UnsupportedSyntax,
            "Syntax is not supported by the expression parser.",
        );
    }
    // Scan remaining input so lexical errors cannot disappear after a parse error.
    parser.scanner.by_ref().for_each(drop);
    ParsedExpression {
        root,
        nodes: parser.nodes,
        diagnostics: parser.diagnostics,
        lexical_diagnostics: parser.scanner.diagnostics().to_vec(),
    }
}

struct Parser<'a> {
    scanner: Scanner<'a>,
    current: Token<'a>,
    line_break: bool,
    nodes: Vec<Expr<'a>>,
    diagnostics: Vec<ParseDiagnostic>,
    depth: usize,
}

fn next_significant<'a>(scanner: &mut Scanner<'a>) -> (Token<'a>, bool) {
    let mut line_break = false;
    loop {
        let token = scanner.next().expect("parser never advances past EOF");
        if token.kind.is_trivia() {
            line_break |= token
                .text
                .chars()
                .any(|ch| matches!(ch, '\r' | '\n' | '\u{2028}' | '\u{2029}'));
        } else {
            return (token, line_break);
        }
    }
}

impl<'a> Parser<'a> {
    fn advance(&mut self) -> Token<'a> {
        let previous = self.current;
        if previous.kind != EndOfFile {
            (self.current, self.line_break) = next_significant(&mut self.scanner);
        }
        previous
    }
    fn error(&mut self, code: ParseDiagnosticCode, message: &'static str) {
        self.diagnostics.push(ParseDiagnostic {
            code,
            span: Span::token(self.current),
            message,
        });
    }
    fn expect(&mut self, kind: SyntaxKind) -> Option<Token<'a>> {
        if self.current.kind == kind {
            Some(self.advance())
        } else {
            self.error(
                TokenExpected,
                "Expected a closing delimiter or punctuation token.",
            );
            None
        }
    }
    fn add(&mut self, span: Span, kind: ExprKind<'a>) -> ExprId {
        let id = ExprId(self.nodes.len());
        self.nodes.push(Expr { span, kind });
        id
    }
    fn span(&self, id: ExprId) -> Span {
        self.nodes[id.0].span
    }
    fn expression(&mut self, min: u8) -> Option<ExprId> {
        if self.depth >= 128 {
            self.error(
                NestingLimit,
                "Expression nesting exceeds the supported limit of 128.",
            );
            return None;
        }
        self.depth += 1;
        let result = self.expression_inner(min);
        self.depth -= 1;
        result
    }
    fn expression_inner(&mut self, min: u8) -> Option<ExprId> {
        let mut left = self.prefix()?;
        loop {
            // `>` was deliberately not combined during ordinary scanning.
            if self.current.kind == GreaterThanToken {
                self.current = self
                    .scanner
                    .rescan_greater(self.current)
                    .expect("current parser token belongs to scanner");
            }
            if min <= 3 && self.current.kind == QuestionToken {
                self.advance();
                let when_true = self.expression(2)?;
                self.expect(ColonToken)?;
                let when_false = self.expression(2)?;
                left = self.add(
                    self.span(left).through(self.span(when_false)),
                    ExprKind::Conditional {
                        condition: left,
                        when_true,
                        when_false,
                    },
                );
                continue;
            }
            let Some((precedence, right_associative)) = binary_precedence(self.current.kind) else {
                break;
            };
            if precedence < min {
                break;
            }
            let operator = self.advance();
            if precedence == 2 && !self.assignment_target(left) {
                self.diagnostics.push(ParseDiagnostic {
                    code: InvalidAssignmentTarget,
                    span: self.span(left),
                    message: "The left side is not a supported assignment target.",
                });
            }
            if operator.kind == AsteriskAsteriskToken
                && matches!(
                    self.nodes[left.0].kind,
                    ExprKind::Unary { operator: unary, postfix: false, .. } if !matches!(unary.kind, PlusPlusToken | MinusMinusToken)
                )
            {
                self.diagnostics.push(ParseDiagnostic { code:InvalidExponentiation,span:self.span(left),message:"A unary expression on the left of exponentiation requires parentheses." });
            }
            let right = self.expression(if right_associative {
                precedence
            } else {
                precedence + 1
            })?;
            if mixed_nullish(operator.kind, &self.nodes[left.0].kind)
                || mixed_nullish(operator.kind, &self.nodes[right.0].kind)
            {
                self.diagnostics.push(ParseDiagnostic { code:MixedNullishAndLogical,span:Span::token(operator),message:"Nullish coalescing and logical operators require explicit parentheses when mixed." });
            }
            left = self.add(
                self.span(left).through(self.span(right)),
                ExprKind::Binary {
                    operator,
                    left,
                    right,
                },
            );
        }
        Some(left)
    }
    fn prefix(&mut self) -> Option<ExprId> {
        if matches!(
            self.current.kind,
            PlusToken
                | MinusToken
                | ExclamationToken
                | TildeToken
                | TypeOfKeyword
                | VoidKeyword
                | DeleteKeyword
                | PlusPlusToken
                | MinusMinusToken
        ) {
            let operator = self.advance();
            let operand = self.expression(17)?;
            if matches!(operator.kind, PlusPlusToken | MinusMinusToken)
                && !self.assignment_target(operand)
            {
                self.error(
                    InvalidAssignmentTarget,
                    "An update expression requires an assignment target.",
                );
            }
            let span = Span::token(operator).through(self.span(operand));
            return Some(self.add(
                span,
                ExprKind::Unary {
                    operator,
                    operand,
                    postfix: false,
                },
            ));
        }
        let primary = self.primary()?;
        self.postfix(primary)
    }
    fn primary(&mut self) -> Option<ExprId> {
        let token = self.current;
        match token.kind {
            Identifier | ThisKeyword | SuperKeyword => {
                self.advance();
                Some(self.add(Span::token(token), ExprKind::Identifier { token }))
            }
            NumericLiteral
            | BigIntLiteral
            | StringLiteral
            | NoSubstitutionTemplateLiteral
            | TrueKeyword
            | FalseKeyword
            | NullKeyword => {
                self.advance();
                Some(self.add(Span::token(token), ExprKind::Literal { token }))
            }
            OpenParenToken => {
                self.advance();
                let expression = self.expression(1)?;
                let close = self.expect(CloseParenToken)?;
                Some(self.add(
                    Span::token(token).through(Span::token(close)),
                    ExprKind::Parenthesized { expression },
                ))
            }
            OpenBracketToken => self.array(),
            TemplateHead => self.template(),
            EndOfFile | CloseParenToken | CloseBracketToken | CloseBraceToken | CommaToken
            | ColonToken | SemicolonToken => {
                self.error(ExpressionExpected, "Expression expected.");
                None
            }
            _ => {
                self.error(
                    UnsupportedSyntax,
                    "This expression form is not implemented yet.",
                );
                None
            }
        }
    }
    fn postfix(&mut self, mut expression: ExprId) -> Option<ExprId> {
        loop {
            let mut optional = false;
            if self.current.kind == QuestionDotToken {
                optional = true;
                self.advance();
            }
            match self.current.kind {
                OpenParenToken => {
                    self.advance();
                    let mut arguments = Vec::new();
                    while self.current.kind != CloseParenToken && self.current.kind != EndOfFile {
                        arguments.push(self.argument()?);
                        if self.current.kind != CommaToken {
                            break;
                        }
                        self.advance();
                    }
                    let close = self.expect(CloseParenToken)?;
                    expression = self.add(
                        self.span(expression).through(Span::token(close)),
                        ExprKind::Call {
                            callee: expression,
                            arguments,
                            optional,
                        },
                    );
                }
                OpenBracketToken => {
                    self.advance();
                    let index = self.expression(1)?;
                    let close = self.expect(CloseBracketToken)?;
                    expression = self.add(
                        self.span(expression).through(Span::token(close)),
                        ExprKind::Element {
                            object: expression,
                            index,
                            optional,
                        },
                    );
                }
                DotToken if !optional => {
                    self.advance();
                    expression = self.property(expression, false)?;
                }
                _ if optional => {
                    expression = self.property(expression, true)?;
                }
                PlusPlusToken | MinusMinusToken if !self.line_break => {
                    if !self.assignment_target(expression) {
                        self.error(
                            InvalidAssignmentTarget,
                            "An update expression requires an assignment target.",
                        );
                    }
                    let operator = self.advance();
                    expression = self.add(
                        self.span(expression).through(Span::token(operator)),
                        ExprKind::Unary {
                            operator,
                            operand: expression,
                            postfix: true,
                        },
                    );
                    // Further member access requires a parenthesized update expression.
                    break;
                }
                _ => break,
            }
        }
        Some(expression)
    }
    fn property(&mut self, object: ExprId, optional: bool) -> Option<ExprId> {
        let name = self.current;
        if !matches!(name.kind, Identifier | PrivateIdentifier)
            && (name.kind as u16) < (BreakKeyword as u16)
        {
            self.error(
                TokenExpected,
                "A property name is required after member access.",
            );
            return None;
        }
        self.advance();
        Some(self.add(
            self.span(object).through(Span::token(name)),
            ExprKind::Property {
                object,
                name,
                optional,
            },
        ))
    }
    fn assignment_target(&self, mut id: ExprId) -> bool {
        loop {
            match &self.nodes[id.0].kind {
                ExprKind::Identifier { token } => {
                    return !matches!(token.kind, ThisKeyword | SuperKeyword);
                }
                ExprKind::Property { .. } | ExprKind::Element { .. } => {
                    return !self.optional_chain(id);
                }
                ExprKind::Parenthesized { expression } => id = *expression,
                _ => return false,
            }
        }
    }
    fn optional_chain(&self, mut id: ExprId) -> bool {
        loop {
            let (optional, next) = match &self.nodes[id.0].kind {
                ExprKind::Property {
                    object, optional, ..
                }
                | ExprKind::Element {
                    object, optional, ..
                } => (*optional, *object),
                ExprKind::Call {
                    callee, optional, ..
                } => (*optional, *callee),
                _ => return false,
            };
            if optional {
                return true;
            }
            id = next;
        }
    }
    fn argument(&mut self) -> Option<ExprId> {
        if self.current.kind == DotDotDotToken {
            let start = self.advance();
            let expression = self.expression(2)?;
            Some(self.add(
                Span::token(start).through(self.span(expression)),
                ExprKind::Spread { expression },
            ))
        } else {
            self.expression(2)
        }
    }
    fn array(&mut self) -> Option<ExprId> {
        let open = self.advance();
        let mut elements = Vec::new();
        while self.current.kind != CloseBracketToken && self.current.kind != EndOfFile {
            if self.current.kind == CommaToken {
                elements.push(None);
                self.advance();
                continue;
            }
            elements.push(Some(self.argument()?));
            if self.current.kind != CommaToken {
                break;
            }
            self.advance();
        }
        let close = self.expect(CloseBracketToken)?;
        Some(self.add(
            Span::token(open).through(Span::token(close)),
            ExprKind::Array { elements },
        ))
    }
    fn template(&mut self) -> Option<ExprId> {
        let head = self.advance();
        let mut parts = vec![head];
        let mut expressions = Vec::new();
        let tail = loop {
            expressions.push(self.expression(1)?);
            if self.current.kind != CloseBraceToken {
                self.error(
                    TokenExpected,
                    "A template substitution requires a closing brace.",
                );
                return None;
            }
            let continuation = self
                .scanner
                .rescan_template(self.current)
                .expect("current template closing brace belongs to scanner");
            self.current = continuation;
            let part = self.advance();
            parts.push(part);
            if part.kind == TemplateTail {
                break part;
            }
        };
        Some(self.add(
            Span::token(head).through(Span::token(tail)),
            ExprKind::Template { parts, expressions },
        ))
    }
}

fn binary_precedence(kind: SyntaxKind) -> Option<(u8, bool)> {
    Some(match kind {
        CommaToken => (1, false),
        EqualsToken
        | PlusEqualsToken
        | MinusEqualsToken
        | AsteriskEqualsToken
        | AsteriskAsteriskEqualsToken
        | SlashEqualsToken
        | PercentEqualsToken
        | LessThanLessThanEqualsToken
        | GreaterThanGreaterThanEqualsToken
        | GreaterThanGreaterThanGreaterThanEqualsToken
        | AmpersandEqualsToken
        | BarEqualsToken
        | CaretEqualsToken
        | AmpersandAmpersandEqualsToken
        | BarBarEqualsToken
        | QuestionQuestionEqualsToken => (2, true),
        QuestionQuestionToken => (4, false),
        BarBarToken => (5, false),
        AmpersandAmpersandToken => (6, false),
        BarToken => (7, false),
        CaretToken => (8, false),
        AmpersandToken => (9, false),
        EqualsEqualsToken
        | ExclamationEqualsToken
        | EqualsEqualsEqualsToken
        | ExclamationEqualsEqualsToken => (10, false),
        LessThanToken
        | LessThanEqualsToken
        | GreaterThanToken
        | GreaterThanEqualsToken
        | InKeyword
        | InstanceOfKeyword => (11, false),
        LessThanLessThanToken
        | GreaterThanGreaterThanToken
        | GreaterThanGreaterThanGreaterThanToken => (12, false),
        PlusToken | MinusToken => (13, false),
        AsteriskToken | SlashToken | PercentToken => (14, false),
        AsteriskAsteriskToken => (16, true),
        _ => return None,
    })
}

fn mixed_nullish(operator: SyntaxKind, operand: &ExprKind<'_>) -> bool {
    let ExprKind::Binary {
        operator: child, ..
    } = operand
    else {
        return false;
    };
    (operator == QuestionQuestionToken
        && matches!(child.kind, AmpersandAmpersandToken | BarBarToken))
        || (matches!(operator, AmpersandAmpersandToken | BarBarToken)
            && child.kind == QuestionQuestionToken)
}
