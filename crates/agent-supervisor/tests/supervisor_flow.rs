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
    assert!(stdout.contains("event[3] capability_granted agent=1 resource=1 capability=1"));
    assert!(stdout.contains("event[4] observation agent=1 resource=1"));
    assert!(stdout.contains("event[5] action agent=1 resource=1 action=1"));
    assert!(stdout.contains("event[6] verification agent=1 resource=1 action=1"));
    assert!(stdout.contains("event[7] checkpoint agent=1 resource=1 checkpoint=1"));
    assert!(stdout.contains("event[8] rollback agent=1 resource=1 checkpoint=1"));
    assert!(stdout.contains("event[9] intent_declared agent=1 resource=1 intent=1"));
    assert!(stdout.contains("event[10] task_created agent=1 resource=1 task=1"));
    assert!(stdout.contains("event[11] intent_bound agent=1 resource=1 intent=1"));
    assert!(stdout.contains("event[12] capability_derived agent=1 resource=1 capability=2"));
    assert!(stdout.contains("event[13] delegation agent=1 resource=1 task=1 target_agent=2"));
    assert!(stdout.contains("event[14] task_accepted agent=2 resource=1 task=1"));
    assert!(stdout.contains("event[15] task_queued agent=2 resource=1 task=1"));
    assert!(stdout.contains("event[16] task_dispatched agent=2 resource=1 task=1"));
    assert!(stdout.contains("event[17] task_completed agent=2 resource=1 task=1"));
    assert!(stdout.contains("event[18] task_verified agent=1 resource=1 task=1"));
    assert!(stdout.contains("event[19] intent_fulfilled agent=1 resource=1 intent=1"));
    assert!(stdout.contains("event[20] message_sent agent=1 target_agent=2 message=1"));
    assert!(stdout.contains("event[21] message_received agent=2 target_agent=1 message=1"));
    assert!(stdout.contains("event[22] message_acknowledged agent=2 target_agent=1 message=1"));
}
