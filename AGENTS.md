# Working on utsc

The goal is a fast native Rust implementation of the latest stable TypeScript
compiler. Read README.md and docs/compatibility.md for the actual implemented
scope. This repository currently contains a scanner, not a working compiler.

- Use this checkout. Cloud tasks are isolated; do not create Git worktrees unless
  the user explicitly requests one.
- Rust is pinned in rust-toolchain.toml. In the managed cloud environment, source
  scripts/cloud-env.sh; run bash scripts/cloud-install.sh if dependencies are absent.
- Run bash scripts/check.sh before committing. Run cargo bench --locked --bench
  scanner for scanner performance work. These checks do not prove compiler parity.
- Use the pinned official TypeScript compiler as the compatibility oracle.
  pingdotgg/ts-rust is a reference, not an oracle. Record imported data and licenses.
- Never hide mismatches by weakening checks, dropping tests, normalizing away
  differences, or promoting known failures to passing tests. Document and test
  deliberate API deviations separately.
- Preserve UTF-8 byte and UTF-16 code-unit positions as distinct quantities.
  Consume input on every error. Never silently replace invalid UTF-8.
- Prefer borrowed text, typed enums/indices, immutable source, and scoped ownership.
  Avoid global mutable state, stringly typed compiler data, and unnecessary clones.
  Unsafe Rust is forbidden. Keep generated tables reproducible via npm run generate.
- No semantic approximation or compatibility-changing optimization may be enabled
  as the default. Performance claims need a fixed corpus, matched work, warmups,
  repeated samples, tool versions, and an explanation of measurement limits.
- Add parser/checker/emitter capabilities in tested vertical slices. Do not report
  an unsupported feature as valid or a lexical check as a compilation test.
