use utsc::ast::{ExprId, ExprKind, ParseDiagnosticCode};
use utsc::{SyntaxKind, parse_expression};

fn shape(source: &str) -> String {
    let parsed = parse_expression(source);
    assert!(parsed.is_valid(), "{source}: {parsed:?}");
    fn node(parsed: &utsc::ast::ParsedExpression<'_>, id: ExprId) -> String {
        match &parsed.node(id).unwrap().kind {
            ExprKind::Identifier { token } | ExprKind::Literal { token } => token.text.to_owned(),
            ExprKind::Binary {
                left,
                operator,
                right,
            } => format!(
                "({} {} {})",
                node(parsed, *left),
                operator.text,
                node(parsed, *right)
            ),
            ExprKind::Parenthesized { expression } => {
                format!("paren({})", node(parsed, *expression))
            }
            ExprKind::Unary {
                operator,
                operand,
                postfix,
            } => format!(
                "{}({},post={})",
                operator.text,
                node(parsed, *operand),
                postfix
            ),
            _ => panic!("unexpected test node"),
        }
    }
    node(&parsed, parsed.root.unwrap())
}
#[test]
fn operator_precedence_associativity_and_greater_rescans() {
    assert_eq!(shape("a + b * c ** d ** e"), "(a + (b * (c ** (d ** e))))");
    assert_eq!(shape("a = b += c"), "(a = (b += c))");
    assert_eq!(shape("a >>> b + c >= d"), "((a >>> (b + c)) >= d)");
    assert_eq!(shape("a, b = c"), "(a , (b = c))");
    assert_eq!(shape("(a + b) * c"), "(paren((a + b)) * c)");
    assert_eq!(shape("-a * b"), "(-(a,post=false) * b)");
    assert_eq!(shape("a ** -b"), "(a ** -(b,post=false))");
}
#[test]
fn nested_templates_and_optional_chains_produce_typed_nodes() {
    let source = "`😀${a ? `inner${b}` : c}tail${fn?.(x)?.[i]}end`";
    let parsed = parse_expression(source);
    assert!(parsed.is_valid(), "{parsed:?}");
    let root = parsed.node(parsed.root.unwrap()).unwrap();
    assert_eq!(root.span.end, source.encode_utf16().count());
    let ExprKind::Template { parts, expressions } = &root.kind else {
        panic!("expected template")
    };
    assert_eq!(parts.len(), 3);
    assert_eq!(expressions.len(), 2);
    assert_eq!(parts.last().unwrap().kind, SyntaxKind::TemplateTail);
    assert_eq!(
        parsed
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, ExprKind::Template { .. }))
            .count(),
        2
    );
}
#[test]
fn arrays_and_call_spreads_preserve_holes() {
    let parsed = parse_expression("fn([,x,,...xs], a ? b : c, ...rest,)");
    assert!(parsed.is_valid(), "{parsed:?}");
    let array = parsed
        .nodes
        .iter()
        .find_map(|node| {
            if let ExprKind::Array { elements } = &node.kind {
                Some(elements)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(array.len(), 4);
    assert_eq!(array.iter().filter(|e| e.is_none()).count(), 2);
}
#[test]
fn invalid_assignment_targets_and_grammar_restrictions_are_diagnostics() {
    for (source, code) in [
        ("a + b = c", ParseDiagnosticCode::InvalidAssignmentTarget),
        ("a?.b.c = d", ParseDiagnosticCode::InvalidAssignmentTarget),
        ("-a ** b", ParseDiagnosticCode::InvalidExponentiation),
        ("a ?? b || c", ParseDiagnosticCode::MixedNullishAndLogical),
        ("a && b ?? c", ParseDiagnosticCode::MixedNullishAndLogical),
    ] {
        let parsed = parse_expression(source);
        assert!(!parsed.is_valid(), "{source}");
        assert!(
            parsed.diagnostics.iter().any(|d| d.code == code),
            "{source}: {parsed:?}"
        );
    }
    assert!(parse_expression("(a?.b).c = d").is_valid());
    assert!(parse_expression("(a) = d").is_valid());
    assert!(parse_expression("++a ** b").is_valid());
}
#[test]
fn line_breaks_in_comments_prevent_postfix_update() {
    assert!(parse_expression("a /* comment */ ++").is_valid());
    assert!(!parse_expression("a /*\n*/ ++").is_valid());
    assert!(!parse_expression("a\u{2028}++").is_valid());
}
#[test]
fn unsupported_syntax_and_trailing_input_are_never_reported_as_success() {
    for source in [
        "",
        "a b",
        "x as number",
        "x => x",
        "new Foo()",
        "{}",
        "/pattern/g",
        "(a",
        "fn(,)",
        "`x${a",
        "'bad\n",
    ] {
        assert!(!parse_expression(source).is_valid(), "{source}");
    }
}
#[test]
fn nesting_limit_prevents_stack_overflow_and_arena_drop_remains_flat() {
    let source = format!("{}x{}", "(".repeat(10_000), ")".repeat(10_000));
    let parsed = parse_expression(&source);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.code == ParseDiagnosticCode::NestingLimit)
    );
    let flat = vec!["x"; 20_000].join("+");
    let parsed = parse_expression(&flat);
    assert!(parsed.is_valid());
    assert_eq!(parsed.nodes.len(), 39_999);
    assert!(serde_json::to_string(&parsed).is_ok());
}
#[test]
fn seeded_malformed_expressions_terminate_and_have_valid_span_boundaries() {
    let mut seed = 0x702_u32;
    let alphabet = [
        "a", "变量", "😀", " ", "\n", "(", ")", "[", "]", "{", "}", "`", "${", ".", "?", "+", "**",
        ">", ",", "'", "#", "/*",
    ];
    for _ in 0..4000 {
        let mut text = String::new();
        for _ in 0..40 {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            text.push_str(alphabet[seed as usize % alphabet.len()]);
        }
        let parsed = parse_expression(&text);
        for node in &parsed.nodes {
            assert!(text.get(node.span.byte_start..node.span.byte_end).is_some());
            assert_eq!(
                node.span.start,
                text[..node.span.byte_start].encode_utf16().count()
            );
            assert_eq!(
                node.span.end,
                text[..node.span.byte_end].encode_utf16().count()
            );
        }
    }
}
