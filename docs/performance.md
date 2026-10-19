# Performance measurements

The goal is extremely high compiler throughput without sacrificing compatibility.
The current implementation only supports scanner microbenchmarks. It does not yet
have a measured advantage over a complete TypeScript compiler.

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
