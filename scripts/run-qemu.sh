#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="$("$ROOT_DIR/scripts/build-qemu-image.sh")"

set +e
OUTPUT="$(qemu-system-x86_64 \
  -drive "format=raw,file=$IMAGE" \
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
  "AGENT_KERNEL_GDT_TSS_OK" \
  "AGENT_KERNEL_EXCEPTION_BASELINE_OK" \
  "AGENT_KERNEL_AGENT_USER_MEMORY_OK" \
  "AGENT_KERNEL_AGENT_ADDRESS_SPACE_OK" \
  "AGENT_KERNEL_MULTI_AGENT_MEMORY_OK" \
  "AGENT_KERNEL_PIT_IRQ_OK" \
  "AGENT_KERNEL_AGENT_CPU_PREEMPTION_OK" \
  "AGENT_KERNEL_AGENT_RING3_PREEMPTION_OK" \
  "AGENT_KERNEL_AGENT_B_PREEMPTION_OK" \
  "AGENT_KERNEL_TIMER_PREEMPTION_OK" \
  "AGENT_KERNEL_AGENT_CPU_RESUME_OK" \
  "AGENT_KERNEL_AGENT_CALL_YIELD_OK" \
  "AGENT_KERNEL_AGENT_CR3_SWITCH_OK" \
  "AGENT_KERNEL_MULTI_AGENT_CONTEXT_SWITCH_OK" \
  "AGENT_KERNEL_MULTI_AGENT_ISOLATION_OK" \
  "event[1] agent_registered" \
  "event[2] capability_granted" \
  "event[3] agent_image_registered" \
  "event[4] agent_image_verified" \
  "event[5] agent_launched" \
  "event[6] observation" \
  "event[7] action" \
  "event[8] verification" \
  "AGENT_KERNEL_PORT_IO_BACKEND_OK" \
  "AGENT_KERNEL_PORT_COMMAND_FLOW_OK" \
  "AGENT_KERNEL_DRIVER_INVOCATION_FLOW_OK" \
  "AGENT_KERNEL_UART_IRQ_OK" \
  "event[9] driver_endpoint_registered" \
  "event[10] agent_registered" \
  "event[11] capability_derived" \
  "event[12] agent_image_registered" \
  "event[13] agent_image_verified" \
  "event[14] agent_launched" \
  "event[15] driver_bound" \
  "event[16] agent_registered" \
  "event[17] intent_declared" \
  "event[18] task_created" \
  "event[19] intent_bound" \
  "event[20] capability_derived" \
  "event[21] delegation" \
  "event[22] agent_registered" \
  "event[23] intent_declared" \
  "event[24] task_created" \
  "event[25] intent_bound" \
  "event[26] capability_derived" \
  "event[27] delegation" \
  "event[28] agent_image_registered" \
  "event[29] agent_image_verified" \
  "event[30] agent_launched" \
  "event[31] task_accepted" \
  "event[32] task_queued" \
  "event[33] agent_launched" \
  "event[34] task_accepted" \
  "event[35] task_queued" \
  "event[36] task_dispatched" \
  "event[37] task_quantum_expired" \
  "event[38] task_dispatched" \
  "event[39] task_quantum_expired" \
  "event[40] task_dispatched" \
  "event[41] task_yielded" \
  "event[42] task_dispatched" \
  "event[43] task_yielded" \
  "event[44] device_event_raised" \
  "event[45] device_event_delivered" \
  "event[46] driver_invocation_queued" \
  "event[47] driver_invocation_dispatched" \
  "event[48] driver_invocation_ticked" \
  "event[49] device_event_acknowledged" \
  "event[50] driver_command_submitted" \
  "event[51] driver_command_dispatched" \
  "event[52] driver_command_completed" \
  "event[53] driver_invocation_completed" \
  "SUPERVISOR_HANDOFF_READY"
do
  if ! grep -Fq "$expected" <<<"$OUTPUT"; then
    printf 'missing expected QEMU output: %s\n' "$expected" >&2
    exit 1
  fi
done

EVENT_COUNT="$(grep -Fc 'event[' <<<"$OUTPUT")"
if [[ "$EVENT_COUNT" -ne 53 ]]; then
  printf 'expected exactly 53 kernel events, observed %s\n' "$EVENT_COUNT" >&2
  exit 1
fi
