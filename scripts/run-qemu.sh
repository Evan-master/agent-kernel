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
  "AGENT_KERNEL_TASK_PREFIX_VERIFIED_OK" \
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
  "AGENT_KERNEL_RUNTIME_ADMISSION_CAPACITY_OK" \
  "AGENT_KERNEL_AGENT_CALL_RUNTIME_ADMISSION_REQUEST_OK" \
  "AGENT_KERNEL_AGENT_CALL_RUNTIME_ADMISSION_DISCOVERY_OK" \
  "AGENT_KERNEL_AGENT_CALL_RUNTIME_ADMISSION_COMPACTION_OK" \
  "AGENT_KERNEL_AGENT_CALL_TASK_COMPACTION_OK" \
  "AGENT_KERNEL_AGENT_CALL_INTENT_COMPACTION_OK" \
  "AGENT_KERNEL_AGENT_CALL_CAPABILITY_COMPACTION_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_REQUEST_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_RESIDENT_WAIT_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_NOTIFICATION_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_SUPERVISOR_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_COMMIT_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_RELEASE_OK" \
  "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_PARTIAL_RECLAIM_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_REPEAT_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_COMPACTION_OK" \
  "AGENT_KERNEL_NATIVE_TASK_COMPACTION_OK" \
  "AGENT_KERNEL_NATIVE_INTENT_COMPACTION_OK" \
  "AGENT_KERNEL_NATIVE_CAPABILITY_COMPACTION_OK" \
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
  "event[196] task_verified" \
  "event[197] intent_fulfilled" \
  "event[198] task_verified" \
  "event[199] intent_fulfilled" \
  "event[200] task_verified" \
  "event[201] intent_fulfilled" \
  "event[202] task_verified" \
  "event[203] intent_fulfilled" \
  "event[204] task_verified" \
  "event[205] intent_fulfilled" \
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
  "event[222] agent_image_registered" \
  "event[223] agent_image_verified" \
  "event[224] agent_launched" \
  "event[225] task_accepted" \
  "event[226] agent_registered" \
  "event[227] intent_declared" \
  "event[228] task_created" \
  "event[229] intent_bound" \
  "event[230] capability_derived" \
  "event[231] delegation" \
  "event[232] capability_derived" \
  "event[233] agent_image_registered" \
  "event[234] agent_image_verified" \
  "event[235] agent_launched" \
  "event[236] task_accepted" \
  "event[237] task_queued" \
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
  "event[248] agent_registered" \
  "event[249] intent_declared" \
  "event[250] task_created" \
  "event[251] intent_bound" \
  "event[252] capability_derived" \
  "event[253] delegation" \
  "event[254] agent_image_registered" \
  "event[255] agent_image_verified" \
  "event[256] agent_launched" \
  "event[257] task_accepted" \
  "event[258] task_dispatched" \
  "event[259] task_quantum_expired" \
  "event[260] task_dispatched" \
  "event[261] runtime_admission_requested" \
  "event[262] runtime_admission_requested" \
  "event[263] message_wait_started" \
  "event[264] runtime_admission_admitted" \
  "event[265] task_queued" \
  "event[266] runtime_admission_admitted" \
  "event[267] task_queued" \
  "event[268] task_dispatched" \
  "event[269] task_quantum_expired" \
  "event[270] task_dispatched" \
  "event[271] task_quantum_expired" \
  "event[272] task_dispatched" \
  "event[273] task_result_submitted" \
  "event[274] message_sent" \
  "event[275] message_wait_woken" \
  "event[276] task_completed" \
  "event[277] task_dispatched" \
  "event[278] task_result_submitted" \
  "event[279] message_sent" \
  "event[280] task_completed" \
  "event[281] task_dispatched" \
  "event[282] message_received" \
  "event[283] message_acknowledged" \
  "event[284] message_received" \
  "event[285] message_acknowledged" \
  "event[286] runtime_admission_requested" \
  "event[287] runtime_admission_requested" \
  "event[288] message_wait_started" \
  "event[289] task_verified" \
  "event[290] intent_fulfilled" \
  "event[291] task_verified" \
  "event[292] intent_fulfilled" \
  "event[293] runtime_admission_released" \
  "event[294] runtime_admission_released" \
  "event[295] runtime_admission_admitted" \
  "event[296] task_queued" \
  "event[297] runtime_admission_admitted" \
  "event[298] task_queued" \
  "event[299] task_dispatched" \
  "event[300] task_quantum_expired" \
  "event[301] task_dispatched" \
  "event[302] task_quantum_expired" \
  "event[303] task_dispatched" \
  "event[304] task_result_submitted" \
  "event[305] message_sent" \
  "event[306] message_wait_woken" \
  "event[307] task_completed" \
  "event[308] task_dispatched" \
  "event[309] task_result_submitted" \
  "event[310] message_sent" \
  "event[311] task_completed" \
  "event[312] task_dispatched" \
  "event[313] message_received" \
  "event[314] message_acknowledged" \
  "event[315] message_received" \
  "event[316] message_acknowledged" \
  "event[317] runtime_admission_compacted" \
  "event[318] runtime_admission_compacted" \
  "event[319] task_compacted" \
  "event[320] task_compacted" \
  "event[321] task_compacted" \
  "event[322] task_compacted" \
  "event[323] task_compacted" \
  "event[324] task_compacted" \
  "event[325] intent_compacted" \
  "event[326] intent_compacted" \
  "event[327] intent_compacted" \
  "event[328] intent_compacted" \
  "event[329] intent_compacted" \
  "event[330] intent_compacted" \
  "event[331] capability_derived" \
  "event[332] capability_revoked" \
  "event[333] capability_compacted" \
  "event[334] capability_compacted" \
  "event[335] capability_derived" \
  "event[336] capability_derived" \
  "event[337] task_result_submitted" \
  "event[338] task_completed" \
  "event[339] task_verified" \
  "event[340] intent_fulfilled" \
  "event[341] task_verified" \
  "event[342] intent_fulfilled" \
  "event[343] task_verified" \
  "event[344] intent_fulfilled" \
  "event[345] runtime_admission_released" \
  "event[346] runtime_admission_released" \
  "event[347] device_event_raised" \
  "event[348] device_event_delivered" \
  "event[349] driver_invocation_queued" \
  "event[350] driver_invocation_dispatched" \
  "event[351] driver_invocation_ticked" \
  "event[352] device_event_acknowledged" \
  "event[353] driver_command_submitted" \
  "event[354] driver_command_dispatched" \
  "event[355] driver_command_completed" \
  "event[356] driver_invocation_completed" \
  "SUPERVISOR_HANDOFF_READY"
do
  if ! grep -Fq "$expected" <<<"$OUTPUT"; then
    printf 'missing expected QEMU output: %s\n' "$expected" >&2
    exit 1
  fi
done

EVENT_COUNT="$(grep -Fc 'event[' <<<"$OUTPUT")"
if [[ "$EVENT_COUNT" -ne 356 ]]; then
  printf 'expected exactly 356 kernel events, observed %s\n' "$EVENT_COUNT" >&2
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
check_marker_count "AGENT_KERNEL_RUNTIME_ADMISSION_CAPACITY_OK" 1
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
check_marker_count "AGENT_KERNEL_TASK_PREFIX_VERIFIED_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_TASK_COMPACTION_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_TASK_COMPACTION_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_INTENT_COMPACTION_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_INTENT_COMPACTION_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_CAPABILITY_COMPACTION_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_CAPABILITY_COMPACTION_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSE_EXECUTION_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSED_RECLAIMED_OK" 1
