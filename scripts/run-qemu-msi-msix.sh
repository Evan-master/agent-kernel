#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="$("$ROOT_DIR/scripts/build-qemu-msi-msix-image.sh" "$@")"

set +e
OUTPUT="$(qemu-system-x86_64 \
  -machine q35 \
  -smp 1 \
  -m 256M \
  -drive "format=raw,file=$IMAGE" \
  -device "intel-iommu,aw-bits=39,intremap=off,caching-mode=off" \
  -device "edu,bus=pcie.0,addr=0x5" \
  -object "rng-random,id=rng0,filename=/dev/urandom" \
  -device "virtio-rng-pci-non-transitional,rng=rng0,bus=pcie.0,addr=0x6,iommu_platform=on" \
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
  "AGENT_KERNEL_INTERRUPT_CAPABILITY_OK" \
  "AGENT_KERNEL_MULTI_DEVICE_DMA_DOMAIN_OK" \
  "AGENT_KERNEL_MSI_CONFIGURED_OK" \
  "AGENT_KERNEL_EDU_MSI_DELIVERED_OK" \
  "AGENT_KERNEL_MSIX_CONFIGURED_OK" \
  "AGENT_KERNEL_VIRTIO_RNG_MSIX_DELIVERED_OK" \
  "AGENT_KERNEL_DMA_REQUESTER_DETACHED_OK" \
  "AGENT_KERNEL_DMA_DETACH_FAULT_OK" \
  "AGENT_KERNEL_SHARED_DOMAIN_SURVIVOR_OK" \
  "AGENT_KERNEL_MSI_MSIX_PROOF_OK"; do
  if ! grep -Fq "$expected" <<<"$OUTPUT"; then
    printf 'missing QEMU MSI/MSI-X evidence: %s\n' "$expected" >&2
    exit 1
  fi
done

if grep -Eq "AGENT_KERNEL_.*_ERROR" <<<"$OUTPUT"; then
  printf 'QEMU MSI/MSI-X proof emitted an error marker\n' >&2
  exit 1
fi
