use std::process::Command;

#[test]
fn supervisor_runs_minimal_agent_kernel_flow() {
    let output = Command::new(env!("CARGO_BIN_EXE_agent-supervisor"))
        .output()
        .expect("supervisor binary should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Agent Kernel supervisor boot"));
    assert!(stdout.contains("event[1] observation agent=1 resource=1"));
    assert!(stdout.contains("event[2] action agent=1 resource=1 action=1"));
    assert!(stdout.contains("event[3] verification agent=1 resource=1 action=1"));
    assert!(stdout.contains("event[4] checkpoint agent=1 resource=1 checkpoint=1"));
    assert!(stdout.contains("event[5] rollback agent=1 resource=1 checkpoint=1"));
}
