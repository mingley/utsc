# utsc

A native Rust implementation of the TypeScript compiler, targeting the latest
stable release: **TypeScript 7.0.2**. The goal is full compatibility and high
performance with idiomatic Rust ownership and data structures.

**Status: compiler foundation, not a production-ready compiler.** This first
milestone implements a context-free scanner, source positions, a JSON CLI, and
an upstream differential test harness. There is no parser, type checker, module
resolver, or JavaScript emitter yet. No full-compiler speedup or conformance
claim is made.

## Develop

Install Rust using rustup; rust-toolchain.toml selects **1.99.1** with rustfmt and
Clippy. Node **22 or later** is needed only for development compatibility tests
and generation (CI uses Node 24). Cargo builds the native binary without Node.

```sh
npm ci --ignore-scripts
cargo build --locked
bash scripts/check.sh
```

In the managed cloud environment:

```sh
bash scripts/cloud-install.sh
source scripts/cloud-env.sh
bash scripts/check.sh
```

The bootstrap retains toolchains and caches outside the checkout. No services,
credentials, or application environment variables are required. Cloud tasks are
already isolated; use the existing checkout, without creating worktrees unless
explicitly requested.

## Run

```sh
cargo run --locked -- --version
cargo run --locked -- scan tests/fixtures/representative.ts
printf 'const answer = 42;\n' | cargo run --locked -- scan --stdin
printf '{"name":"example.ts","source":"const x = 1;"}\n' | cargo run --locked -- scan --jsonl
```

Each source produces one JSON record containing name, tokens, and lexical
diagnostics. Tokens preserve spelling and trivia, with UTF-8 byte spans and
UTF-16 code-unit spans. The scanner always advances on malformed input and emits
one EOF. Exit status is **0** for no lexical diagnostics, **1** for lexical
diagnostics, and **2** for an invocation, input, or protocol error. Success is
not a statement that the source is a valid TypeScript program.

The binary accepts UTF-8 files. Invalid UTF-8 and unpaired JSON surrogate escapes
are rejected rather than silently replaced. Diagnostic identifiers are native
names, not a claim of TSxxxx diagnostic parity.

## Correctness

```sh
cargo test --locked
npm test
npm run check:generated
# Add whole .ts/.tsx/.js files or directories to the raw-scanner corpus:
npm test -- --corpus path/to/project
```

The default suite compares **12,687 cases / 128,506 tokens** against the pinned
TypeScript scanner: hand-written edge cases, 4,000 deterministic generated inputs,
every generated Unicode range boundary, and complete upstream declaration/source
files. It compares kinds, spelling, UTF-16 spans, and untermination. Six upstream
scanner API divergences have explicit separate assertions; no cases are silently
normalized or accepted as expected failures. Rust tests include 10,000 deterministic
malformed Unicode inputs, source mappings, and CLI error handling.

This is lexical coverage, **not TypeScript compiler conformance**. See
[compatibility and limits](docs/compatibility.md), the
[architecture and milestone gates](docs/architecture.md), and
[benchmark methodology](docs/performance.md).

## Performance

```sh
cargo bench --locked --bench scanner
npm run bench:reference
```

These run scanners in-process over identical repeated source fixtures, with seven
samples and a median/range. They exclude file loading and JSON output. The
reference scanner also computes cooked token values that utsc does not yet
compute; the results cannot establish equal-work speedups or compiler performance.

## References and license

[Microsoft TypeScript](https://github.com/microsoft/TypeScript) is the behavior
reference. [pingdotgg/ts-rust](https://github.com/pingdotgg/ts-rust) was reviewed
for architecture and failure modes; its implementation and benchmark claims are
not treated as a correctness authority. No code from that repository is included.

Apache-2.0. See LICENSE and NOTICE for generated data provenance.
