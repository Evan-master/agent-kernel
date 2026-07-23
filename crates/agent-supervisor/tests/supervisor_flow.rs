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
    assert!(stdout.contains(
        "event[5] agent_image_registered agent=1 resource=1 capability=1 image=1 kind=supervisor"
    ));
    assert!(stdout.contains(
        "event[6] agent_image_verified agent=1 resource=1 capability=1 image=1 kind=supervisor"
    ));
    assert!(stdout.contains("event[7] agent_launched agent=1 resource=1 capability=1 image=1"));
    assert!(stdout.contains("event[8] fault_handler_installed agent=1 resource=1 target_agent=3"));
    assert!(stdout.contains(
        "event[9] fault_policy_installed agent=1 resource=1 policy=1 action=route_to_handler"
    ));
    assert!(stdout.contains("event[10] observation agent=1 resource=1"));
    assert!(stdout.contains("event[11] action agent=1 resource=1 action=1"));
    assert!(stdout.contains("event[12] verification agent=1 resource=1 action=1"));
    assert!(stdout.contains("event[13] checkpoint agent=1 resource=1 checkpoint=1"));
    assert!(stdout.contains("event[14] rollback agent=1 resource=1 checkpoint=1"));
    assert!(stdout.contains("event[15] intent_declared agent=1 resource=1 intent=1"));
    assert!(stdout.contains("event[16] task_created agent=1 resource=1 task=1"));
    assert!(stdout.contains("event[17] intent_bound agent=1 resource=1 intent=1"));
    assert!(stdout.contains("event[18] capability_derived agent=1 resource=1 capability=2"));
    assert!(stdout.contains("event[19] delegation agent=1 resource=1 task=1 target_agent=2"));
    assert!(stdout.contains(
        "event[20] agent_image_registered agent=1 resource=1 capability=1 image=2 kind=worker"
    ));
    assert!(stdout.contains(
        "event[21] agent_image_verified agent=1 resource=1 capability=1 image=2 kind=worker"
    ));
    assert!(
        stdout.contains("event[22] agent_launched agent=2 resource=1 capability=2 image=2 task=1")
    );
    assert!(stdout.contains("event[23] task_accepted agent=2 resource=1 task=1"));
    assert!(stdout.contains("event[24] task_queued agent=2 resource=1 task=1"));
    assert!(stdout.contains("event[25] task_dispatched agent=2 resource=1 task=1"));
    assert!(stdout.contains("event[26] task_ticked agent=2 resource=1 task=1 ticks=1 quantum=1"));
    assert!(stdout
        .contains("event[27] task_quantum_expired agent=2 resource=1 task=1 ticks=2 quantum=0"));
    assert!(stdout.contains("event[28] task_dispatched agent=2 resource=1 task=1"));
    assert!(stdout.contains("event[29] task_faulted agent=2 resource=1 task=1 fault=1 detail=7"));
    assert!(stdout.contains("event[30] message_sent agent=1 target_agent=3 message=1"));
    assert!(stdout
        .contains("event[31] fault_routed agent=1 resource=1 task=1 fault=1 detail=7 target_agent=3 message=1"));
    assert!(stdout
        .contains("event[32] fault_policy_applied agent=1 resource=1 task=1 fault=1 detail=7 policy=1 action=route_to_handler message=1"));
    assert!(stdout.contains("event[33] message_received agent=3 target_agent=1 message=1"));
    assert!(stdout.contains("event[34] message_acknowledged agent=3 target_agent=1 message=1"));
    assert!(stdout
        .contains("event[35] task_fault_recovered agent=1 resource=1 task=1 fault=1 detail=7"));
    assert!(stdout.contains("event[36] task_queued agent=2 resource=1 task=1"));
    assert!(stdout.contains("event[37] task_dispatched agent=2 resource=1 task=1"));
    assert!(stdout.contains("event[38] task_waiting agent=2 resource=1 task=1 waiter=1 signal=1"));
    assert!(stdout.contains(
        "event[39] signal_emitted agent=1 resource=1 task=1 waiter=1 signal=1 target_agent=2"
    ));
    assert!(stdout.contains(
        "event[40] task_woken agent=1 resource=1 task=1 waiter=1 signal=1 target_agent=2"
    ));
    assert!(stdout.contains("event[41] task_dispatched agent=2 resource=1 task=1"));
    assert!(stdout.contains("event[42] task_completed agent=2 resource=1 task=1"));
    assert!(stdout.contains("event[43] task_verified agent=1 resource=1 task=1"));
    assert!(stdout.contains("event[44] intent_fulfilled agent=1 resource=1 intent=1"));
    assert!(stdout.contains("event[45] message_sent agent=1 target_agent=2 message=2"));
    assert!(stdout.contains("event[46] message_received agent=2 target_agent=1 message=2"));
    assert!(stdout.contains("event[47] message_acknowledged agent=2 target_agent=1 message=2"));
    assert!(stdout.contains("event[48] capability_granted agent=1 resource=2 capability=3"));
    assert!(stdout.contains("event[49] memory_cell_created agent=1 resource=2 memory_cell=1"));
    assert!(stdout.contains("event[50] memory_cell_recalled agent=1 resource=2 memory_cell=1"));
    assert!(stdout.contains("event[51] memory_cell_remembered agent=1 resource=2 memory_cell=1"));
    assert!(stdout
        .contains("event[52] namespace_entry_bound agent=1 resource=1 namespace_entry=1 key=1"));
    assert!(stdout
        .contains("event[53] namespace_entry_resolved agent=1 resource=1 namespace_entry=1 key=1"));
    assert!(stdout
        .contains("event[54] namespace_entry_rebound agent=1 resource=1 namespace_entry=1 key=1"));
    assert!(stdout.contains("event[55] resource_created agent=1 resource=3 capability=4"));
    assert!(stdout.contains("event[56] capability_granted agent=1 resource=3 capability=4"));
    assert!(stdout.contains("event[57] resource_retired agent=1 resource=3 capability=4"));
    assert!(stdout.contains("event[58] capability_derived agent=1 resource=1 capability=5"));
    assert!(stdout.contains("event[59] observation agent=2 resource=1"));
    assert!(stdout.contains("event[60] agent_registered agent=4 target_agent=4"));
    assert!(stdout.contains("event[61] resource_created agent=1 resource=4 capability=6"));
    assert!(stdout.contains("event[62] capability_granted agent=1 resource=4 capability=6"));
    assert!(stdout.contains("event[63] driver_endpoint_registered agent=1 resource=4 capability=6"));
    assert!(stdout.contains("event[64] capability_derived agent=1 resource=4 capability=7"));
    assert!(stdout.contains(
        "event[65] agent_image_registered agent=1 resource=4 capability=6 image=3 kind=driver"
    ));
    assert!(stdout.contains(
        "event[66] agent_image_verified agent=1 resource=4 capability=6 image=3 kind=driver"
    ));
    assert!(stdout.contains("event[67] agent_launched agent=4 resource=4 capability=7 image=3"));
    assert!(stdout.contains(
        "event[68] driver_bound agent=1 resource=4 capability=6 driver_binding=1 target_agent=4"
    ));
    assert!(stdout.contains(
        "event[69] device_event_raised agent=1 resource=4 capability=6 driver_binding=1 device_event=1 driver_invocation=0 kind=state_changed code=1 value=2"
    ));
    assert!(stdout.contains(
        "event[70] device_event_delivered agent=4 resource=4 capability=7 driver_binding=1 device_event=1 driver_invocation=1 kind=state_changed code=1 value=2"
    ));
    assert!(stdout.contains(
        "event[71] driver_invocation_queued agent=4 resource=4 capability=7 driver_binding=1 device_event=1 driver_invocation=1 ticks=0 quantum=0"
    ));
    assert!(stdout.contains(
        "event[72] driver_invocation_dispatched agent=4 resource=4 capability=0 driver_binding=1 device_event=1 driver_invocation=1 ticks=0 quantum=2"
    ));
    assert!(stdout.contains(
        "event[73] driver_invocation_ticked agent=4 resource=4 capability=0 driver_binding=1 device_event=1 driver_invocation=1 ticks=1 quantum=1"
    ));
    assert!(stdout.contains(
        "event[74] device_event_acknowledged agent=4 resource=4 capability=7 driver_binding=1 device_event=1 driver_invocation=1 kind=state_changed code=1 value=2"
    ));
    assert!(stdout.contains(
        "event[75] driver_command_submitted agent=4 resource=4 capability=7 driver_binding=1 device_event=1 driver_invocation=1 driver_command=1 kind=write opcode=3 value=11"
    ));
    assert!(stdout.contains(
        "event[76] driver_command_dispatched agent=4 resource=4 capability=7 driver_binding=1 device_event=1 driver_invocation=1 driver_command=1 kind=write opcode=3 value=11"
    ));
    assert!(stdout.contains(
        "event[77] driver_command_completed agent=4 resource=4 capability=7 driver_binding=1 device_event=1 driver_invocation=1 driver_command=1 kind=write opcode=3 value=11 result_code=0 result_value=11"
    ));
    assert!(stdout.contains(
        "event[78] driver_invocation_completed agent=4 resource=4 capability=7 driver_binding=1 device_event=1 driver_invocation=1 ticks=1 quantum=0"
    ));
    assert!(stdout.contains(
        "event_archive_checkpoint generation=1 first=1 through=64 count=64 actor=1 authority=1 root=1 previous_digest=0000000000000000000000000000000000000000000000000000000000000000 digest=b72f0e90513d7ff3b294acb3e925d0544c4a64c4d8ef4d0972fce823449aff0d"
    ));
    assert!(stdout.contains(
        "durable_archive_receipt slot=A storage=1 generation=1 archive_digest=b72f0e90513d7ff3b294acb3e925d0544c4a64c4d8ef4d0972fce823449aff0d manifest_digest=0289ba7e8ccd007a088fc96487f60a6cf57cb21e776bb509a6c0eb782c8673e0 readback_digest=a0e6a8d11ba1cf8075292e82ce27fa24fcc8a03b5c83fbdf496d336bafd6ea06 flush_epoch=3"
    ));
}
