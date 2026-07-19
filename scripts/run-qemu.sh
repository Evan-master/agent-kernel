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
  "AGENT_KERNEL_NATIVE_FAULT_MEMORY_RECLAIMED_OK" \
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
  "AGENT_KERNEL_NATIVE_COMPLETION_MEMORY_RECLAIMED_OK" \
  "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RECLAIMED_OK" \
  "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_FRAME_POOL_OK" \
  "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_ALLOCATED_OK" \
  "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REBUILT_OK" \
  "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_CANCEL_OK" \
  "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_BATCH_OK" \
  "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_CONCURRENCY_OK" \
  "AGENT_KERNEL_AGENT_CALL_RUNTIME_ADMISSION_REQUEST_OK" \
  "AGENT_KERNEL_AGENT_CALL_RUNTIME_ADMISSION_DISCOVERY_OK" \
  "AGENT_KERNEL_AGENT_CALL_RUNTIME_ADMISSION_COMPACTION_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_REQUEST_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_RESIDENT_WAIT_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_NOTIFICATION_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_SUPERVISOR_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_COMMIT_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_RELEASE_OK" \
  "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_PARTIAL_RECLAIM_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_REPEAT_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_COMPACTION_OK" \
  "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSE_EXECUTION_OK" \
  "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSED_RECLAIMED_OK" \
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
  "AGENT_KERNEL_PORT_IO_BACKEND_OK" \
  "AGENT_KERNEL_PORT_COMMAND_FLOW_OK" \
  "AGENT_KERNEL_DRIVER_INVOCATION_FLOW_OK" \
  "AGENT_KERNEL_UART_IRQ_OK" \
  "event[1] agent_registered" \
  "event[2] capability_granted" \
  "event[3] agent_image_registered" \
  "event[4] agent_image_verified" \
  "event[5] agent_launched" \
  "event[6] observation" \
  "event[7] action" \
  "event[8] verification" \
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
  "event[50] resource_created" \
  "event[51] capability_granted" \
  "event[52] capability_derived" \
  "event[53] intent_declared" \
  "event[54] task_created" \
  "event[55] intent_bound" \
  "event[56] capability_derived" \
  "event[57] delegation" \
  "event[58] agent_image_registered" \
  "event[59] agent_image_verified" \
  "event[60] agent_launched" \
  "event[61] task_accepted" \
  "event[62] agent_registered" \
  "event[63] intent_declared" \
  "event[64] task_created" \
  "event[65] intent_bound" \
  "event[66] capability_derived" \
  "event[67] delegation" \
  "event[68] agent_image_registered" \
  "event[69] agent_image_verified" \
  "event[70] agent_launched" \
  "event[71] task_accepted" \
  "event[72] fault_handler_installed" \
  "event[73] fault_policy_installed" \
  "event[74] agent_registered" \
  "event[75] intent_declared" \
  "event[76] task_created" \
  "event[77] intent_bound" \
  "event[78] capability_derived" \
  "event[79] delegation" \
  "event[80] capability_derived" \
  "event[81] agent_image_registered" \
  "event[82] agent_image_verified" \
  "event[83] agent_launched" \
  "event[84] task_accepted" \
  "event[85] task_dispatched" \
  "event[86] task_quantum_expired" \
  "event[87] task_dispatched" \
  "event[88] task_quantum_expired" \
  "event[89] task_dispatched" \
  "event[90] message_wait_started" \
  "event[91] task_dispatched" \
  "event[92] task_result_submitted" \
  "event[93] message_sent" \
  "event[94] message_wait_woken" \
  "event[95] task_quantum_expired" \
  "event[96] task_dispatched" \
  "event[97] message_received" \
  "event[98] message_acknowledged" \
  "event[99] task_result_submitted" \
  "event[100] task_completed" \
  "event[101] task_dispatched" \
  "event[102] task_yielded" \
  "event[103] task_dispatched" \
  "event[104] task_completed" \
  "event[105] task_queued" \
  "event[106] task_dispatched" \
  "event[107] task_quantum_expired" \
  "event[108] task_dispatched" \
  "event[109] message_wait_started" \
  "event[110] task_queued" \
  "event[111] task_queued" \
  "event[112] task_dispatched" \
  "event[113] task_quantum_expired" \
  "event[114] task_dispatched" \
  "event[115] task_quantum_expired" \
  "event[116] task_dispatched" \
  "event[117] memory_cell_created" \
  "event[118] resource_retired" \
  "event[119] task_faulted" \
  "event[120] task_dispatched" \
  "event[121] task_result_inspected" \
  "event[122] task_verified" \
  "event[123] intent_fulfilled" \
  "event[124] task_completed" \
  "event[125] task_fault_recovered" \
  "event[126] task_queued" \
  "event[127] task_dispatched" \
  "event[128] task_quantum_expired" \
  "event[129] task_dispatched" \
  "event[130] task_faulted" \
  "event[131] task_fault_recovered" \
  "event[132] task_queued" \
  "event[133] task_dispatched" \
  "event[134] task_quantum_expired" \
  "event[135] task_dispatched" \
  "event[136] task_faulted" \
  "event[137] task_fault_recovered" \
  "event[138] task_queued" \
  "event[139] task_dispatched" \
  "event[140] task_quantum_expired" \
  "event[141] task_dispatched" \
  "event[142] task_faulted" \
  "event[143] message_sent" \
  "event[144] message_wait_woken" \
  "event[145] fault_routed" \
  "event[146] fault_policy_applied" \
  "event[147] task_dispatched" \
  "event[148] message_received" \
  "event[149] message_acknowledged" \
  "event[150] task_result_submitted" \
  "event[151] task_completed" \
  "event[152] task_fault_recovered" \
  "event[153] task_queued" \
  "event[154] task_dispatched" \
  "event[155] task_completed" \
  "event[156] task_queued" \
  "event[157] task_dispatched" \
  "event[158] task_quantum_expired" \
  "event[159] task_dispatched" \
  "event[160] resource_created" \
  "event[161] capability_granted" \
  "event[162] capability_derived" \
  "event[163] capability_revoked" \
  "event[164] resource_retired" \
  "event[165] intent_declared" \
  "event[166] task_created" \
  "event[167] intent_bound" \
  "event[168] capability_derived" \
  "event[169] delegation" \
  "event[170] agent_registered" \
  "event[171] agent_suspended" \
  "event[172] agent_resumed" \
  "event[173] agent_retired" \
  "event[174] resource_created" \
  "event[175] capability_granted" \
  "event[176] memory_cell_created" \
  "event[177] memory_cell_recalled" \
  "event[178] resource_retired" \
  "event[179] resource_created" \
  "event[180] capability_granted" \
  "event[181] memory_cell_created" \
  "event[182] memory_cell_recalled" \
  "event[183] resource_created" \
  "event[184] capability_granted" \
  "event[185] memory_cell_created" \
  "event[186] resource_retired" \
  "event[187] memory_cell_recalled" \
  "event[188] resource_created" \
  "event[189] capability_granted" \
  "event[190] memory_cell_created" \
  "event[191] memory_cell_recalled" \
  "event[192] resource_retired" \
  "event[193] task_result_submitted" \
  "event[194] resource_retired" \
  "event[195] task_completed" \
  "event[196] agent_registered" \
  "event[197] intent_declared" \
  "event[198] task_created" \
  "event[199] intent_bound" \
  "event[200] capability_derived" \
  "event[201] delegation" \
  "event[202] agent_image_registered" \
  "event[203] agent_image_verified" \
  "event[204] agent_launched" \
  "event[205] task_accepted" \
  "event[206] agent_registered" \
  "event[207] intent_declared" \
  "event[208] task_created" \
  "event[209] intent_bound" \
  "event[210] capability_derived" \
  "event[211] delegation" \
  "event[212] agent_image_registered" \
  "event[213] agent_image_verified" \
  "event[214] agent_launched" \
  "event[215] task_accepted" \
  "event[216] agent_registered" \
  "event[217] intent_declared" \
  "event[218] task_created" \
  "event[219] intent_bound" \
  "event[220] capability_derived" \
  "event[221] delegation" \
  "event[222] capability_derived" \
  "event[223] agent_image_registered" \
  "event[224] agent_image_verified" \
  "event[225] agent_launched" \
  "event[226] task_accepted" \
  "event[227] task_queued" \
  "event[228] agent_registered" \
  "event[229] intent_declared" \
  "event[230] task_created" \
  "event[231] intent_bound" \
  "event[232] capability_derived" \
  "event[233] delegation" \
  "event[234] agent_image_registered" \
  "event[235] agent_image_verified" \
  "event[236] agent_launched" \
  "event[237] task_accepted" \
  "event[238] agent_registered" \
  "event[239] intent_declared" \
  "event[240] task_created" \
  "event[241] intent_bound" \
  "event[242] capability_derived" \
  "event[243] delegation" \
  "event[244] agent_image_registered" \
  "event[245] agent_image_verified" \
  "event[246] agent_launched" \
  "event[247] task_accepted" \
  "event[248] task_dispatched" \
  "event[249] task_quantum_expired" \
  "event[250] task_dispatched" \
  "event[251] runtime_admission_requested" \
  "event[252] runtime_admission_requested" \
  "event[253] message_wait_started" \
  "event[254] runtime_admission_admitted" \
  "event[255] task_queued" \
  "event[256] runtime_admission_admitted" \
  "event[257] task_queued" \
  "event[258] task_dispatched" \
  "event[259] task_quantum_expired" \
  "event[260] task_dispatched" \
  "event[261] task_quantum_expired" \
  "event[262] task_dispatched" \
  "event[263] task_result_submitted" \
  "event[264] message_sent" \
  "event[265] message_wait_woken" \
  "event[266] task_completed" \
  "event[267] task_dispatched" \
  "event[268] task_result_submitted" \
  "event[269] message_sent" \
  "event[270] task_completed" \
  "event[271] task_dispatched" \
  "event[272] message_received" \
  "event[273] message_acknowledged" \
  "event[274] message_received" \
  "event[275] message_acknowledged" \
  "event[276] runtime_admission_requested" \
  "event[277] runtime_admission_requested" \
  "event[278] message_wait_started" \
  "event[279] task_verified" \
  "event[280] intent_fulfilled" \
  "event[281] task_verified" \
  "event[282] intent_fulfilled" \
  "event[283] runtime_admission_released" \
  "event[284] runtime_admission_released" \
  "event[285] runtime_admission_admitted" \
  "event[286] task_queued" \
  "event[287] runtime_admission_admitted" \
  "event[288] task_queued" \
  "event[289] task_dispatched" \
  "event[290] task_quantum_expired" \
  "event[291] task_dispatched" \
  "event[292] task_quantum_expired" \
  "event[293] task_dispatched" \
  "event[294] task_result_submitted" \
  "event[295] message_sent" \
  "event[296] message_wait_woken" \
  "event[297] task_completed" \
  "event[298] task_dispatched" \
  "event[299] task_result_submitted" \
  "event[300] message_sent" \
  "event[301] task_completed" \
  "event[302] task_dispatched" \
  "event[303] message_received" \
  "event[304] message_acknowledged" \
  "event[305] message_received" \
  "event[306] message_acknowledged" \
  "event[307] runtime_admission_compacted" \
  "event[308] runtime_admission_compacted" \
  "event[309] task_result_submitted" \
  "event[310] task_completed" \
  "event[311] task_verified" \
  "event[312] intent_fulfilled" \
  "event[313] task_verified" \
  "event[314] intent_fulfilled" \
  "event[315] task_verified" \
  "event[316] intent_fulfilled" \
  "event[317] runtime_admission_released" \
  "event[318] runtime_admission_released" \
  "event[319] device_event_raised" \
  "event[320] device_event_delivered" \
  "event[321] driver_invocation_queued" \
  "event[322] driver_invocation_dispatched" \
  "event[323] driver_invocation_ticked" \
  "event[324] device_event_acknowledged" \
  "event[325] driver_command_submitted" \
  "event[326] driver_command_dispatched" \
  "event[327] driver_command_completed" \
  "event[328] driver_invocation_completed" \
  "SUPERVISOR_HANDOFF_READY"
