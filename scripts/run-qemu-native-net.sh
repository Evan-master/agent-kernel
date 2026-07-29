#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="$("$ROOT_DIR/scripts/build-qemu-native-net-image.sh" "$@")"

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
  "AGENT_KERNEL_NATIVE_NET_DMAR_DISCOVERY_OK" \
  "AGENT_KERNEL_NATIVE_NET_CAPABILITY_OK" \
  "AGENT_KERNEL_NATIVE_NET_DMA_DOMAIN_OK" \
  "AGENT_KERNEL_NATIVE_NET_MSIX_CONFIGURED_OK" \
  "AGENT_KERNEL_NATIVE_NET_TX_MSIX_DELIVERED_OK" \
  "AGENT_KERNEL_NATIVE_NET_ARP_REPLY_OK" \
  "AGENT_KERNEL_NATIVE_NET_ENDPOINT_RELEASED_OK" \
  "AGENT_KERNEL_NATIVE_NET_DMA_DENIAL_OK" \
  "AGENT_KERNEL_NATIVE_NET_PROOF_OK"; do
  if ! grep -Fq "$expected" <<<"$OUTPUT"; then
    printf 'missing QEMU native-network evidence: %s\n' "$expected" >&2
    exit 1
  fi
done

if grep -Eq "AGENT_KERNEL_.*_ERROR" <<<"$OUTPUT"; then
  printf 'QEMU native-network proof emitted an error marker\n' >&2
  exit 1
fi
