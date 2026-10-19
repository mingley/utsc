# Architecture and implementation sequence

utsc targets native compilation in Rust. Node and the npm TypeScript package are
only development tools for data generation and differential tests. The shipped
scanner binary does not launch another compiler or require Node.

## Current ownership model

SourceFile owns UTF-8 source and its line index. Scanner borrows source and keeps
only cursors, its termination state, and per-source diagnostics. Tokens borrow
spelling and carry both byte spans and UTF-16 spans. The iterator emits trivia
and one EOF, then remains exhausted. It allocates no token strings for ordinary
identifiers; escaped identifiers allocate a temporary cooked name for keyword
classification. The CLI materializes tokens for JSON output, so CLI throughput
and memory are different from scanner-only measurements.

Token classification and Unicode tables are generated from the pinned upstream
release. Rust enums represent kinds and diagnostics. Native diagnostic codes
are intentionally distinct from TSxxxx until diagnostic parity is implemented.
No shared compiler cache, reference-counted AST graph, or unsafe code is needed
for this milestone. Keep modules small; add crates when ownership and dependency
boundaries are demonstrated rather than preallocating a large empty workspace.

## Reference review

Reviewed pingdotgg/ts-rust at 780dc512d64f40afebffa6a7028207649fb7ed1c,
including its README, scanner, package boundaries, ownership notes, and development
recommendations. Its reported results are claims about that project, not evidence
for utsc. No implementation is copied. It is useful for identifying behavior to
investigate, memory ownership hazards, and integration costs in a large direct
port. The official pinned TypeScript release remains the compatibility authority.

## Milestones and gates

1. **Current:** source positions, context-free tokenization, native CLI, reproducible
   generation, differential corpus, regression tests, and scanner benchmarks.
2. **Front end:** cooked literals, exact flags/diagnostics, parser-directed slash,
   greater-than and template rescans, JSX/JSDoc modes, typed AST, precedence and
   error recovery. Gate each feature on upstream syntax baselines, including invalid
   input and source ranges. Preserve source fidelity and deterministic ordering.
3. **Programs and binding:** compiler options and tsconfig inheritance, file loading,
   module resolution, standard libraries, symbols, declaration merging and scopes.
   Use typed arena indices and per-program interners where measurements justify
   them. Keep immutable data shareable and ownership of mutable state explicit.
4. **Checking:** structural assignability, inference, generic instantiation,
   conditional/mapped/template literal types, control-flow narrowing, overloads,
   recursion limits, and all language-version/strictness modes. Differential tests
   must compare diagnostic codes, categories, messages, ranges, and order.
5. **Emit:** JavaScript lowering across targets, declarations, maps and build info;
   compare emitted artifacts and runtime behavior, not just successful exit codes.
6. **Production:** project references/incremental builds, cancellation, watch and
   editor protocols, long-lived memory/resource tests, determinism under parallel
   execution, platform CI, representative external projects, and the full upstream
   conformance suite for the pinned release. No production-ready release until
   these required gates pass with an explicit coverage report.

Measure before introducing concurrency, custom allocators, caches, SIMD, or
specialization. Correctness is the default; optimizations that change upstream
behavior do not satisfy compatibility. Cache keys must include options and program
identity; memory must be reclaimable when a compilation or editor session ends.

Updating the latest-stable target is deliberate: inspect the release, update npm
and Rust metadata/documentation together, regenerate tables, run all differential
and regression checks, and explain new incompatibilities. A floating dependency
would make both correctness and performance claims irreproducible.
