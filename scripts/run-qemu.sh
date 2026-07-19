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
  "AGENT_KERNEL_AGENT_CALL_AGENT_IMAGE_RECORD_RETIREMENT_OK" \
  "AGENT_KERNEL_NATIVE_RESOURCE_MANAGER_AGENT_OK" \
  "AGENT_KERNEL_NATIVE_CAPABILITY_MANAGER_OK" \
  "AGENT_KERNEL_NATIVE_TASK_MANAGER_OK" \
  "AGENT_KERNEL_TASK_PREFIX_VERIFIED_OK" \
  "AGENT_KERNEL_NATIVE_AGENT_MANAGER_OK" \
  "AGENT_KERNEL_NATIVE_AGENT_RECORD_RETIREMENT_OK" \
  "AGENT_KERNEL_NATIVE_AGENT_IMAGE_RECORD_RETIREMENT_OK" \
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
  "AGENT_KERNEL_NATIVE_AGENT_IMAGE_SLOT_REUSE_OK" \
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
  "event[83] agent_image_registered" \
  "event[84] agent_image_retired" \
  "event[85] agent_launched" \
  "event[86] task_accepted" \
  "event[87] task_dispatched" \
  "event[88] task_quantum_expired" \
  "event[89] task_dispatched" \
  "event[90] task_quantum_expired" \
  "event[91] task_dispatched" \
  "event[92] message_wait_started" \
  "event[93] task_dispatched" \
  "event[94] task_result_submitted" \
  "event[95] message_sent" \
  "event[96] message_wait_woken" \
  "event[97] task_quantum_expired" \
  "event[98] task_dispatched" \
  "event[99] message_received" \
  "event[100] message_acknowledged" \
  "event[101] task_result_submitted" \
  "event[102] task_completed" \
  "event[103] task_dispatched" \
  "event[104] task_yielded" \
  "event[105] task_dispatched" \
  "event[106] task_completed" \
  "event[107] task_queued" \
  "event[108] task_dispatched" \
  "event[109] task_quantum_expired" \
  "event[110] task_dispatched" \
  "event[111] message_wait_started" \
  "event[112] task_queued" \
  "event[113] task_queued" \
  "event[114] task_dispatched" \
  "event[115] task_quantum_expired" \
  "event[116] task_dispatched" \
  "event[117] task_quantum_expired" \
  "event[118] task_dispatched" \
  "event[119] memory_cell_created" \
  "event[120] resource_retired" \
  "event[121] task_faulted" \
  "event[122] task_dispatched" \
  "event[123] task_result_inspected" \
  "event[124] task_verified" \
  "event[125] intent_fulfilled" \
  "event[126] task_completed" \
  "event[127] task_fault_recovered" \
  "event[128] task_queued" \
  "event[129] task_dispatched" \
  "event[130] task_quantum_expired" \
  "event[131] task_dispatched" \
  "event[132] task_faulted" \
  "event[133] task_fault_recovered" \
  "event[134] task_queued" \
  "event[135] task_dispatched" \
  "event[136] task_quantum_expired" \
  "event[137] task_dispatched" \
  "event[138] task_faulted" \
  "event[139] task_fault_recovered" \
  "event[140] task_queued" \
  "event[141] task_dispatched" \
  "event[142] task_quantum_expired" \
  "event[143] task_dispatched" \
  "event[144] task_faulted" \
  "event[145] message_sent" \
  "event[146] message_wait_woken" \
  "event[147] fault_routed" \
  "event[148] fault_policy_applied" \
  "event[149] task_dispatched" \
  "event[150] message_received" \
  "event[151] message_acknowledged" \
  "event[152] task_result_submitted" \
  "event[153] task_completed" \
  "event[154] task_fault_recovered" \
  "event[155] task_queued" \
  "event[156] task_dispatched" \
  "event[157] task_completed" \
  "event[158] task_queued" \
  "event[159] task_dispatched" \
  "event[160] task_quantum_expired" \
  "event[161] task_dispatched" \
  "event[162] resource_created" \
  "event[163] capability_granted" \
  "event[164] capability_derived" \
  "event[165] capability_revoked" \
  "event[166] resource_retired" \
  "event[167] intent_declared" \
  "event[168] task_created" \
  "event[169] intent_bound" \
  "event[170] capability_derived" \
  "event[171] delegation" \
  "event[172] agent_registered" \
  "event[173] message_sent" \
  "event[174] agent_suspended" \
  "event[175] agent_resumed" \
  "event[176] agent_retired" \
  "event[177] orphaned_message_retired" \
  "event[178] agent_record_retired" \
  "event[179] agent_image_record_retired" \
  "event[180] agent_registered" \
  "event[181] resource_created" \
  "event[182] capability_granted" \
  "event[183] memory_cell_created" \
  "event[184] memory_cell_recalled" \
  "event[185] resource_retired" \
  "event[186] resource_created" \
  "event[187] capability_granted" \
  "event[188] memory_cell_created" \
  "event[189] memory_cell_recalled" \
  "event[190] resource_created" \
  "event[191] capability_granted" \
  "event[192] memory_cell_created" \
  "event[193] resource_retired" \
  "event[194] memory_cell_recalled" \
  "event[195] resource_created" \
  "event[196] capability_granted" \
  "event[197] memory_cell_created" \
  "event[198] memory_cell_recalled" \
  "event[199] resource_retired" \
  "event[200] task_result_submitted" \
  "event[201] resource_retired" \
  "event[202] task_completed" \
  "event[203] task_verified" \
  "event[204] intent_fulfilled" \
  "event[205] task_verified" \
  "event[206] intent_fulfilled" \
  "event[207] task_verified" \
  "event[208] intent_fulfilled" \
  "event[209] task_verified" \
  "event[210] intent_fulfilled" \
  "event[211] task_verified" \
  "event[212] intent_fulfilled" \
  "event[213] agent_registered" \
  "event[214] intent_declared" \
  "event[215] task_created" \
  "event[216] intent_bound" \
  "event[217] capability_derived" \
  "event[218] delegation" \
  "event[219] agent_image_registered" \
  "event[220] agent_image_verified" \
  "event[221] agent_launched" \
  "event[222] task_accepted" \
  "event[223] agent_registered" \
  "event[224] intent_declared" \
  "event[225] task_created" \
  "event[226] intent_bound" \
  "event[227] capability_derived" \
  "event[228] delegation" \
  "event[229] agent_image_registered" \
  "event[230] agent_image_verified" \
  "event[231] agent_launched" \
  "event[232] task_accepted" \
  "event[233] agent_registered" \
  "event[234] intent_declared" \
  "event[235] task_created" \
  "event[236] intent_bound" \
  "event[237] capability_derived" \
  "event[238] delegation" \
  "event[239] capability_derived" \
  "event[240] agent_image_registered" \
  "event[241] agent_image_verified" \
  "event[242] agent_launched" \
  "event[243] task_accepted" \
  "event[244] task_queued" \
  "event[245] agent_registered" \
  "event[246] intent_declared" \
  "event[247] task_created" \
  "event[248] intent_bound" \
  "event[249] capability_derived" \
  "event[250] delegation" \
  "event[251] agent_image_registered" \
  "event[252] agent_image_verified" \
  "event[253] agent_launched" \
  "event[254] task_accepted" \
  "event[255] agent_registered" \
  "event[256] intent_declared" \
  "event[257] task_created" \
  "event[258] intent_bound" \
  "event[259] capability_derived" \
  "event[260] delegation" \
  "event[261] agent_image_registered" \
  "event[262] agent_image_verified" \
  "event[263] agent_launched" \
  "event[264] task_accepted" \
  "event[265] task_dispatched" \
  "event[266] task_quantum_expired" \
  "event[267] task_dispatched" \
  "event[268] runtime_admission_requested" \
  "event[269] runtime_admission_requested" \
  "event[270] message_wait_started" \
  "event[271] runtime_admission_admitted" \
  "event[272] task_queued" \
  "event[273] runtime_admission_admitted" \
  "event[274] task_queued" \
  "event[275] task_dispatched" \
  "event[276] task_quantum_expired" \
  "event[277] task_dispatched" \
  "event[278] task_quantum_expired" \
  "event[279] task_dispatched" \
  "event[280] task_result_submitted" \
  "event[281] message_sent" \
  "event[282] message_wait_woken" \
  "event[283] task_completed" \
  "event[284] task_dispatched" \
  "event[285] task_result_submitted" \
  "event[286] message_sent" \
  "event[287] task_completed" \
  "event[288] task_dispatched" \
  "event[289] message_received" \
  "event[290] message_acknowledged" \
  "event[291] message_retired" \
  "event[292] message_received" \
  "event[293] message_acknowledged" \
  "event[294] message_retired" \
  "event[295] runtime_admission_requested" \
  "event[296] runtime_admission_requested" \
  "event[297] message_wait_started" \
  "event[298] task_verified" \
  "event[299] intent_fulfilled" \
  "event[300] task_verified" \
  "event[301] intent_fulfilled" \
  "event[302] runtime_admission_released" \
  "event[303] runtime_admission_released" \
  "event[304] runtime_admission_admitted" \
  "event[305] task_queued" \
  "event[306] runtime_admission_admitted" \
  "event[307] task_queued" \
  "event[308] task_dispatched" \
  "event[309] task_quantum_expired" \
  "event[310] task_dispatched" \
  "event[311] task_quantum_expired" \
  "event[312] task_dispatched" \
  "event[313] task_result_submitted" \
  "event[314] message_sent" \
  "event[315] message_wait_woken" \
  "event[316] task_completed" \
  "event[317] task_dispatched" \
  "event[318] task_result_submitted" \
  "event[319] message_sent" \
  "event[320] task_completed" \
  "event[321] task_dispatched" \
  "event[322] message_received" \
  "event[323] message_acknowledged" \
  "event[324] message_retired" \
  "event[325] message_received" \
  "event[326] message_acknowledged" \
  "event[327] message_retired" \
  "event[328] runtime_admission_compacted" \
  "event[329] runtime_admission_compacted" \
  "event[330] task_compacted" \
  "event[331] task_compacted" \
  "event[332] task_compacted" \
  "event[333] task_compacted" \
  "event[334] task_compacted" \
  "event[335] task_compacted" \
  "event[336] intent_compacted" \
  "event[337] intent_compacted" \
  "event[338] intent_compacted" \
  "event[339] intent_compacted" \
  "event[340] intent_compacted" \
  "event[341] intent_compacted" \
  "event[342] agent_entry_retired" \
  "event[343] agent_entry_retired" \
  "event[344] capability_derived" \
  "event[345] capability_revoked" \
  "event[346] capability_compacted" \
  "event[347] capability_compacted" \
  "event[348] capability_derived" \
  "event[349] capability_derived" \
  "event[350] task_result_submitted" \
  "event[351] task_completed" \
  "event[352] task_verified" \
  "event[353] intent_fulfilled" \
  "event[354] task_verified" \
  "event[355] intent_fulfilled" \
  "event[356] task_verified" \
  "event[357] intent_fulfilled" \
  "event[358] runtime_admission_released" \
  "event[359] runtime_admission_released" \
  "event[360] agent_image_registered" \
  "event[361] device_event_raised" \
  "event[362] device_event_delivered" \
  "event[363] driver_invocation_queued" \
  "event[364] driver_invocation_dispatched" \
  "event[365] driver_invocation_ticked" \
  "event[366] device_event_acknowledged" \
  "event[367] driver_command_submitted" \
  "event[368] driver_command_dispatched" \
  "event[369] driver_command_completed" \
  "event[370] driver_invocation_completed" \
  "SUPERVISOR_HANDOFF_READY"
do
  if ! grep -Fq "$expected" <<<"$OUTPUT"; then
    printf 'missing expected QEMU output: %s\n' "$expected" >&2
    exit 1
  fi
done

EVENT_COUNT="$(grep -Fc 'event[' <<<"$OUTPUT")"
if [[ "$EVENT_COUNT" -ne 370 ]]; then
  printf 'expected exactly 370 kernel events, observed %s\n' "$EVENT_COUNT" >&2
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
check_marker_count "AGENT_KERNEL_AGENT_CALL_AGENT_IMAGE_RECORD_RETIREMENT_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_AGENT_IMAGE_RECORD_RETIREMENT_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_AGENT_IMAGE_SLOT_REUSE_OK" 1
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
