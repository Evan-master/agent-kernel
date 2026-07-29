#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="$("$ROOT_DIR/scripts/build-qemu-native-udp-driver-image.sh" "$@")"
ECHO_LOG="$(mktemp)"
"$ROOT_DIR/scripts/qemu-udp-echo.rb" >"$ECHO_LOG" 2>&1 &
ECHO_PID=$!

cleanup() {
  kill "$ECHO_PID" 2>/dev/null || true
  wait "$ECHO_PID" 2>/dev/null || true
  rm -f "$ECHO_LOG"
}
trap cleanup EXIT

for _ in {1..100}; do
  if grep -Fq "ready" "$ECHO_LOG"; then
    break
  fi
  if ! kill -0 "$ECHO_PID" 2>/dev/null; then
    cat "$ECHO_LOG" >&2
    exit 1
  fi
  sleep 0.01
done
if ! grep -Fq "ready" "$ECHO_LOG"; then
  printf 'UDP echo service did not become ready\n' >&2
  exit 1
fi

set +e
OUTPUT="$(qemu-system-x86_64 \
  -machine q35 \
  -smp 1 \
  -m 256M \
  -drive "format=raw,file=$IMAGE" \
  -device "intel-iommu,aw-bits=39,intremap=off,caching-mode=off" \
  -netdev "user,id=net0" \
  -device "virtio-net-pci-non-transitional,netdev=net0,bus=pcie.0,addr=0x5,mac=52:54:00:12:34:56,iommu_platform=on,vectors=3,csum=off,gso=off,guest_csum=off,guest_tso4=off,guest_tso6=off,guest_ecn=off,guest_ufo=off,guest_uso4=off,guest_uso6=off,mrg_rxbuf=on,ctrl_vq=off,mq=off,indirect_desc=off,event_idx=off,packed=off" \
  -serial stdio \
  -display none \
  -no-reboot \
  -device isa-debug-exit,iobase=0xf4,iosize=0x04 2>&1)"
STATUS=$?
set -e

printf '%s\n' "$OUTPUT"

if [[ "$STATUS" -ne 33 ]]; then
  printf 'qemu exited with unexpected status %s\n' "$STATUS" >&2
  exit "$STATUS"
fi

for expected in \
  "AGENT_KERNEL_QEMU_BOOT_OK" \
  "AGENT_KERNEL_ACPI_TOPOLOGY_OK" \
  "AGENT_KERNEL_NATIVE_UDP_DMAR_DISCOVERY_OK" \
  "AGENT_KERNEL_NATIVE_UDP_DRIVER_CAPABILITY_OK" \
  "AGENT_KERNEL_NATIVE_UDP_DRIVER_IMAGE_OK" \
  "AGENT_KERNEL_NATIVE_UDP_RING3_READY_OK" \
  "AGENT_KERNEL_NATIVE_UDP_NEIGHBOR_DRIVER_OK" \
  "AGENT_KERNEL_NATIVE_UDP_ARP_REPLY_OK" \
  "AGENT_KERNEL_NATIVE_UDP_EXCHANGE_DRIVER_OK" \
  "AGENT_KERNEL_NATIVE_UDP_ECHO_OK" \
  "AGENT_KERNEL_NATIVE_UDP_MSIX_OK" \
  "AGENT_KERNEL_NATIVE_UDP_CORE_EVIDENCE_OK" \
  "AGENT_KERNEL_NATIVE_UDP_ENDPOINT_RELEASED_OK" \
  "AGENT_KERNEL_NATIVE_UDP_VTD_TEARDOWN_OK" \
  "AGENT_KERNEL_NATIVE_UDP_PROOF_OK"; do
  if ! grep -Fq "$expected" <<<"$OUTPUT"; then
    printf 'missing QEMU native UDP Driver evidence: %s\n' "$expected" >&2
    exit 1
  fi
done

if grep -Eq "AGENT_KERNEL_.*_ERROR" <<<"$OUTPUT"; then
  printf 'QEMU native UDP Driver proof emitted an error marker\n' >&2
  exit 1
fi
