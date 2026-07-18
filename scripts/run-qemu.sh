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
  "AGENT_KERNEL_RESOURCE_MANAGER_MEMORY_OK" \
  "AGENT_KERNEL_MULTI_AGENT_MEMORY_OK" \
  "AGENT_KERNEL_RUNTIME_FRAME_POOL_OK" \
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
  "AGENT_KERNEL_NATIVE_FAULT_REPAIR_ADMITTED_OK" \
  "AGENT_KERNEL_NATIVE_FAULT_RECOVERY_COUNTERS_OK" \
  "AGENT_KERNEL_NATIVE_FAULT_HANDLER_AGENT_OK" \
  "AGENT_KERNEL_NATIVE_RESOURCE_MANAGER_READY_OK" \
  "AGENT_KERNEL_NATIVE_RESOURCE_MANAGER_QUEUED_OK" \
  "AGENT_KERNEL_NATIVE_RESOURCE_MANAGER_EXECUTION_OK" \
  "AGENT_KERNEL_NATIVE_RESOURCE_MANAGER_COUNTERS_OK" \
  "AGENT_KERNEL_NATIVE_RESOURCE_MANAGER_AGENT_OK" \
  "AGENT_KERNEL_NATIVE_CAPABILITY_MANAGER_OK" \
  "AGENT_KERNEL_NATIVE_TASK_MANAGER_OK" \
  "AGENT_KERNEL_NATIVE_AGENT_MANAGER_OK" \
  "AGENT_KERNEL_AGENT_CALL_ALLOCATE_MEMORY_PAGE_OK" \
  "AGENT_KERNEL_AGENT_CALL_INSPECT_MEMORY_PAGE_OK" \
  "AGENT_KERNEL_AGENT_CALL_RELEASE_MEMORY_PAGE_OK" \
  "AGENT_KERNEL_NATIVE_MEMORY_PAGE_MANAGER_OK" \
  "AGENT_KERNEL_AGENT_CALL_ALLOCATE_MEMORY_REGION_OK" \
  "AGENT_KERNEL_AGENT_CALL_INSPECT_MEMORY_REGION_OK" \
  "AGENT_KERNEL_AGENT_CALL_RELEASE_MEMORY_REGION_OK" \
  "AGENT_KERNEL_RUNTIME_FRAME_POOL_RELEASED_OK" \
  "AGENT_KERNEL_NATIVE_MEMORY_REGION_MANAGER_OK" \
  "AGENT_KERNEL_NATIVE_MEMORY_CONCURRENCY_OK" \
  "AGENT_KERNEL_AGENT_CALL_DECLARE_INTENT_OK" \
  "AGENT_KERNEL_AGENT_CALL_CREATE_TASK_OK" \
  "AGENT_KERNEL_AGENT_CALL_DELEGATE_TASK_OK" \
  "AGENT_KERNEL_AGENT_CALL_REGISTER_MANAGED_AGENT_OK" \
  "AGENT_KERNEL_AGENT_CALL_SUSPEND_MANAGED_AGENT_OK" \
  "AGENT_KERNEL_AGENT_CALL_RESUME_MANAGED_AGENT_OK" \
  "AGENT_KERNEL_AGENT_CALL_RETIRE_MANAGED_AGENT_OK" \
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
  "event[71] agent_registered" \
  "event[72] intent_declared" \
  "event[73] task_created" \
  "event[74] intent_bound" \
  "event[75] capability_derived" \
  "event[76] delegation" \
  "event[77] capability_derived" \
  "event[78] agent_image_registered" \
  "event[79] agent_image_verified" \
  "event[80] agent_launched" \
  "event[81] task_accepted" \
  "event[82] task_dispatched" \
  "event[83] task_quantum_expired" \
  "event[84] task_dispatched" \
  "event[85] task_quantum_expired" \
  "event[86] task_dispatched" \
  "event[87] message_wait_started" \
  "event[88] task_dispatched" \
  "event[89] task_result_submitted" \
  "event[90] message_sent" \
  "event[91] message_wait_woken" \
  "event[92] task_quantum_expired" \
  "event[93] task_dispatched" \
  "event[94] message_received" \
  "event[95] message_acknowledged" \
  "event[96] task_result_submitted" \
  "event[97] task_completed" \
  "event[98] task_dispatched" \
  "event[99] task_yielded" \
  "event[100] task_dispatched" \
  "event[101] task_completed" \
  "event[102] task_queued" \
  "event[103] task_dispatched" \
  "event[104] task_quantum_expired" \
  "event[105] task_dispatched" \
  "event[106] message_wait_started" \
  "event[107] task_queued" \
  "event[108] task_queued" \
  "event[109] task_dispatched" \
  "event[110] task_quantum_expired" \
  "event[111] task_dispatched" \
  "event[112] task_quantum_expired" \
  "event[113] task_dispatched" \
  "event[114] task_faulted" \
  "event[115] task_dispatched" \
  "event[116] task_result_inspected" \
  "event[117] task_verified" \
  "event[118] intent_fulfilled" \
  "event[119] task_completed" \
  "event[120] task_fault_recovered" \
  "event[121] task_queued" \
  "event[122] task_dispatched" \
  "event[123] task_quantum_expired" \
  "event[124] task_dispatched" \
  "event[125] task_faulted" \
  "event[126] task_fault_recovered" \
  "event[127] task_queued" \
  "event[128] task_dispatched" \
  "event[129] task_quantum_expired" \
  "event[130] task_dispatched" \
  "event[131] task_faulted" \
  "event[132] task_fault_recovered" \
  "event[133] task_queued" \
  "event[134] task_dispatched" \
  "event[135] task_quantum_expired" \
  "event[136] task_dispatched" \
  "event[137] task_faulted" \
  "event[138] message_sent" \
  "event[139] message_wait_woken" \
  "event[140] fault_routed" \
  "event[141] fault_policy_applied" \
  "event[142] task_dispatched" \
  "event[143] message_received" \
  "event[144] message_acknowledged" \
  "event[145] task_result_submitted" \
  "event[146] task_completed" \
  "event[147] task_fault_recovered" \
  "event[148] task_queued" \
  "event[149] task_dispatched" \
  "event[150] task_completed" \
  "event[151] task_queued" \
  "event[152] task_dispatched" \
  "event[153] task_quantum_expired" \
  "event[154] task_dispatched" \
  "event[155] resource_created" \
  "event[156] capability_granted" \
  "event[157] capability_derived" \
  "event[158] capability_revoked" \
  "event[159] resource_retired" \
  "event[160] intent_declared" \
  "event[161] task_created" \
  "event[162] intent_bound" \
  "event[163] capability_derived" \
  "event[164] delegation" \
  "event[165] agent_registered" \
  "event[166] agent_suspended" \
  "event[167] agent_resumed" \
  "event[168] agent_retired" \
  "event[169] resource_created" \
  "event[170] capability_granted" \
  "event[171] memory_cell_created" \
  "event[172] memory_cell_recalled" \
  "event[173] resource_retired" \
  "event[174] resource_created" \
  "event[175] capability_granted" \
  "event[176] memory_cell_created" \
  "event[177] memory_cell_recalled" \
  "event[178] resource_created" \
  "event[179] capability_granted" \
  "event[180] memory_cell_created" \
  "event[181] resource_retired" \
  "event[182] memory_cell_recalled" \
  "event[183] resource_created" \
  "event[184] capability_granted" \
  "event[185] memory_cell_created" \
  "event[186] memory_cell_recalled" \
  "event[187] resource_retired" \
  "event[188] resource_retired" \
  "event[189] task_result_submitted" \
  "event[190] task_completed" \
  "event[191] device_event_raised" \
  "event[192] device_event_delivered" \
  "event[193] driver_invocation_queued" \
  "event[194] driver_invocation_dispatched" \
  "event[195] driver_invocation_ticked" \
  "event[196] device_event_acknowledged" \
  "event[197] driver_command_submitted" \
  "event[198] driver_command_dispatched" \
  "event[199] driver_command_completed" \
  "event[200] driver_invocation_completed" \
  "SUPERVISOR_HANDOFF_READY"
do
  if ! grep -Fq "$expected" <<<"$OUTPUT"; then
    printf 'missing expected QEMU output: %s\n' "$expected" >&2
    exit 1
  fi
done

EVENT_COUNT="$(grep -Fc 'event[' <<<"$OUTPUT")"
if [[ "$EVENT_COUNT" -ne 200 ]]; then
  printf 'expected exactly 200 kernel events, observed %s\n' "$EVENT_COUNT" >&2
  exit 1
fi

for marker in \
  "AGENT_KERNEL_AGENT_CALL_ALLOCATE_MEMORY_REGION_OK" \
  "AGENT_KERNEL_AGENT_CALL_INSPECT_MEMORY_REGION_OK" \
  "AGENT_KERNEL_AGENT_CALL_RELEASE_MEMORY_REGION_OK"
do
  MARKER_COUNT="$(grep -Fxc "$marker" <<<"$OUTPUT")"
  if [[ "$MARKER_COUNT" -ne 3 ]]; then
    printf 'expected exactly 3 occurrences of %s, observed %s\n' "$marker" "$MARKER_COUNT" >&2
    exit 1
  fi
done
