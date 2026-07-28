#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="$("$ROOT_DIR/scripts/build-qemu-dma-iommu-image.sh" "$@")"

set +e
OUTPUT="$(qemu-system-x86_64 \
  -machine q35 \
  -smp 1 \
  -m 256M \
  -drive "format=raw,file=$IMAGE" \
  -device "intel-iommu,aw-bits=39,intremap=off,caching-mode=off" \
  -device "edu,addr=0x5" \
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
  "AGENT_KERNEL_DMAR_DISCOVERY_OK" \
  "AGENT_KERNEL_EDU_PCI_TARGET_OK" \
  "AGENT_KERNEL_DMA_BUS_MASTER_QUIESCED_OK" \
  "AGENT_KERNEL_DMA_CAPABILITY_OK" \
  "AGENT_KERNEL_VTD_TRANSLATION_OK" \
  "AGENT_KERNEL_DMA_ALLOWED_OK" \
  "AGENT_KERNEL_DMA_REVOKED_FAULT_OK" \
  "AGENT_KERNEL_DMA_IOMMU_PROOF_OK"; do
  if ! grep -Fq "$expected" <<<"$OUTPUT"; then
    printf 'missing QEMU DMA/IOMMU evidence: %s\n' "$expected" >&2
    exit 1
  fi
done

if grep -Eq "AGENT_KERNEL_.*_ERROR" <<<"$OUTPUT"; then
  printf 'QEMU DMA/IOMMU proof emitted an error marker\n' >&2
  exit 1
fi
