# Compiler baselines

utsc currently cannot compile TypeScript. This harness measures competitors and
explicitly records utsc as `not_implemented`, without a timing or ranking. It
cannot demonstrate that utsc beats either competitor.

## Reproduce on Linux

Prerequisites: Node 22+, Python 3, the root npm dependencies. Install the optional
benchmark tools with their separate integrity-pinned lockfile:

```sh
npm ci --ignore-scripts
npm ci --prefix benchmarks/compiler --ignore-scripts
npm run bench:compiler
# Faster harness smoke run (not comparable to the default workload):
npm run bench:compiler -- --samples 3 --modules 64
```

No lifecycle scripts or network access are needed during a run. The native
compiler executables come from installed platform packages, not a postinstall
download. Defaults are five samples and 256 generated modules plus an index.
The configuration checks all three shared TypeScript 7.0.2 library files and
uses strict mode, `skipLibCheck: false`, no incremental state, and fresh compiler
processes. No `--fast` or semantic approximation mode is enabled.

Competitors are:

- `typescript` **7.0.2**, current native `tsc`, invoked directly.
- `typescript` **6.0.3**, the JavaScript compiler, invoked with Node.
- `tsc-rs` **0.7.0**, the Rust port, invoked directly. Its binary reports
  `Version 7.1.0-dev`; package version and reported version are recorded separately.

The generated project exercises imports, interfaces, structural assignments,
conditional/mapped/template literal types, inference, narrowing, and declarations.
It is synthetic and small; it is not a representative real-project suite.

## Gates and recorded differences

Each compiler must accept the valid project with no diagnostics and match the
reference diagnostics on a type-error probe, a syntax-error probe, and an error
in the last module of a duplicate full project graph. This last probe checks
that the complete generated graph is actually checked. Messages, codes, ranges,
and order must match; only CRLF/LF output transport differences are normalized.

The pinned `tsc` 7 invocation returns exit **1** on these diagnostics, whereas
`tsc` 6 and `ts-rust` return **2**. This is a declared per-compiler adapter, not an
unrecorded normalization; raw statuses/output are retained. A future utsc
candidate must match current `tsc`'s exit status. Success on invalid inputs,
missing/moved/changed diagnostics, a crash, or a timeout fail the check gate.

Emit is gated separately: require the exact file list and bytes of JavaScript,
declarations, and both source-map types. Artifacts are hashed after every timed
emit. A failed emit gate never blocks a separately eligible no-emit check
baseline, and never receives an emit timing. It remains a visible failure.

**Current observation:** JS/declaration bytes match on this corpus, but source
maps differ for both older `tsc` and `ts-rust` versus native `tsc` 7. JS `tsc`
has different mapping entries; `ts-rust` also omits the empty `sourceRoot` field.
These facts alone do not prove that either implementation's maps are invalid.
They fail this harness's deliberately strict byte-parity requirement.

The runner returns **1** when any requested gate fails, even if other workloads
complete. Do not call that an entirely passing comparison. JSON results and
compiler artifacts remain under `target/compiler-bench/run-*/` for inspection.
The harness tests run in normal CI without installing optional competitors:

```sh
npm run test:bench
```

Linux measurement tests verify exit-code preservation, separate output streams,
and killing the whole compiler process group on timeout. Other platforms run the
portable gate/statistics tests; resource measurements currently support Linux.

## Measurement limits

Timing includes process startup and uses warm filesystem caches. Each competitor
gets a check and emit warmup after reference acquisition; run order rotates each
sample. Cleanup, source generation, artifact hashing, and Python wrapper startup
are excluded from the reported timer. Raw samples, median/min/max wall and CPU
times, maximum resident set size, versions, host information, lockfile/harness
hashes, and input fingerprint are saved. The reference acquisition is extra
untimed work and is disclosed in the report.

Linux `RUSAGE_CHILDREN.ru_maxrss` measures the largest child process, not the sum
of simultaneous process-tree memory. Compiler subprocesses are timed independently.
Record CPU constraints and avoid concurrent local jobs when collecting a baseline.
A shared host can still be noisy; distributions must accompany medians.

A future compiler may be supplied with `--utsc-bin path`. A binary identifying
itself as the current scanner milestone is explicitly excluded. Other candidates
must pass the same gates; a scan speed cannot become a compile speed.
