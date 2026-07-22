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
  "AGENT_KERNEL_NATIVE_SIGNED_PACKAGE_OK" \
  "AGENT_KERNEL_NATIVE_TRUSTED_SIGNER_OK" \
  "AGENT_KERNEL_NATIVE_TRUST_STORE_OK" \
  "AGENT_KERNEL_AGENT_CALL_SIGNER_ROTATION_OK" \
  "AGENT_KERNEL_NATIVE_SIGNER_ROTATION_OK" \
  "AGENT_KERNEL_NATIVE_REVOKED_SIGNER_REJECTED_OK" \
  "AGENT_KERNEL_NATIVE_ROTATED_SIGNER_ADMISSION_OK" \
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
  "AGENT_KERNEL_NATIVE_SEGMENTED_PACKAGE_OK" \
  "AGENT_KERNEL_NATIVE_RODATA_NX_OK" \
  "AGENT_KERNEL_NATIVE_RELOCATION_OK" \
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
  "event[9] agent_image_signer_trusted" \
  "event[10] driver_endpoint_registered" \
  "event[11] agent_registered" \
  "event[12] capability_derived" \
  "event[13] agent_image_registered" \
  "event[14] agent_image_verified" \
  "event[15] agent_launched" \
  "event[16] driver_bound" \
  "event[17] agent_registered" \
  "event[18] intent_declared" \
  "event[19] task_created" \
  "event[20] intent_bound" \
  "event[21] capability_derived" \
  "event[22] delegation" \
  "event[23] agent_registered" \
  "event[24] intent_declared" \
  "event[25] task_created" \
  "event[26] intent_bound" \
  "event[27] capability_derived" \
  "event[28] delegation" \
  "event[29] agent_image_registered" \
  "event[30] agent_image_verified" \
  "event[31] agent_image_registered" \
  "event[32] agent_image_verified" \
  "event[33] agent_launched" \
  "event[34] task_accepted" \
  "event[35] task_queued" \
  "event[36] agent_launched" \
  "event[37] task_accepted" \
  "event[38] task_queued" \
  "event[39] agent_registered" \
  "event[40] intent_declared" \
  "event[41] task_created" \
  "event[42] intent_bound" \
  "event[43] capability_derived" \
  "event[44] delegation" \
  "event[45] capability_derived" \
  "event[46] agent_image_registered" \
  "event[47] agent_image_verified" \
  "event[48] agent_launched" \
  "event[49] task_accepted" \
  "event[50] agent_registered" \
  "event[51] resource_created" \
  "event[52] capability_granted" \
  "event[53] capability_derived" \
  "event[54] intent_declared" \
  "event[55] task_created" \
  "event[56] intent_bound" \
  "event[57] capability_derived" \
  "event[58] delegation" \
  "event[59] agent_image_registered" \
  "event[60] agent_image_verified" \
  "event[61] agent_launched" \
  "event[62] task_accepted" \
  "event[63] agent_registered" \
  "event[64] intent_declared" \
  "event[65] task_created" \
  "event[66] intent_bound" \
  "event[67] capability_derived" \
  "event[68] delegation" \
  "event[69] agent_image_registered" \
  "event[70] agent_image_verified" \
  "event[71] agent_launched" \
  "event[72] task_accepted" \
  "event[73] fault_handler_installed" \
  "event[74] fault_policy_installed" \
  "event[75] agent_registered" \
  "event[76] intent_declared" \
  "event[77] task_created" \
  "event[78] intent_bound" \
  "event[79] capability_derived" \
  "event[80] delegation" \
  "event[81] capability_derived" \
  "event[82] agent_image_registered" \
  "event[83] agent_image_verified" \
  "event[84] agent_image_registered" \
  "event[85] agent_image_retired" \
  "event[86] agent_launched" \
  "event[87] task_accepted" \
  "event[88] task_dispatched" \
  "event[89] task_quantum_expired" \
  "event[90] task_dispatched" \
  "event[91] task_quantum_expired" \
  "event[92] task_dispatched" \
  "event[93] message_wait_started" \
  "event[94] task_dispatched" \
  "event[95] task_result_submitted" \
  "event[96] message_sent" \
  "event[97] message_wait_woken" \
  "event[98] task_quantum_expired" \
  "event[99] task_dispatched" \
  "event[100] message_received" \
  "event[101] message_acknowledged" \
  "event[102] task_result_submitted" \
  "event[103] task_completed" \
  "event[104] task_dispatched" \
  "event[105] task_yielded" \
  "event[106] task_dispatched" \
  "event[107] task_completed" \
  "event[108] task_queued" \
  "event[109] task_dispatched" \
  "event[110] task_quantum_expired" \
  "event[111] task_dispatched" \
  "event[112] message_wait_started" \
  "event[113] task_queued" \
  "event[114] task_queued" \
  "event[115] task_dispatched" \
  "event[116] task_quantum_expired" \
  "event[117] task_dispatched" \
  "event[118] task_quantum_expired" \
  "event[119] task_dispatched" \
  "event[120] memory_cell_created" \
  "event[121] resource_retired" \
  "event[122] task_faulted" \
  "event[123] task_dispatched" \
  "event[124] task_result_inspected" \
  "event[125] task_verified" \
  "event[126] intent_fulfilled" \
  "event[127] task_completed" \
  "event[128] task_fault_recovered" \
  "event[129] task_queued" \
  "event[130] task_dispatched" \
  "event[131] task_quantum_expired" \
  "event[132] task_dispatched" \
  "event[133] task_faulted" \
  "event[134] task_fault_recovered" \
  "event[135] task_queued" \
  "event[136] task_dispatched" \
  "event[137] task_quantum_expired" \
  "event[138] task_dispatched" \
  "event[139] task_faulted" \
  "event[140] task_fault_recovered" \
  "event[141] task_queued" \
  "event[142] task_dispatched" \
  "event[143] task_quantum_expired" \
  "event[144] task_dispatched" \
  "event[145] task_faulted" \
  "event[146] message_sent" \
  "event[147] message_wait_woken" \
  "event[148] fault_routed" \
  "event[149] fault_policy_applied" \
  "event[150] task_dispatched" \
  "event[151] message_received" \
  "event[152] message_acknowledged" \
  "event[153] task_result_submitted" \
  "event[154] task_completed" \
  "event[155] task_fault_recovered" \
  "event[156] task_queued" \
  "event[157] task_dispatched" \
  "event[158] task_completed" \
  "event[159] task_queued" \
  "event[160] task_dispatched" \
  "event[161] task_quantum_expired" \
  "event[162] task_dispatched" \
  "event[163] resource_created" \
  "event[164] capability_granted" \
  "event[165] capability_derived" \
  "event[166] capability_revoked" \
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
  "event[187] namespace_entry_bound" \
  "event[188] namespace_entry_resolved" \
  "event[189] namespace_entry_resolved" \
  "event[190] resource_created" \
  "event[191] capability_granted" \
  "event[192] memory_cell_created" \
  "event[193] memory_cell_recalled" \
  "event[194] resource_created" \
  "event[195] capability_granted" \
  "event[196] memory_cell_created" \
  "event[197] resource_retired" \
  "event[198] memory_cell_recalled" \
  "event[199] resource_created" \
  "event[200] capability_granted" \
  "event[201] memory_cell_created" \
  "event[202] memory_cell_recalled" \
  "event[203] resource_retired" \
  "event[204] resource_created" \
  "event[205] capability_granted" \
  "event[206] resource_created" \
  "event[207] capability_granted" \
  "event[208] namespace_entry_rebound" \
  "event[209] namespace_entry_bound" \
  "event[210] namespace_entry_bound" \
  "event[211] namespace_entry_resolved" \
  "event[212] namespace_entry_resolved" \
  "event[213] namespace_entry_resolved" \
  "event[214] namespace_entry_resolved" \
  "event[215] namespace_entry_resolved" \
  "event[216] namespace_entry_resolved" \
  "event[217] namespace_entry_resolved" \
  "event[218] namespace_entry_rebound" \
  "event[219] agent_image_signer_trusted" \
  "event[220] agent_image_signer_revoked" \
  "event[221] task_result_submitted" \
  "event[222] resource_retired" \
  "event[223] task_completed" \
  "event[224] task_verified" \
  "event[225] intent_fulfilled" \
  "event[226] task_verified" \
  "event[227] intent_fulfilled" \
  "event[228] task_verified" \
  "event[229] intent_fulfilled" \
  "event[230] task_verified" \
  "event[231] intent_fulfilled" \
  "event[232] task_verified" \
  "event[233] intent_fulfilled" \
  "event[234] agent_registered" \
  "event[235] intent_declared" \
  "event[236] task_created" \
  "event[237] intent_bound" \
  "event[238] capability_derived" \
  "event[239] delegation" \
  "event[240] agent_image_registered" \
  "event[241] agent_image_verified" \
  "event[242] agent_launched" \
  "event[243] task_accepted" \
  "event[244] agent_registered" \
  "event[245] intent_declared" \
  "event[246] task_created" \
  "event[247] intent_bound" \
  "event[248] capability_derived" \
  "event[249] delegation" \
  "event[250] agent_image_registered" \
  "event[251] agent_image_verified" \
  "event[252] agent_launched" \
  "event[253] task_accepted" \
  "event[254] agent_registered" \
  "event[255] intent_declared" \
  "event[256] task_created" \
  "event[257] intent_bound" \
  "event[258] capability_derived" \
  "event[259] delegation" \
  "event[260] capability_derived" \
  "event[261] agent_image_registered" \
  "event[262] agent_image_verified" \
  "event[263] agent_launched" \
  "event[264] task_accepted" \
  "event[265] task_queued" \
  "event[266] agent_registered" \
  "event[267] intent_declared" \
  "event[268] task_created" \
  "event[269] intent_bound" \
  "event[270] capability_derived" \
  "event[271] delegation" \
  "event[272] agent_image_registered" \
  "event[273] agent_image_verified" \
  "event[274] agent_launched" \
  "event[275] task_accepted" \
  "event[276] agent_registered" \
  "event[277] intent_declared" \
  "event[278] task_created" \
  "event[279] intent_bound" \
  "event[280] capability_derived" \
  "event[281] delegation" \
  "event[282] agent_image_registered" \
  "event[283] agent_image_verified" \
  "event[284] agent_launched" \
  "event[285] task_accepted" \
  "event[286] task_dispatched" \
  "event[287] task_quantum_expired" \
  "event[288] task_dispatched" \
  "event[289] runtime_admission_requested" \
  "event[290] runtime_admission_requested" \
  "event[291] message_wait_started" \
  "event[292] runtime_admission_admitted" \
  "event[293] task_queued" \
  "event[294] runtime_admission_admitted" \
  "event[295] task_queued" \
  "event[296] task_dispatched" \
  "event[297] task_quantum_expired" \
  "event[298] task_dispatched" \
  "event[299] task_quantum_expired" \
  "event[300] task_dispatched" \
  "event[301] task_result_submitted" \
  "event[302] message_sent" \
  "event[303] message_wait_woken" \
  "event[304] task_completed" \
  "event[305] task_dispatched" \
  "event[306] task_result_submitted" \
  "event[307] message_sent" \
  "event[308] task_completed" \
  "event[309] task_dispatched" \
  "event[310] message_received" \
  "event[311] message_acknowledged" \
  "event[312] message_retired" \
  "event[313] message_received" \
  "event[314] message_acknowledged" \
  "event[315] message_retired" \
  "event[316] waiter_compacted" \
  "event[317] waiter_compacted" \
  "event[318] waiter_compacted" \
  "event[319] runtime_admission_requested" \
  "event[320] runtime_admission_requested" \
  "event[321] message_wait_started" \
  "event[322] task_verified" \
  "event[323] intent_fulfilled" \
  "event[324] task_verified" \
  "event[325] intent_fulfilled" \
  "event[326] runtime_admission_released" \
  "event[327] runtime_admission_released" \
  "event[328] runtime_admission_admitted" \
  "event[329] task_queued" \
  "event[330] runtime_admission_admitted" \
  "event[331] task_queued" \
  "event[332] task_dispatched" \
  "event[333] task_quantum_expired" \
  "event[334] task_dispatched" \
  "event[335] task_quantum_expired" \
  "event[336] task_dispatched" \
  "event[337] task_result_submitted" \
  "event[338] message_sent" \
  "event[339] message_wait_woken" \
  "event[340] task_completed" \
  "event[341] task_dispatched" \
  "event[342] task_result_submitted" \
  "event[343] message_sent" \
  "event[344] task_completed" \
  "event[345] task_dispatched" \
  "event[346] message_received" \
  "event[347] message_acknowledged" \
  "event[348] message_retired" \
  "event[349] message_received" \
  "event[350] message_acknowledged" \
  "event[351] message_retired" \
  "event[352] waiter_compacted" \
  "event[353] runtime_admission_compacted" \
  "event[354] runtime_admission_compacted" \
  "event[355] task_compacted" \
  "event[356] task_compacted" \
  "event[357] task_compacted" \
  "event[358] task_compacted" \
  "event[359] task_compacted" \
  "event[360] task_compacted" \
  "event[361] fault_compacted" \
  "event[362] fault_compacted" \
  "event[363] fault_compacted" \
  "event[364] fault_compacted" \
  "event[365] intent_compacted" \
  "event[366] intent_compacted" \
  "event[367] intent_compacted" \
  "event[368] intent_compacted" \
  "event[369] intent_compacted" \
  "event[370] intent_compacted" \
  "event[371] agent_entry_retired" \
  "event[372] agent_entry_retired" \
  "event[373] capability_derived" \
  "event[374] capability_revoked" \
  "event[375] capability_compacted" \
  "event[376] capability_derived" \
  "event[377] capability_revoked" \
  "event[378] capability_compacted" \
  "event[379] capability_revoked" \
  "event[380] resource_created" \
  "event[381] capability_granted" \
  "event[382] capability_derived" \
  "event[383] capability_derived" \
  "event[384] memory_cell_record_retired" \
  "event[385] capability_revoked" \
  "event[386] capability_compacted" \
  "event[387] resource_record_retired" \
  "event[388] resource_created" \
  "event[389] capability_granted" \
  "event[390] memory_cell_created" \
  "event[391] task_result_submitted" \
  "event[392] resource_retired" \
  "event[393] task_completed" \
  "event[394] task_verified" \
  "event[395] intent_fulfilled" \
  "event[396] task_verified" \
  "event[397] intent_fulfilled" \
  "event[398] task_verified" \
  "event[399] intent_fulfilled" \
  "event[400] runtime_admission_released" \
  "event[401] runtime_admission_released" \
  "event[402] agent_image_registered" \
  "event[403] device_event_raised" \
  "event[404] device_event_delivered" \
  "event[405] driver_invocation_queued" \
  "event[406] driver_invocation_dispatched" \
  "event[407] driver_invocation_ticked" \
  "event[408] device_event_acknowledged" \
  "event[409] driver_command_submitted" \
  "event[410] driver_command_dispatched" \
  "event[411] driver_command_completed" \
  "event[412] driver_invocation_completed" \
  "SUPERVISOR_HANDOFF_READY"
do
  if ! grep -Fq "$expected" <<<"$OUTPUT"; then
    printf 'missing expected QEMU output: %s\n' "$expected" >&2
    exit 1
  fi
done

EVENT_COUNT="$(grep -Ec '^event\[[0-9]+\] ' <<<"$OUTPUT")"
if [[ "$EVENT_COUNT" -ne 412 ]]; then
  printf 'expected exactly 412 kernel events, observed %s\n' "$EVENT_COUNT" >&2
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

if [[ "$EXPECTED_EVENT_SEQUENCE" -ne 413 ]]; then
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
check_marker_count "AGENT_KERNEL_NATIVE_TRUST_STORE_OK" 1
check_marker_count "AGENT_KERNEL_AGENT_CALL_SIGNER_ROTATION_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_SIGNER_ROTATION_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_REVOKED_SIGNER_REJECTED_OK" 1
check_marker_count "AGENT_KERNEL_NATIVE_ROTATED_SIGNER_ADMISSION_OK" 1
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
