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
  "AGENT_KERNEL_NATIVE_ORPHANED_MESSAGE_RETIREMENT_OK" \
  "AGENT_KERNEL_AGENT_CALL_AGENT_RECORD_RETIREMENT_OK" \
  "AGENT_KERNEL_NATIVE_RESOURCE_MANAGER_AGENT_OK" \
  "AGENT_KERNEL_NATIVE_CAPABILITY_MANAGER_OK" \
  "AGENT_KERNEL_NATIVE_TASK_MANAGER_OK" \
  "AGENT_KERNEL_TASK_PREFIX_VERIFIED_OK" \
  "AGENT_KERNEL_NATIVE_AGENT_MANAGER_OK" \
  "AGENT_KERNEL_NATIVE_AGENT_RECORD_RETIREMENT_OK" \
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
  "AGENT_KERNEL_AGENT_CALL_AGENT_ENTRY_RETIREMENT_OK" \
  "AGENT_KERNEL_AGENT_CALL_MESSAGE_RETIREMENT_OK" \
  "AGENT_KERNEL_AGENT_CALL_CAPABILITY_COMPACTION_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_REQUEST_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_RESIDENT_WAIT_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_NOTIFICATION_OK" \
  "AGENT_KERNEL_NATIVE_MESSAGE_RETIREMENT_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_SUPERVISOR_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_COMMIT_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_RELEASE_OK" \
  "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_PARTIAL_RECLAIM_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_REPEAT_OK" \
  "AGENT_KERNEL_NATIVE_RUNTIME_ADMISSION_COMPACTION_OK" \
  "AGENT_KERNEL_NATIVE_TASK_COMPACTION_OK" \
  "AGENT_KERNEL_NATIVE_INTENT_COMPACTION_OK" \
  "AGENT_KERNEL_NATIVE_AGENT_ENTRY_RETIREMENT_OK" \
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
  "AGENT_KERNEL_AGENT_CALL_ORPHANED_MESSAGE_RETIREMENT_OK" \
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
  "event[171] message_sent" \
  "event[172] agent_suspended" \
  "event[173] agent_resumed" \
  "event[174] agent_retired" \
  "event[175] orphaned_message_retired" \
  "event[176] agent_record_retired" \
  "event[177] agent_registered" \
  "event[178] resource_created" \
  "event[179] capability_granted" \
  "event[180] memory_cell_created" \
  "event[181] memory_cell_recalled" \
  "event[182] resource_retired" \
  "event[183] resource_created" \
  "event[184] capability_granted" \
  "event[185] memory_cell_created" \
  "event[186] memory_cell_recalled" \
  "event[187] resource_created" \
  "event[188] capability_granted" \
  "event[189] memory_cell_created" \
  "event[190] resource_retired" \
  "event[191] memory_cell_recalled" \
  "event[192] resource_created" \
  "event[193] capability_granted" \
  "event[194] memory_cell_created" \
  "event[195] memory_cell_recalled" \
  "event[196] resource_retired" \
  "event[197] task_result_submitted" \
  "event[198] resource_retired" \
  "event[199] task_completed" \
  "event[200] task_verified" \
  "event[201] intent_fulfilled" \
  "event[202] task_verified" \
  "event[203] intent_fulfilled" \
  "event[204] task_verified" \
  "event[205] intent_fulfilled" \
  "event[206] task_verified" \
  "event[207] intent_fulfilled" \
  "event[208] task_verified" \
  "event[209] intent_fulfilled" \
  "event[210] agent_registered" \
  "event[211] intent_declared" \
  "event[212] task_created" \
  "event[213] intent_bound" \
  "event[214] capability_derived" \
  "event[215] delegation" \
  "event[216] agent_image_registered" \
  "event[217] agent_image_verified" \
  "event[218] agent_launched" \
  "event[219] task_accepted" \
  "event[220] agent_registered" \
  "event[221] intent_declared" \
  "event[222] task_created" \
  "event[223] intent_bound" \
  "event[224] capability_derived" \
  "event[225] delegation" \
  "event[226] agent_image_registered" \
  "event[227] agent_image_verified" \
  "event[228] agent_launched" \
  "event[229] task_accepted" \
  "event[230] agent_registered" \
  "event[231] intent_declared" \
  "event[232] task_created" \
  "event[233] intent_bound" \
  "event[234] capability_derived" \
  "event[235] delegation" \
  "event[236] capability_derived" \
  "event[237] agent_image_registered" \
  "event[238] agent_image_verified" \
  "event[239] agent_launched" \
  "event[240] task_accepted" \
  "event[241] task_queued" \
  "event[242] agent_registered" \
  "event[243] intent_declared" \
  "event[244] task_created" \
  "event[245] intent_bound" \
  "event[246] capability_derived" \
  "event[247] delegation" \
  "event[248] agent_image_registered" \
  "event[249] agent_image_verified" \
  "event[250] agent_launched" \
  "event[251] task_accepted" \
  "event[252] agent_registered" \
  "event[253] intent_declared" \
  "event[254] task_created" \
  "event[255] intent_bound" \
  "event[256] capability_derived" \
  "event[257] delegation" \
  "event[258] agent_image_registered" \
  "event[259] agent_image_verified" \
  "event[260] agent_launched" \
  "event[261] task_accepted" \
  "event[262] task_dispatched" \
  "event[263] task_quantum_expired" \
  "event[264] task_dispatched" \
  "event[265] runtime_admission_requested" \
  "event[266] runtime_admission_requested" \
  "event[267] message_wait_started" \
  "event[268] runtime_admission_admitted" \
  "event[269] task_queued" \
  "event[270] runtime_admission_admitted" \
  "event[271] task_queued" \
  "event[272] task_dispatched" \
  "event[273] task_quantum_expired" \
  "event[274] task_dispatched" \
  "event[275] task_quantum_expired" \
  "event[276] task_dispatched" \
  "event[277] task_result_submitted" \
  "event[278] message_sent" \
  "event[279] message_wait_woken" \
  "event[280] task_completed" \
  "event[281] task_dispatched" \
  "event[282] task_result_submitted" \
  "event[283] message_sent" \
  "event[284] task_completed" \
  "event[285] task_dispatched" \
  "event[286] message_received" \
  "event[287] message_acknowledged" \
  "event[288] message_retired" \
  "event[289] message_received" \
  "event[290] message_acknowledged" \
  "event[291] message_retired" \
  "event[292] runtime_admission_requested" \
  "event[293] runtime_admission_requested" \
  "event[294] message_wait_started" \
  "event[295] task_verified" \
  "event[296] intent_fulfilled" \
  "event[297] task_verified" \
  "event[298] intent_fulfilled" \
  "event[299] runtime_admission_released" \
  "event[300] runtime_admission_released" \
  "event[301] runtime_admission_admitted" \
  "event[302] task_queued" \
  "event[303] runtime_admission_admitted" \
  "event[304] task_queued" \
  "event[305] task_dispatched" \
  "event[306] task_quantum_expired" \
  "event[307] task_dispatched" \
  "event[308] task_quantum_expired" \
  "event[309] task_dispatched" \
  "event[310] task_result_submitted" \
  "event[311] message_sent" \
  "event[312] message_wait_woken" \
  "event[313] task_completed" \
  "event[314] task_dispatched" \
  "event[315] task_result_submitted" \
  "event[316] message_sent" \
  "event[317] task_completed" \
  "event[318] task_dispatched" \
  "event[319] message_received" \
  "event[320] message_acknowledged" \
  "event[321] message_retired" \
  "event[322] message_received" \
  "event[323] message_acknowledged" \
  "event[324] message_retired" \
  "event[325] runtime_admission_compacted" \
  "event[326] runtime_admission_compacted" \
  "event[327] task_compacted" \
  "event[328] task_compacted" \
  "event[329] task_compacted" \
  "event[330] task_compacted" \
  "event[331] task_compacted" \
  "event[332] task_compacted" \
  "event[333] intent_compacted" \
  "event[334] intent_compacted" \
  "event[335] intent_compacted" \
  "event[336] intent_compacted" \
  "event[337] intent_compacted" \
  "event[338] intent_compacted" \
  "event[339] agent_entry_retired" \
  "event[340] agent_entry_retired" \
  "event[341] capability_derived" \
  "event[342] capability_revoked" \
  "event[343] capability_compacted" \
  "event[344] capability_compacted" \
  "event[345] capability_derived" \
  "event[346] capability_derived" \
  "event[347] task_result_submitted" \
  "event[348] task_completed" \
  "event[349] task_verified" \
  "event[350] intent_fulfilled" \
  "event[351] task_verified" \
  "event[352] intent_fulfilled" \
  "event[353] task_verified" \
  "event[354] intent_fulfilled" \
  "event[355] runtime_admission_released" \
  "event[356] runtime_admission_released" \
  "event[357] device_event_raised" \
  "event[358] device_event_delivered" \
  "event[359] driver_invocation_queued" \
  "event[360] driver_invocation_dispatched" \
  "event[361] driver_invocation_ticked" \
  "event[362] device_event_acknowledged" \
  "event[363] driver_command_submitted" \
  "event[364] driver_command_dispatched" \
  "event[365] driver_command_completed" \
  "event[366] driver_invocation_completed" \
  "SUPERVISOR_HANDOFF_READY"
