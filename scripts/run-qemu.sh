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
  "AGENT_KERNEL_NATIVE_FAULT_STORE_FULL_OK" \
  "AGENT_KERNEL_AGENT_CALL_RUNTIME_ADMISSION_REQUEST_OK" \
  "AGENT_KERNEL_AGENT_CALL_RUNTIME_ADMISSION_DISCOVERY_OK" \
  "AGENT_KERNEL_AGENT_CALL_RUNTIME_ADMISSION_COMPACTION_OK" \
  "AGENT_KERNEL_AGENT_CALL_TASK_COMPACTION_OK" \
  "AGENT_KERNEL_AGENT_CALL_FAULT_COMPACTION_OK" \
  "AGENT_KERNEL_AGENT_CALL_INTENT_COMPACTION_OK" \
  "AGENT_KERNEL_AGENT_CALL_AGENT_ENTRY_RETIREMENT_OK" \
  "AGENT_KERNEL_AGENT_CALL_MESSAGE_RETIREMENT_OK" \
  "AGENT_KERNEL_AGENT_CALL_CAPABILITY_CLEANUP_REVOCATION_OK" \
  "AGENT_KERNEL_AGENT_CALL_CAPABILITY_COMPACTION_OK" \
  "AGENT_KERNEL_AGENT_CALL_RESOURCE_RECORD_RETIREMENT_OK" \
  "AGENT_KERNEL_AGENT_CALL_MEMORY_CELL_RECORD_RETIREMENT_OK" \
  "AGENT_KERNEL_AGENT_CALL_WAITER_COMPACTION_OK" \
  "AGENT_KERNEL_AGENT_CALL_EVENT_ARCHIVE_OK" \
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
  "AGENT_KERNEL_NATIVE_FAULT_COMPACTION_OK" \
  "AGENT_KERNEL_NATIVE_INTENT_COMPACTION_OK" \
  "AGENT_KERNEL_NATIVE_AGENT_ENTRY_RETIREMENT_OK" \
  "AGENT_KERNEL_NATIVE_CAPABILITY_CLEANUP_REVOCATION_OK" \
  "AGENT_KERNEL_NATIVE_CAPABILITY_COMPACTION_OK" \
  "AGENT_KERNEL_NATIVE_RESOURCE_RECORD_RETIREMENT_OK" \
  "AGENT_KERNEL_NATIVE_RESOURCE_STORE_REUSE_OK" \
  "AGENT_KERNEL_NATIVE_MEMORY_CELL_RECORD_RETIREMENT_OK" \
  "AGENT_KERNEL_NATIVE_MEMORY_CELL_STORE_REUSE_OK" \
  "AGENT_KERNEL_NATIVE_WAITER_SLOT_REUSE_OK" \
  "AGENT_KERNEL_NATIVE_WAITER_COMPACTION_OK" \
  "AGENT_KERNEL_NATIVE_EVENT_LOG_FULL_OK" \
  "AGENT_KERNEL_NATIVE_EVENT_ARCHIVE_OK" \
  "AGENT_KERNEL_NATIVE_EVENT_ARCHIVE_REPLAY_OK" \
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
  "event[186] namespace_entry_bound" \
  "event[187] namespace_entry_resolved" \
  "event[188] namespace_entry_rebound" \
  "event[189] namespace_entry_retired" \
  "event[190] namespace_entry_bound" \
  "event[191] resource_created" \
  "event[192] capability_granted" \
  "event[193] memory_cell_created" \
  "event[194] memory_cell_recalled" \
  "event[195] resource_created" \
  "event[196] capability_granted" \
  "event[197] memory_cell_created" \
  "event[198] resource_retired" \
  "event[199] memory_cell_recalled" \
  "event[200] resource_created" \
  "event[201] capability_granted" \
  "event[202] memory_cell_created" \
  "event[203] memory_cell_recalled" \
  "event[204] resource_retired" \
  "event[205] task_result_submitted" \
  "event[206] resource_retired" \
  "event[207] task_completed" \
  "event[208] task_verified" \
  "event[209] intent_fulfilled" \
  "event[210] task_verified" \
  "event[211] intent_fulfilled" \
  "event[212] task_verified" \
  "event[213] intent_fulfilled" \
  "event[214] task_verified" \
  "event[215] intent_fulfilled" \
  "event[216] task_verified" \
  "event[217] intent_fulfilled" \
  "event[218] agent_registered" \
  "event[219] intent_declared" \
  "event[220] task_created" \
  "event[221] intent_bound" \
  "event[222] capability_derived" \
  "event[223] delegation" \
  "event[224] agent_image_registered" \
  "event[225] agent_image_verified" \
  "event[226] agent_launched" \
  "event[227] task_accepted" \
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
  "event[244] capability_derived" \
  "event[245] agent_image_registered" \
  "event[246] agent_image_verified" \
  "event[247] agent_launched" \
  "event[248] task_accepted" \
  "event[249] task_queued" \
  "event[250] agent_registered" \
  "event[251] intent_declared" \
  "event[252] task_created" \
  "event[253] intent_bound" \
  "event[254] capability_derived" \
  "event[255] delegation" \
  "event[256] agent_image_registered" \
  "event[257] agent_image_verified" \
  "event[258] agent_launched" \
  "event[259] task_accepted" \
  "event[260] agent_registered" \
  "event[261] intent_declared" \
  "event[262] task_created" \
  "event[263] intent_bound" \
  "event[264] capability_derived" \
  "event[265] delegation" \
  "event[266] agent_image_registered" \
  "event[267] agent_image_verified" \
  "event[268] agent_launched" \
  "event[269] task_accepted" \
  "event[270] task_dispatched" \
  "event[271] task_quantum_expired" \
  "event[272] task_dispatched" \
  "event[273] runtime_admission_requested" \
  "event[274] runtime_admission_requested" \
  "event[275] message_wait_started" \
  "event[276] runtime_admission_admitted" \
  "event[277] task_queued" \
  "event[278] runtime_admission_admitted" \
  "event[279] task_queued" \
  "event[280] task_dispatched" \
  "event[281] task_quantum_expired" \
  "event[282] task_dispatched" \
  "event[283] task_quantum_expired" \
  "event[284] task_dispatched" \
  "event[285] task_result_submitted" \
  "event[286] message_sent" \
  "event[287] message_wait_woken" \
  "event[288] task_completed" \
  "event[289] task_dispatched" \
  "event[290] task_result_submitted" \
  "event[291] message_sent" \
  "event[292] task_completed" \
  "event[293] task_dispatched" \
  "event[294] message_received" \
  "event[295] message_acknowledged" \
  "event[296] message_retired" \
  "event[297] message_received" \
  "event[298] message_acknowledged" \
  "event[299] message_retired" \
  "event[300] waiter_compacted" \
  "event[301] waiter_compacted" \
  "event[302] waiter_compacted" \
  "event[303] runtime_admission_requested" \
  "event[304] runtime_admission_requested" \
  "event[305] message_wait_started" \
  "event[306] task_verified" \
  "event[307] intent_fulfilled" \
  "event[308] task_verified" \
  "event[309] intent_fulfilled" \
  "event[310] runtime_admission_released" \
  "event[311] runtime_admission_released" \
  "event[312] runtime_admission_admitted" \
  "event[313] task_queued" \
  "event[314] runtime_admission_admitted" \
  "event[315] task_queued" \
  "event[316] task_dispatched" \
  "event[317] task_quantum_expired" \
  "event[318] task_dispatched" \
  "event[319] task_quantum_expired" \
  "event[320] task_dispatched" \
  "event[321] task_result_submitted" \
  "event[322] message_sent" \
  "event[323] message_wait_woken" \
  "event[324] task_completed" \
  "event[325] task_dispatched" \
  "event[326] task_result_submitted" \
  "event[327] message_sent" \
  "event[328] task_completed" \
  "event[329] task_dispatched" \
  "event[330] message_received" \
  "event[331] message_acknowledged" \
  "event[332] message_retired" \
  "event[333] message_received" \
  "event[334] message_acknowledged" \
  "event[335] message_retired" \
  "event[336] waiter_compacted" \
  "event[337] runtime_admission_compacted" \
  "event[338] runtime_admission_compacted" \
  "event[339] task_compacted" \
  "event[340] task_compacted" \
  "event[341] task_compacted" \
  "event[342] task_compacted" \
  "event[343] task_compacted" \
  "event[344] task_compacted" \
  "event[345] fault_compacted" \
  "event[346] fault_compacted" \
  "event[347] fault_compacted" \
  "event[348] fault_compacted" \
  "event[349] intent_compacted" \
  "event[350] intent_compacted" \
  "event[351] intent_compacted" \
  "event[352] intent_compacted" \
  "event[353] intent_compacted" \
  "event[354] intent_compacted" \
  "event[355] agent_entry_retired" \
  "event[356] agent_entry_retired" \
  "event[357] capability_derived" \
  "event[358] capability_revoked" \
  "event[359] capability_revoked" \
  "event[360] capability_compacted" \
  "event[361] capability_compacted" \
  "event[362] capability_compacted" \
  "event[363] resource_record_retired" \
  "event[364] resource_created" \
  "event[365] capability_granted" \
  "event[366] capability_derived" \
  "event[367] capability_derived" \
  "event[368] memory_cell_record_retired" \
  "event[369] capability_revoked" \
  "event[370] capability_compacted" \
  "event[371] resource_record_retired" \
  "event[372] resource_created" \
  "event[373] capability_granted" \
  "event[374] memory_cell_created" \
  "event[375] task_result_submitted" \
  "event[376] resource_retired" \
  "event[377] task_completed" \
  "event[378] task_verified" \
  "event[379] intent_fulfilled" \
  "event[380] task_verified" \
  "event[381] intent_fulfilled" \
  "event[382] task_verified" \
  "event[383] intent_fulfilled" \
  "event[384] runtime_admission_released" \
  "event[385] runtime_admission_released" \
  "event[386] agent_image_registered" \
  "event[387] device_event_raised" \
  "event[388] device_event_delivered" \
  "event[389] driver_invocation_queued" \
  "event[390] driver_invocation_dispatched" \
  "event[391] driver_invocation_ticked" \
  "event[392] device_event_acknowledged" \
  "event[393] driver_command_submitted" \
  "event[394] driver_command_dispatched" \
  "event[395] driver_command_completed" \
  "event[396] driver_invocation_completed" \
  "SUPERVISOR_HANDOFF_READY"
