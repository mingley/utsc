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

## Expression parser milestone

`parse-expression` builds a flat typed AST with borrowed token spellings. It
requests greater-than operator and template continuation rescans from the
scanner at grammar boundaries. Calls, properties, elements, optional chains,
array holes/spreads, unary/update, binary/assignment/conditional expressions,
parentheses and nested template substitutions are implemented.

The native TypeScript 7.0.2 API is used to compare 1,532 expression trees and
23,383 visited nodes exactly on structure, operators, raw text and UTF-16 spans.
This uses the native upstream parser, not the JavaScript scanner API. Separate
Rust tests cover precedence and associativity, invalid targets, exponentiation,
nullish/logical mixing, comment line breaks, 4,000 malformed Unicode expressions,
a nesting bound of 128, and flat ownership of a 20,000-term tree.

Parser diagnostics have native identifiers. Error messages/codes/recovery are
not yet upstream-compatible. Unsupported expression forms and leftover tokens
are rejected explicitly. Contextual keyword identifiers, object literals,
regular expressions, functions/arrows, type assertions, await/yield, JSX and
whole statements/source files are not implemented by this parser. Grammar and
semantic validity across all TypeScript contexts are not claimed.

## Not yet supported

- Parser-led rescanning for regex literals; JSX and JSDoc lexical modes. The
  raw scanner returns individual `>` tokens and `/` punctuation by design. It stops a template head after `${`;
  the expression parser requests continuations while raw scanning still leaves
  that decision to the caller.
- Cooked string/numeric values, exact token flags, complete lexical validation,
  strict/target-dependent legacy literal rules, or TSxxxx diagnostic parity.
- Lone UTF-16 surrogates and non-UTF-8 source encodings.
- Whole-source-file/type grammar, binding, checking, standard libraries, module
  resolution, tsconfig, transforms/emit, declaration/map output, project references,
  incremental builds,
  watch mode, language server, or the upstream compiler conformance suite.

No percentage of full TypeScript compliance is implied. A successful scan only
means that this scanner reported no lexical errors it currently knows about.
Future production claims require the gates in architecture.md, including invalid
program diagnostics, emitted artifacts, project-level behavior, and memory and
cancellation tests for long-lived sessions.
