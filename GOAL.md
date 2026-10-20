# Completion goal

Build a native, idiomatic Rust TypeScript compiler that is fully compatible with
the latest stable upstream release and beats both current tsc and ts-rust on
matched, representative workloads. **This goal is not complete.** This document
records the goal and its gates; it is not a background-job scheduler.

## Required completion gates

- Track a pinned, reproducible latest-stable compatibility release. Update the
  pin deliberately when upstream advances; preserve versioned compatibility data.
- Implement complete syntax, parser recovery and source fidelity for JS/TS/JSX/TSX,
  including contextual scanning, cooked values, flags and exact diagnostics.
- Implement programs, libraries, tsconfig/options, binding, module resolution,
  declaration merging, type inference/assignability/narrowing, overloads, generic
  instantiation and all strictness/target modes without semantic approximations.
- Emit matching JavaScript, declarations, source maps and build information for
  supported targets. Validate runtime behavior, not only output hashes.
- Implement project references, incremental/watch behavior, cancellation and editor
  protocols with deterministic diagnostics and bounded long-lived resource usage.
- Pass the full upstream compiler conformance suite and pinned real projects;
  distinguish passes, failures, skips, unsupported cases and unrun tests. Scanner
  and expression-subset tests never count as full-compiler conformance.
- Beat both current tsc and ts-rust on a pinned real-project matrix at equal work
  and correctness, with multiple runs, distributions, CPU, memory, warm/cold state
  and platform/resource limits. Preserve known failures in benchmark reports.
- Verify packaging, platform CI and restoration/reproduction on fresh machines.

## Current evidence and continuation

The pushed foundation includes a differential-tested raw scanner and competitor
benchmarks. Parser-directed greater-than/template rescans and a typed expression
parser are now implemented; the expression parser is intentionally partial.
Whole source files, a type checker and emission remain unimplemented.

Continue through the gates in docs/architecture.md. Near-term work is complete
source-file grammar and type syntax, exact scanner/parser diagnostics, then
binding/checking/emit in differential-tested vertical slices. Keep rejecting
unsupported input explicitly. Record every completed milestone and its actual
validation; do not mark this goal complete because one milestone passes.
