# Compatibility contract

The target is the latest stable TypeScript release, pinned to **7.0.2** for this
milestone (upstream commit `2bd066d87f5bafd315be9f40889d0a60b9e58e0b`).
The npm lockfile pins artifact integrity. Generated Rust token names, keyword
mappings and Unicode ranges come from that release; `npm run check:generated`
verifies reproducibility without modifying files. Updating the target requires
review and a fresh compatibility report, not a floating dependency.

TypeScript's current compiler behavior, its regression suite, and ECMAScript's
lexical rules are the references. The old TypeScript language specification
alone is not sufficient to define current checker behavior.

## Implemented and measured

- Context-free standard-mode scanning: trivia, keywords, escaped and Unicode
  identifiers, private names, punctuation, numeric and bigint spellings, quoted
  strings, template heads, shebangs, conflict markers, and error recovery.
- Borrowed raw token spellings, byte ranges, UTF-16 ranges, and untermination.
- UTF-16 line/column mapping, including CRLF, astral characters, and Unicode line
  separators. Invalid byte boundaries and positions inside surrogate pairs are
  rejected by reverse conversion.
- Named native lexical diagnostics and a native JSON/JSONL CLI.

The default differential corpus has 12,687 passing cases and 128,506 matching
tokens. It includes valid and malformed input. Whole upstream sources are tested
as raw scanner input; this does not imply that utsc can compile those files.

The comparison requires **exact kind, spelling, start/end, and untermination**.
It does not compare cooked values, full token flags, or diagnostic parity. Rust
regressions separately test termination, span bounds, text preservation, CLI
protocols, source mapping, and 10,000 deterministic Unicode inputs. These counts
are reproducible with the pinned dependencies and describe this corpus only.

## Deliberate differences from the unstable TypeScript 7 scanner API

Each of these has an explicit assertion on both the upstream observation and
utsc's result. They are outside the exact-match count, not skipped passing tests.

| Input | Upstream 7.0.2 raw scanner | utsc |
| --- | --- | --- |
| Bare `#` | Empty PrivateIdentifier, no cursor progress | Consumes `#` and reports invalid_private_identifier |
| U+0085 NEXT LINE | Consumes without returning a token | Preserves as Unknown and reports invalid_character |
| U+1680 OGHAM SPACE MARK | Unknown | WhitespaceTrivia |
| U+1685 OGHAM LETTER ONN | WhitespaceTrivia (upstream ogham constant is 5765) | Identifier |
| U+2028 LINE SEPARATOR | Consumes without returning a trivia token | NewLineTrivia |
| U+2029 PARAGRAPH SEPARATOR | Consumes without returning a trivia token | NewLineTrivia |

These assertions describe the exported unstable JavaScript scanner, not a claim
about diagnostics from TypeScript's native compiler. Native compiler behavior
must be checked independently as later compiler phases are implemented.

## Not yet supported

- Parser-led rescanning for regex literals, greater-than operators, or template
  tails; JSX and JSDoc lexical modes. The standard scanner returns individual `>`
  tokens and `/` punctuation by design. It stops a template head after `${`;
  without a parser the final backtick can be interpreted as a new template.
- Cooked string/numeric values, exact token flags, complete lexical validation,
  strict/target-dependent legacy literal rules, or TSxxxx diagnostic parity.
- Lone UTF-16 surrogates and non-UTF-8 source encodings.
- Parsing, binding, checking, standard libraries, module resolution, tsconfig,
  transforms/emit, declaration/map output, project references, incremental builds,
  watch mode, language server, or the upstream compiler conformance suite.

No percentage of full TypeScript compliance is implied. A successful scan only
means that this scanner reported no lexical errors it currently knows about.
Future production claims require the gates in architecture.md, including invalid
program diagnostics, emitted artifacts, project-level behavior, and memory and
cancellation tests for long-lived sessions.
