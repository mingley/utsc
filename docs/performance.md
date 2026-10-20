# Performance measurements

The goal is extremely high compiler throughput without sacrificing compatibility.
The current utsc implementation only supports scanner microbenchmarks. We also
have a correctness-gated harness for competitor compiler baselines; utsc cannot
participate yet. It has no measured advantage over a complete TypeScript compiler.

The performance acceptance target is to beat both current `tsc` and `ts-rust` on
fixed representative projects at matching diagnostics and emitted output, with
wall/CPU time, memory, warm/cold behavior, platform versions, and variability
reported. The current synthetic baseline does not meet that evidentiary bar.
See [compiler baselines](../benchmarks/compiler/README.md) for methodology and
explicit failure reporting.

## Reproduce

Run after `npm ci --ignore-scripts` with the pinned Rust toolchain:

```sh
cargo bench --locked --bench scanner
npm run bench:reference
```

Both benchmarks scan the same two source fixtures repeated 4,096 times. ASCII is
2,662,400 UTF-8 bytes; Unicode is 282,624 bytes. Each side runs in-process, warms
up, consumes token kind/end positions into a checksum, and reports the median
and range of seven samples lasting at least 500 ms each. The native benchmark
uses `black_box` to retain work. The reference warms up for at least 500 ms;
the native benchmark performs one complete warmup scan. No file loading, process
startup, JSON serialization, or compilation time is included in reported throughput.
Both checksums must match when comparing the same fixture.

The TypeScript reference also computes cooked token values and full flags that
utsc does not yet implement. This is a baseline, **not an equal-work speedup
comparison**. A compiler speed claim will require matched diagnostics and emitted
outputs on pinned real projects, plus wall time, CPU time, peak memory, multiple
runs, tool versions, and a description of the host and resource constraints.

## Initial native baseline

Measured on an Intel Xeon Platinum 8573C in the managed Linux x86_64
environment (5 visible CPUs), with Rust 1.99.1, thin LTO and one codegen unit. `cargo bench --locked --bench scanner`:

| Fixture | Median MiB/s | Range | Tokens | Checksum |
| --- | ---: | --- | ---: | ---: |
| ASCII | 96.4 | 95.8–97.1 | 958,465 | 1,275,967,000,577 |
| Unicode | 96.1 | 95.2–98.9 | 98,305 | 11,681,591,297 |

This shared-host observation is not a throughput guarantee. Do not extrapolate
scanner throughput to parsing or checking. Keep this baseline when measuring
changes; require all correctness checks before accepting an optimization.

The reference run used Node 24.19.0 and TypeScript 7.0.2 on the same host:

| Fixture | Reference median MiB/s | Range | Checksum |
| --- | ---: | --- | ---: |
| ASCII | 46.8 | 46.3–47.3 | 1,275,967,000,577 |
| Unicode | 44.2 | 43.4–44.3 | 11,681,591,297 |

Checksums matched. The unequal cooked-value/flag work described above prevents
using these numbers as an equal-work speedup claim.

## Compiler baseline captured October 19, 2026

Five samples, 256 generated modules plus an index (257 timed TypeScript files,
329,486 UTF-8 source bytes), identical strict options and shared standard
libraries, no incremental state, warm filesystem caches. Processes were run
sequentially with rotating order, after correctness checks; no concurrent local
validation was run during this captured measurement. The host is shared, so
ranges remain material.

| Compiler | Median check ms | Wall range ms | Median CPU ms | Largest check-process RSS MiB |
| --- | ---: | --- | ---: | ---: |
| tsc 7.0.2 | 214.4 | 204.7–409.2 | 871.9 | 97.3 |
| tsc-js 6.0.3 | 1326.3 | 1161.9–2095.9 | 2500.5 | 192.0 |
| ts-rust 0.7.0 | 64.0 | 58.9–422.3 | 184.4 | 68.1 |
| utsc | Unimplemented | — | — | — |

All three competitors passed exact diagnostics on the selected invalid probes,
including an error in the final module of the complete graph. The separate CLI
exit-status adapters (1 versus 2) are explicit. This small probe set does not
establish compiler conformance.

Only current tsc 7 passed byte-identical emit: the two other compilers had source
map differences, so their emit timings are withheld. The benchmark command
therefore exited **1**, retaining those failures. JS and declaration bytes
matched on this corpus. Map byte differences alone do not prove invalid maps.

The raw samples, distributions, hashes, versions, and failed emit gates are in
[the captured report](../benchmarks/compiler/results/2026-10-19-linux-x64.json).
This report was captured from the local working tree (`git_dirty: true`); its
harness hash, lockfile hashes, and input fingerprint identify the exact inputs.
These results neither rank utsc nor show that it beats either competitor.

Use `npm ci --prefix benchmarks/compiler --ignore-scripts` then
`npm run bench:compiler` to reproduce. See
[compiler harness documentation](../benchmarks/compiler/README.md) for the
methodology, gate failures, and measurement limits. A production performance
claim still requires representative pinned real projects and full parity.
