#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TOOLCHAIN="nightly"
TARGET="x86_64-unknown-none"
PROFILE="debug"

if [[ "${1:-}" == "--release" ]]; then
  PROFILE="release"
  shift
fi
if [[ "$#" -ne 0 ]]; then
  printf 'usage: %s [--release]\n' "$0" >&2
  exit 2
fi

KERNEL="$ROOT_DIR/target/$TARGET/$PROFILE/agent-kernel-x86_64"
if [[ "$PROFILE" == "debug" ]]; then
  IMAGE="$ROOT_DIR/target/agent-kernel-x86_64-bios.img"
else
  IMAGE="$ROOT_DIR/target/agent-kernel-x86_64-release-bios.img"
fi

export PATH="$HOME/.cargo/bin:$PATH"
export RUSTC="$(rustup which rustc --toolchain "$TOOLCHAIN")"

cd "$ROOT_DIR"
if [[ "$PROFILE" == "release" ]]; then
  rustup run "$TOOLCHAIN" cargo build -p agent-kernel-x86_64 --features bare-metal --target "$TARGET" --release
else
  rustup run "$TOOLCHAIN" cargo build -p agent-kernel-x86_64 --features bare-metal --target "$TARGET"
fi
rustup run "$TOOLCHAIN" cargo run -p agent-kernel-image -- "$KERNEL" "$IMAGE" >/dev/null

printf '%s\n' "$IMAGE"
