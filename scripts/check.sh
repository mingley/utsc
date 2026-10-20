#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
npm run test:bench
npm run check:generated
npm test
