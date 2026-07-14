#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TOOLCHAIN="nightly"
TARGET="x86_64-unknown-none"
KERNEL="$ROOT_DIR/target/$TARGET/debug/agent-kernel-x86_64"
IMAGE="$ROOT_DIR/target/agent-kernel-x86_64-bios.img"

export PATH="$HOME/.cargo/bin:$PATH"
export RUSTC="$(rustup which rustc --toolchain "$TOOLCHAIN")"

cd "$ROOT_DIR"
rustup run "$TOOLCHAIN" cargo build -p agent-kernel-x86_64 --features bare-metal --target "$TARGET"
rustup run "$TOOLCHAIN" cargo run -p agent-kernel-image -- "$KERNEL" "$IMAGE" >/dev/null

printf '%s\n' "$IMAGE"
