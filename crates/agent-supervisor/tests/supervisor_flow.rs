use std::process::Command;

#[test]
fn supervisor_runs_minimal_agent_kernel_flow() {
    let output = Command::new(env!("CARGO_BIN_EXE_agent-supervisor"))
        .output()
        .expect("supervisor binary should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Agent Kernel supervisor boot"));
    assert!(stdout.contains("event[1] agent_registered agent=1 target_agent=1"));
    assert!(stdout.contains("event[2] agent_registered agent=2 target_agent=2"));
    assert!(stdout.contains("event[3] agent_registered agent=3 target_agent=3"));
    assert!(stdout.contains("event[4] capability_granted agent=1 resource=1 capability=1"));
    assert!(stdout.contains("event[5] fault_handler_installed agent=1 resource=1 target_agent=3"));
    assert!(stdout.contains(
        "event[6] fault_policy_installed agent=1 resource=1 policy=1 action=route_to_handler"
    ));
    assert!(stdout.contains("event[7] observation agent=1 resource=1"));
    assert!(stdout.contains("event[8] action agent=1 resource=1 action=1"));
    assert!(stdout.contains("event[9] verification agent=1 resource=1 action=1"));
    assert!(stdout.contains("event[10] checkpoint agent=1 resource=1 checkpoint=1"));
    assert!(stdout.contains("event[11] rollback agent=1 resource=1 checkpoint=1"));
    assert!(stdout.contains("event[12] intent_declared agent=1 resource=1 intent=1"));
    assert!(stdout.contains("event[13] task_created agent=1 resource=1 task=1"));
    assert!(stdout.contains("event[14] intent_bound agent=1 resource=1 intent=1"));
    assert!(stdout.contains("event[15] capability_derived agent=1 resource=1 capability=2"));
    assert!(stdout.contains("event[16] delegation agent=1 resource=1 task=1 target_agent=2"));
    assert!(stdout.contains("event[17] task_accepted agent=2 resource=1 task=1"));
    assert!(stdout.contains("event[18] task_queued agent=2 resource=1 task=1"));
    assert!(stdout.contains("event[19] task_dispatched agent=2 resource=1 task=1"));
    assert!(stdout.contains("event[20] task_ticked agent=2 resource=1 task=1 ticks=1 quantum=1"));
    assert!(stdout
        .contains("event[21] task_quantum_expired agent=2 resource=1 task=1 ticks=2 quantum=0"));
    assert!(stdout.contains("event[22] task_dispatched agent=2 resource=1 task=1"));
    assert!(stdout.contains("event[23] task_faulted agent=2 resource=1 task=1 fault=1 detail=7"));
    assert!(stdout.contains("event[24] message_sent agent=1 target_agent=3 message=1"));
    assert!(stdout
        .contains("event[25] fault_routed agent=1 resource=1 task=1 fault=1 detail=7 target_agent=3 message=1"));
    assert!(stdout
        .contains("event[26] fault_policy_applied agent=1 resource=1 task=1 fault=1 detail=7 policy=1 action=route_to_handler message=1"));
    assert!(stdout.contains("event[27] message_received agent=3 target_agent=1 message=1"));
    assert!(stdout.contains("event[28] message_acknowledged agent=3 target_agent=1 message=1"));
    assert!(stdout
        .contains("event[29] task_fault_recovered agent=1 resource=1 task=1 fault=1 detail=7"));
    assert!(stdout.contains("event[30] task_queued agent=2 resource=1 task=1"));
    assert!(stdout.contains("event[31] task_dispatched agent=2 resource=1 task=1"));
    assert!(stdout.contains("event[32] task_waiting agent=2 resource=1 task=1 waiter=1 signal=1"));
    assert!(stdout.contains(
        "event[33] signal_emitted agent=1 resource=1 task=1 waiter=1 signal=1 target_agent=2"
    ));
    assert!(stdout.contains(
        "event[34] task_woken agent=1 resource=1 task=1 waiter=1 signal=1 target_agent=2"
    ));
    assert!(stdout.contains("event[35] task_dispatched agent=2 resource=1 task=1"));
    assert!(stdout.contains("event[36] task_completed agent=2 resource=1 task=1"));
    assert!(stdout.contains("event[37] task_verified agent=1 resource=1 task=1"));
    assert!(stdout.contains("event[38] intent_fulfilled agent=1 resource=1 intent=1"));
    assert!(stdout.contains("event[39] message_sent agent=1 target_agent=2 message=2"));
    assert!(stdout.contains("event[40] message_received agent=2 target_agent=1 message=2"));
    assert!(stdout.contains("event[41] message_acknowledged agent=2 target_agent=1 message=2"));
    assert!(stdout.contains("event[42] capability_granted agent=1 resource=2 capability=3"));
    assert!(stdout.contains("event[43] memory_cell_created agent=1 resource=2 memory_cell=1"));
    assert!(stdout.contains("event[44] memory_cell_recalled agent=1 resource=2 memory_cell=1"));
    assert!(stdout.contains("event[45] memory_cell_remembered agent=1 resource=2 memory_cell=1"));
    assert!(stdout
        .contains("event[46] namespace_entry_bound agent=1 resource=1 namespace_entry=1 key=1"));
    assert!(stdout
        .contains("event[47] namespace_entry_resolved agent=1 resource=1 namespace_entry=1 key=1"));
    assert!(stdout
        .contains("event[48] namespace_entry_rebound agent=1 resource=1 namespace_entry=1 key=1"));
    assert!(stdout.contains("event[49] resource_created agent=1 resource=3 capability=4"));
    assert!(stdout.contains("event[50] capability_granted agent=1 resource=3 capability=4"));
    assert!(stdout.contains("event[51] resource_retired agent=1 resource=3 capability=4"));
    assert!(stdout.contains("event[52] capability_derived agent=1 resource=1 capability=5"));
    assert!(stdout.contains("event[53] observation agent=2 resource=1"));
}
