# Source this file from a shell in the managed cloud environment.
# Keep caches and downloaded toolchains outside the project checkout.
export UTSC_TOOLCHAIN_ROOT="${UTSC_TOOLCHAIN_ROOT:-/workspace/.cache/utsc}"
export RUSTUP_HOME="$UTSC_TOOLCHAIN_ROOT/rustup"
export CARGO_HOME="$UTSC_TOOLCHAIN_ROOT/cargo"
export PATH="$CARGO_HOME/bin:$PATH"
