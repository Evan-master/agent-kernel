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
  "AGENT_KERNEL_NATIVE_MULTI_PAGE_CAPSULE_OK" \
  "AGENT_KERNEL_NATIVE_FIFTH_CODE_PAGE_OK" \
  "AGENT_KERNEL_NATIVE_RIGHT_SIZED_CODE_FRAMES_OK" \
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
  "event[166] intent_declared" \
  "event[167] task_created" \
  "event[168] intent_bound" \
  "event[169] capability_derived" \
  "event[170] delegation" \
  "event[171] agent_registered" \
  "event[172] message_sent" \
  "event[173] agent_suspended" \
  "event[174] agent_resumed" \
  "event[175] agent_retired" \
  "event[176] orphaned_message_retired" \
  "event[177] agent_record_retired" \
  "event[178] agent_image_record_retired" \
  "event[179] agent_registered" \
  "event[180] resource_created" \
  "event[181] capability_granted" \
  "event[182] memory_cell_created" \
  "event[183] memory_cell_recalled" \
  "event[184] resource_retired" \
  "event[185] namespace_entry_bound" \
  "event[186] namespace_entry_bound" \
  "event[187] namespace_entry_resolved" \
  "event[188] namespace_entry_resolved" \
  "event[189] resource_created" \
  "event[190] capability_granted" \
  "event[191] memory_cell_created" \
  "event[192] memory_cell_recalled" \
  "event[193] resource_created" \
  "event[194] capability_granted" \
  "event[195] memory_cell_created" \
  "event[196] resource_retired" \
  "event[197] memory_cell_recalled" \
  "event[198] resource_created" \
  "event[199] capability_granted" \
  "event[200] memory_cell_created" \
  "event[201] memory_cell_recalled" \
  "event[202] resource_retired" \
  "event[203] resource_created" \
  "event[204] capability_granted" \
  "event[205] resource_created" \
  "event[206] capability_granted" \
  "event[207] namespace_entry_rebound" \
  "event[208] namespace_entry_bound" \
  "event[209] namespace_entry_bound" \
  "event[210] namespace_entry_resolved" \
  "event[211] namespace_entry_resolved" \
  "event[212] namespace_entry_resolved" \
  "event[213] namespace_entry_resolved" \
  "event[214] namespace_entry_resolved" \
  "event[215] namespace_entry_resolved" \
  "event[216] namespace_entry_resolved" \
  "event[217] namespace_entry_rebound" \
  "event[218] task_result_submitted" \
  "event[219] resource_retired" \
  "event[220] task_completed" \
  "event[221] task_verified" \
  "event[222] intent_fulfilled" \
  "event[223] task_verified" \
  "event[224] intent_fulfilled" \
  "event[225] task_verified" \
  "event[226] intent_fulfilled" \
  "event[227] task_verified" \
  "event[228] intent_fulfilled" \
  "event[229] task_verified" \
  "event[230] intent_fulfilled" \
  "event[231] agent_registered" \
  "event[232] intent_declared" \
  "event[233] task_created" \
  "event[234] intent_bound" \
  "event[235] capability_derived" \
  "event[236] delegation" \
  "event[237] agent_image_registered" \
  "event[238] agent_image_verified" \
  "event[239] agent_launched" \
  "event[240] task_accepted" \
  "event[241] agent_registered" \
  "event[242] intent_declared" \
  "event[243] task_created" \
  "event[244] intent_bound" \
  "event[245] capability_derived" \
  "event[246] delegation" \
  "event[247] agent_image_registered" \
  "event[248] agent_image_verified" \
  "event[249] agent_launched" \
  "event[250] task_accepted" \
  "event[251] agent_registered" \
  "event[252] intent_declared" \
  "event[253] task_created" \
  "event[254] intent_bound" \
  "event[255] capability_derived" \
  "event[256] delegation" \
  "event[257] capability_derived" \
  "event[258] agent_image_registered" \
  "event[259] agent_image_verified" \
  "event[260] agent_launched" \
  "event[261] task_accepted" \
  "event[262] task_queued" \
  "event[263] agent_registered" \
  "event[264] intent_declared" \
  "event[265] task_created" \
  "event[266] intent_bound" \
  "event[267] capability_derived" \
  "event[268] delegation" \
  "event[269] agent_image_registered" \
  "event[270] agent_image_verified" \
  "event[271] agent_launched" \
  "event[272] task_accepted" \
  "event[273] agent_registered" \
  "event[274] intent_declared" \
  "event[275] task_created" \
  "event[276] intent_bound" \
  "event[277] capability_derived" \
  "event[278] delegation" \
  "event[279] agent_image_registered" \
  "event[280] agent_image_verified" \
  "event[281] agent_launched" \
  "event[282] task_accepted" \
  "event[283] task_dispatched" \
  "event[284] task_quantum_expired" \
  "event[285] task_dispatched" \
  "event[286] runtime_admission_requested" \
  "event[287] runtime_admission_requested" \
  "event[288] message_wait_started" \
  "event[289] runtime_admission_admitted" \
  "event[290] task_queued" \
  "event[291] runtime_admission_admitted" \
  "event[292] task_queued" \
  "event[293] task_dispatched" \
  "event[294] task_quantum_expired" \
  "event[295] task_dispatched" \
  "event[296] task_quantum_expired" \
  "event[297] task_dispatched" \
  "event[298] task_result_submitted" \
  "event[299] message_sent" \
  "event[300] message_wait_woken" \
  "event[301] task_completed" \
  "event[302] task_dispatched" \
  "event[303] task_result_submitted" \
  "event[304] message_sent" \
  "event[305] task_completed" \
  "event[306] task_dispatched" \
  "event[307] message_received" \
  "event[308] message_acknowledged" \
  "event[309] message_retired" \
  "event[310] message_received" \
  "event[311] message_acknowledged" \
  "event[312] message_retired" \
  "event[313] waiter_compacted" \
  "event[314] waiter_compacted" \
  "event[315] waiter_compacted" \
  "event[316] runtime_admission_requested" \
  "event[317] runtime_admission_requested" \
  "event[318] message_wait_started" \
  "event[319] task_verified" \
  "event[320] intent_fulfilled" \
  "event[321] task_verified" \
  "event[322] intent_fulfilled" \
  "event[323] runtime_admission_released" \
  "event[324] runtime_admission_released" \
  "event[325] runtime_admission_admitted" \
  "event[326] task_queued" \
  "event[327] runtime_admission_admitted" \
  "event[328] task_queued" \
  "event[329] task_dispatched" \
  "event[330] task_quantum_expired" \
  "event[331] task_dispatched" \
  "event[332] task_quantum_expired" \
  "event[333] task_dispatched" \
  "event[334] task_result_submitted" \
  "event[335] message_sent" \
  "event[336] message_wait_woken" \
  "event[337] task_completed" \
  "event[338] task_dispatched" \
  "event[339] task_result_submitted" \
  "event[340] message_sent" \
  "event[341] task_completed" \
  "event[342] task_dispatched" \
  "event[343] message_received" \
  "event[344] message_acknowledged" \
  "event[345] message_retired" \
  "event[346] message_received" \
  "event[347] message_acknowledged" \
  "event[348] message_retired" \
  "event[349] waiter_compacted" \
  "event[350] runtime_admission_compacted" \
  "event[351] runtime_admission_compacted" \
  "event[352] task_compacted" \
  "event[353] task_compacted" \
  "event[354] task_compacted" \
  "event[355] task_compacted" \
  "event[356] task_compacted" \
  "event[357] task_compacted" \
  "event[358] fault_compacted" \
  "event[359] fault_compacted" \
  "event[360] fault_compacted" \
  "event[361] fault_compacted" \
  "event[362] intent_compacted" \
  "event[363] intent_compacted" \
  "event[364] intent_compacted" \
  "event[365] intent_compacted" \
  "event[366] intent_compacted" \
  "event[367] intent_compacted" \
  "event[368] agent_entry_retired" \
  "event[369] agent_entry_retired" \
  "event[370] capability_derived" \
  "event[371] capability_revoked" \
  "event[372] capability_compacted" \
  "event[373] capability_derived" \
  "event[374] capability_revoked" \
  "event[375] capability_compacted" \
  "event[376] capability_revoked" \
  "event[377] resource_created" \
  "event[378] capability_granted" \
  "event[379] capability_derived" \
  "event[380] capability_derived" \
  "event[381] memory_cell_record_retired" \
  "event[382] capability_revoked" \
  "event[383] capability_compacted" \
  "event[384] resource_record_retired" \
  "event[385] resource_created" \
  "event[386] capability_granted" \
  "event[387] memory_cell_created" \
  "event[388] task_result_submitted" \
  "event[389] resource_retired" \
  "event[390] task_completed" \
  "event[391] task_verified" \
  "event[392] intent_fulfilled" \
  "event[393] task_verified" \
  "event[394] intent_fulfilled" \
  "event[395] task_verified" \
  "event[396] intent_fulfilled" \
  "event[397] runtime_admission_released" \
  "event[398] runtime_admission_released" \
  "event[399] agent_image_registered" \
  "event[400] device_event_raised" \
  "event[401] device_event_delivered" \
  "event[402] driver_invocation_queued" \
  "event[403] driver_invocation_dispatched" \
  "event[404] driver_invocation_ticked" \
  "event[405] device_event_acknowledged" \
  "event[406] driver_command_submitted" \
  "event[407] driver_command_dispatched" \
  "event[408] driver_command_completed" \
  "event[409] driver_invocation_completed" \
  "SUPERVISOR_HANDOFF_READY"