do
  if ! grep -Fq "$expected" <<<"$OUTPUT"; then
    printf 'missing expected QEMU output: %s\n' "$expected" >&2
    exit 1
  fi
done

EVENT_COUNT="$(grep -Fc 'event[' <<<"$OUTPUT")"
if [[ "$EVENT_COUNT" -ne 366 ]]; then
  printf 'expected exactly 366 kernel events, observed %s\n' "$EVENT_COUNT" >&2
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
check_marker_count "AGENT_KERNEL_AGENT_CALL_ORPHANED_MESSAGE_RETIREMENT_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_ORPHANED_MESSAGE_RETIREMENT_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_AGENT_RECORD_RETIREMENT_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_AGENT_RECORD_RETIREMENT_OK" 1
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
check_marker_count "AGENT_KERNEL_AGENT_CALL_MESSAGE_RETIREMENT_OK" 4
check_marker_count "AGENT_KERNEL_NATIVE_MESSAGE_RETIREMENT_OK" 1
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
check_marker_count "AGENT_KERNEL_AGENT_CALL_AGENT_ENTRY_RETIREMENT_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_AGENT_ENTRY_RETIREMENT_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_CAPABILITY_COMPACTION_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_CAPABILITY_COMPACTION_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSE_EXECUTION_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSED_RECLAIMED_OK" 1