do
  if ! grep -Fq "$expected" <<<"$OUTPUT"; then
    printf 'missing expected QEMU output: %s\n' "$expected" >&2
    exit 1
  fi
done

EVENT_COUNT="$(grep -Ec '^event\[[0-9]+\] ' <<<"$OUTPUT")"
if [[ "$EVENT_COUNT" -ne 396 ]]; then
  printf 'expected exactly 396 kernel events, observed %s\n' "$EVENT_COUNT" >&2
  exit 1
fi

EXPECTED_EVENT_SEQUENCE=1
while IFS= read -r event_line; do
  expected_prefix="event[$EXPECTED_EVENT_SEQUENCE] "
  if [[ "$event_line" != "$expected_prefix"* ]]; then
    printf 'expected ordered kernel event %s, observed %s\n' \
      "$EXPECTED_EVENT_SEQUENCE" "$event_line" >&2
    exit 1
  fi
  EXPECTED_EVENT_SEQUENCE=$((EXPECTED_EVENT_SEQUENCE + 1))
done < <(grep -E '^event\[[0-9]+\] ' <<<"$OUTPUT")

if [[ "$EXPECTED_EVENT_SEQUENCE" -ne 397 ]]; then
  printf 'ordered kernel event sequence ended at %s\n' \
    "$((EXPECTED_EVENT_SEQUENCE - 1))" >&2
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
check_marker_count "AGENT_KERNEL_AGENT_CALL_ALLOCATE_MEMORY_PAGE_OK" 2
check_marker_count "AGENT_KERNEL_AGENT_CALL_CAPABILITY_CLEANUP_REVOCATION_OK" 2
check_marker_count "AGENT_KERNEL_AGENT_CALL_CAPABILITY_COMPACTION_OK" 4
check_marker_count "AGENT_KERNEL_AGENT_CALL_RESOURCE_RECORD_RETIREMENT_OK" 2
check_marker_count "AGENT_KERNEL_AGENT_CALL_MEMORY_CELL_RECORD_RETIREMENT_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_NAMESPACE_BIND_OK" 2
check_marker_count "AGENT_KERNEL_AGENT_CALL_NAMESPACE_RESOLVE_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_NAMESPACE_REBIND_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_NAMESPACE_RETIREMENT_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_NAMESPACE_MANAGER_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_NAMESPACE_SLOT_REUSE_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_FAULT_MEMORY_RECLAIMED_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_ORPHANED_MESSAGE_RETIREMENT_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_ORPHANED_MESSAGE_RETIREMENT_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_AGENT_RECORD_RETIREMENT_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_AGENT_RECORD_RETIREMENT_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_AGENT_IMAGE_RECORD_RETIREMENT_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_AGENT_IMAGE_RECORD_RETIREMENT_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_AGENT_IMAGE_SLOT_REUSE_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_COMPLETION_MEMORY_RECLAIMED_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RECLAIMED_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_FRAME_POOL_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_ALLOCATED_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REBUILT_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_CANCEL_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_BATCH_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_RUNTIME_CONCURRENCY_OK" 2
check_marker_count "AGENT_KERNEL_RUNTIME_ADMISSION_CAPACITY_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_FAULT_STORE_FULL_OK" 1
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
check_marker_count "AGENT_KERNEL_AGENT_CALL_FAULT_COMPACTION_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_FAULT_COMPACTION_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_INTENT_COMPACTION_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_INTENT_COMPACTION_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_AGENT_ENTRY_RETIREMENT_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_AGENT_ENTRY_RETIREMENT_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_CAPABILITY_CLEANUP_REVOCATION_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_CAPABILITY_COMPACTION_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_RESOURCE_RECORD_RETIREMENT_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_RESOURCE_STORE_REUSE_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_MEMORY_CELL_RECORD_RETIREMENT_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_MEMORY_CELL_STORE_REUSE_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_WAITER_COMPACTION_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_WAITER_SLOT_REUSE_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_WAITER_COMPACTION_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_EVENT_ARCHIVE_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_EVENT_LOG_FULL_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_EVENT_ARCHIVE_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_EVENT_ARCHIVE_REPLAY_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSE_EXECUTION_OK" 2
check_marker_count "AGENT_KERNEL_NATIVE_ADDRESS_SPACE_REUSED_RECLAIMED_OK" 1