do
  if ! grep -Fq "$expected" <<<"$OUTPUT"; then
    printf 'missing expected QEMU output: %s\n' "$expected" >&2
    exit 1
  fi
done

EVENT_COUNT="$(grep -Ec '^event\[[0-9]+\] ' <<<"$OUTPUT")"
if [[ "$EVENT_COUNT" -ne 409 ]]; then
  printf 'expected exactly 409 kernel events, observed %s\n' "$EVENT_COUNT" >&2
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

if [[ "$EXPECTED_EVENT_SEQUENCE" -ne 410 ]]; then
  printf 'ordered kernel event sequence ended at %s\n' \
    "$((EXPECTED_EVENT_SEQUENCE - 1))" >&2
  exit 1
fi

check_marker_count() {
  local marker="$1"
  local expected_count="$2"
  MARKER_COUNT="$(grep -Fxc "$marker" <<<"$OUTPUT" || true)"
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
check_marker_count "AGENT_KERNEL_AGENT_CALL_CAPABILITY_COMPACTION_OK" 3
check_marker_count "AGENT_KERNEL_AGENT_CALL_RESOURCE_RECORD_RETIREMENT_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_MEMORY_CELL_RECORD_RETIREMENT_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_NAMESPACE_BIND_OK" 4
check_marker_count "AGENT_KERNEL_AGENT_CALL_NAMESPACE_PATH_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_NAMESPACE_MEMORY_PATH_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_NAMESPACE_TYPED_REBIND_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_NAMESPACE_COMPARE_REBIND_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_NAMESPACE_COMPARE_RETIRE_OK" 0
check_marker_count "AGENT_KERNEL_NATIVE_NAMESPACE_MANAGER_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_NAMESPACE_HIERARCHY_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_NAMESPACE_MEMORY_TRANSPORT_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_TYPED_NAMESPACE_REBIND_OK" 1
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
