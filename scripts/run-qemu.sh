#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="$("$ROOT_DIR/scripts/build-qemu-image.sh" "$@")"

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
  "AGENT_KERNEL_AGENT_IMAGE_FORMAT_OK" \
  "AGENT_KERNEL_AGENT_IMAGE_DIGEST_OK" \
  "AGENT_KERNEL_AGENT_IMAGE_LOAD_OK" \
  "AGENT_KERNEL_VERIFIER_IMAGE_OK" \
  "AGENT_KERNEL_VERIFIER_MEMORY_OK" \
  "AGENT_KERNEL_FAULT_HANDLER_MEMORY_OK" \
  "AGENT_KERNEL_MULTI_AGENT_MEMORY_OK" \
  "AGENT_KERNEL_PIT_IRQ_OK" \
  "AGENT_KERNEL_AGENT_CPU_PREEMPTION_OK" \
  "AGENT_KERNEL_AGENT_RING3_PREEMPTION_OK" \
  "AGENT_KERNEL_AGENT_A_PREEMPTION_OK" \
  "AGENT_KERNEL_TIMER_PREEMPTION_OK" \
  "AGENT_KERNEL_KERNEL_SELECTED_DISPATCH_OK" \
  "AGENT_KERNEL_AGENT_CALL_RECEIVE_WAIT_OK" \
  "AGENT_KERNEL_NATIVE_BLOCKING_MAILBOX_WAIT_OK" \
  "AGENT_KERNEL_NATIVE_BLOCKING_MAILBOX_WAKE_OK" \
  "AGENT_KERNEL_VERIFIER_PREEMPTION_OK" \
  "AGENT_KERNEL_AGENT_CPU_RESUME_OK" \
  "AGENT_KERNEL_AGENT_CALL_ABI_OK" \
  "AGENT_KERNEL_AGENT_CALL_RETURN_OK" \
  "AGENT_KERNEL_AGENT_CALL_RESULT_OK" \
  "AGENT_KERNEL_AGENT_CALL_RETURNING_MUTATION_OK" \
  "AGENT_KERNEL_AGENT_CALL_SEND_MESSAGE_OK" \
  "AGENT_KERNEL_AGENT_CALL_RECEIVE_MESSAGE_OK" \
  "AGENT_KERNEL_AGENT_CALL_ACKNOWLEDGE_MESSAGE_OK" \
  "AGENT_KERNEL_NATIVE_MAILBOX_IPC_OK" \
  "AGENT_KERNEL_NATIVE_AGENT_YIELD_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_LOOP_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_QUANTUM_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_STORE_OK" \
  "AGENT_KERNEL_AGENT_CALL_INSPECT_RESULT_OK" \
  "AGENT_KERNEL_AGENT_CALL_VERIFY_OK" \
  "AGENT_KERNEL_RESUMABLE_RUNTIME_REGISTRY_OK" \
  "AGENT_KERNEL_DISPATCH_READINESS_HANDOFF_OK" \
  "AGENT_KERNEL_NATIVE_AGENT_FAULT_CONTAINMENT_OK" \
  "AGENT_KERNEL_NATIVE_AGENT_FAULT_RESTART_OK" \
  "AGENT_KERNEL_NATIVE_AGENT_GENERAL_PROTECTION_OK" \
  "AGENT_KERNEL_NATIVE_AGENT_PAGE_FAULT_OK" \
  "AGENT_KERNEL_NATIVE_AGENT_DEMAND_PAGE_OK" \
  "AGENT_KERNEL_NATIVE_FAULT_HANDLER_WAIT_OK" \
  "AGENT_KERNEL_NATIVE_FAULT_POLICY_ROUTE_OK" \
  "AGENT_KERNEL_NATIVE_FAULT_HANDLER_DECISION_OK" \
  "AGENT_KERNEL_NATIVE_FAULT_HANDLER_AGENT_OK" \
  "AGENT_KERNEL_AGENT_CALL_AUTHORITY_OK" \
  "AGENT_KERNEL_AGENT_CALL_COMPLETE_OK" \
  "AGENT_KERNEL_AGENT_CR3_SWITCH_OK" \
  "AGENT_KERNEL_MULTI_AGENT_CONTEXT_SWITCH_OK" \
  "AGENT_KERNEL_MULTI_AGENT_ISOLATION_OK" \
  "AGENT_KERNEL_HETEROGENEOUS_AGENT_EXECUTION_OK" \
  "AGENT_KERNEL_NATIVE_VERIFIER_OK" \
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
  "event[30] agent_image_registered" \
  "event[31] agent_image_verified" \
  "event[32] agent_launched" \
  "event[33] task_accepted" \
  "event[34] task_queued" \
  "event[35] agent_launched" \
  "event[36] task_accepted" \
  "event[37] task_queued" \
  "event[38] agent_registered" \
  "event[39] intent_declared" \
  "event[40] task_created" \
  "event[41] intent_bound" \
  "event[42] capability_derived" \
  "event[43] delegation" \
  "event[44] capability_derived" \
  "event[45] agent_image_registered" \
  "event[46] agent_image_verified" \
  "event[47] agent_launched" \
  "event[48] task_accepted" \
  "event[49] agent_registered" \
  "event[50] intent_declared" \
  "event[51] task_created" \
  "event[52] intent_bound" \
  "event[53] capability_derived" \
  "event[54] delegation" \
  "event[55] agent_image_registered" \
  "event[56] agent_image_verified" \
  "event[57] agent_launched" \
  "event[58] task_accepted" \
  "event[59] agent_registered" \
  "event[60] intent_declared" \
  "event[61] task_created" \
  "event[62] intent_bound" \
  "event[63] capability_derived" \
  "event[64] delegation" \
  "event[65] agent_image_registered" \
  "event[66] agent_image_verified" \
  "event[67] agent_launched" \
  "event[68] task_accepted" \
  "event[69] fault_handler_installed" \
  "event[70] fault_policy_installed" \
  "event[71] task_dispatched" \
  "event[72] task_quantum_expired" \
  "event[73] task_dispatched" \
  "event[74] task_quantum_expired" \
  "event[75] task_dispatched" \
  "event[76] message_wait_started" \
  "event[77] task_dispatched" \
  "event[78] task_result_submitted" \
  "event[79] message_sent" \
  "event[80] message_wait_woken" \
  "event[81] task_quantum_expired" \
  "event[82] task_dispatched" \
  "event[83] message_received" \
  "event[84] message_acknowledged" \
  "event[85] task_result_submitted" \
  "event[86] task_completed" \
  "event[87] task_dispatched" \
  "event[88] task_yielded" \
  "event[89] task_dispatched" \
  "event[90] task_completed" \
  "event[91] task_queued" \
  "event[92] task_dispatched" \
  "event[93] task_quantum_expired" \
  "event[94] task_dispatched" \
  "event[95] message_wait_started" \
  "event[96] task_queued" \
  "event[97] task_queued" \
  "event[98] task_dispatched" \
  "event[99] task_quantum_expired" \
  "event[100] task_dispatched" \
  "event[101] task_quantum_expired" \
  "event[102] task_dispatched" \
  "event[103] task_faulted" \
  "event[104] task_dispatched" \
  "event[105] task_result_inspected" \
  "event[106] task_verified" \
  "event[107] intent_fulfilled" \
  "event[108] task_completed" \
  "event[109] task_fault_recovered" \
  "event[110] task_queued" \
  "event[111] task_dispatched" \
  "event[112] task_quantum_expired" \
  "event[113] task_dispatched" \
  "event[114] task_faulted" \
  "event[115] task_fault_recovered" \
  "event[116] task_queued" \
  "event[117] task_dispatched" \
  "event[118] task_quantum_expired" \
  "event[119] task_dispatched" \
  "event[120] task_faulted" \
  "event[121] task_fault_recovered" \
  "event[122] task_queued" \
  "event[123] task_dispatched" \
  "event[124] task_quantum_expired" \
  "event[125] task_dispatched" \
  "event[126] task_faulted" \
  "event[127] message_sent" \
  "event[128] message_wait_woken" \
  "event[129] fault_routed" \
  "event[130] fault_policy_applied" \
  "event[131] task_dispatched" \
  "event[132] message_received" \
  "event[133] message_acknowledged" \
  "event[134] task_result_submitted" \
  "event[135] task_completed" \
  "event[136] task_fault_recovered" \
  "event[137] task_queued" \
  "event[138] task_dispatched" \
  "event[139] task_completed" \
  "event[140] device_event_raised" \
  "event[141] device_event_delivered" \
  "event[142] driver_invocation_queued" \
  "event[143] driver_invocation_dispatched" \
  "event[144] driver_invocation_ticked" \
  "event[145] device_event_acknowledged" \
  "event[146] driver_command_submitted" \
  "event[147] driver_command_dispatched" \
  "event[148] driver_command_completed" \
  "event[149] driver_invocation_completed" \
  "SUPERVISOR_HANDOFF_READY"
do
  if ! grep -Fq "$expected" <<<"$OUTPUT"; then
    printf 'missing expected QEMU output: %s\n' "$expected" >&2
    exit 1
  fi
done

EVENT_COUNT="$(grep -Fc 'event[' <<<"$OUTPUT")"
if [[ "$EVENT_COUNT" -ne 149 ]]; then
  printf 'expected exactly 149 kernel events, observed %s\n' "$EVENT_COUNT" >&2
  exit 1
fi