do
  if ! grep -Fq "$expected" <<<"$OUTPUT"; then
    printf 'missing expected QEMU output: %s\n' "$expected" >&2
    exit 1
  fi
done

EVENT_COUNT="$(grep -Fc 'event[' <<<"$OUTPUT")"
if [[ "$EVENT_COUNT" -ne 328 ]]; then
  printf 'expected exactly 328 kernel events, observed %s\n' "$EVENT_COUNT" >&2
  exit 1
fi

check_marker_count() {
  local marker="$1"
  local expected_count="$2"
  MARKER_COUNT="$(grep -Fxc "$marker" <<<"$OUTPUT")"
  if [[ "$MARKER_COUNT" -ne "$expected_count" ]]; then
    printf 'expected exactly %s occurrences of %s, observed %s\n' \
      "$expected_count" "$marker" "$MARKER_COUNT" >&2
    exit 1
  fi
}

check_marker_count "AGENT_KERNEL_AGENT_CALL_ALLOCATE_MEMORY_REGION_OK" 4
check_marker_count "AGENT_KERNEL_AGENT_CALL_INSPECT_MEMORY_REGION_OK" 3
check_marker_count "AGENT_KERNEL_AGENT_CALL_RELEASE_MEMORY_REGION_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_FAULT_MEMORY_RECLAIMED_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_COMPLETION_MEMORY_RECLAIMED_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RECLAIMED_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_FRAME_POOL_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_ALLOCATED_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REBUILT_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_CANCEL_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_BATCH_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_CONCURRENCY_OK" 2
check_marker_count "AGENT_KERNEL_AGENT_CALL_RUNTIME_ADMISSION_REQUEST_OK" 4
check_marker_count "AGENT_KERNEL_AGENT_CALL_RUNTIME_ADMISSION_DISCOVERY_OK" 4
check_marker_count "AGENT_KERNEL_AGENT_CALL_RUNTIME_ADMISSION_COMPACTION_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_REQUEST_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_RESIDENT_WAIT_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_NOTIFICATION_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_SUPERVISOR_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_COMMIT_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_RELEASE_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_PARTIAL_RECLAIM_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_REPEAT_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_COMPACTION_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSE_EXECUTION_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSED_RECLAIMED_OK" 1
