use agent_kernel::AgentKernel;
use agent_kernel_core::{AgentId, AgentStatus, EventKind, KernelError};

type TestKernel = AgentKernel<1, 1, 1, 8, 0, 0, 0, 0, 0, 0>;

#[test]
fn agent_lifecycle_syscalls_update_agent_status_and_events() {
    let mut kernel = TestKernel::new();
    let agent = AgentId::new(1);
    kernel
        .sys_register_agent(agent)
        .expect("agent should register");

    let suspend = kernel
        .sys_suspend_agent(agent)
        .expect("agent should suspend");
    let resume = kernel.sys_resume_agent(agent).expect("agent should resume");
    let retire = kernel.sys_retire_agent(agent).expect("agent should retire");

    assert_eq!(kernel.agents()[0].status, AgentStatus::Retired);
    assert_eq!(suspend.kind, EventKind::AgentSuspended);
    assert_eq!(resume.kind, EventKind::AgentResumed);
    assert_eq!(retire.kind, EventKind::AgentRetired);
    assert_eq!(retire.target_agent, Some(agent));
    assert_eq!(kernel.events().len(), 4);
    assert_eq!(
        kernel.sys_resume_agent(agent),
        Err(KernelError::AgentRetired)
    );
    assert_eq!(kernel.events().len(), 4);
}
