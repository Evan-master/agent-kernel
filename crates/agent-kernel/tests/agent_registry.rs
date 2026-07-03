use agent_kernel::AgentKernel;
use agent_kernel_core::{AgentId, AgentStatus, EventKind};

type TestKernel = AgentKernel<2, 1, 1, 4, 0, 0, 0, 0, 0, 0>;

#[test]
fn sys_register_agent_records_and_exposes_agent() {
    let mut kernel = TestKernel::new();
    let agent = AgentId::new(1);

    let event = kernel
        .sys_register_agent(agent)
        .expect("agent registration should fit");

    assert_eq!(kernel.agents().len(), 1);
    assert_eq!(kernel.agents()[0].id, agent);
    assert_eq!(kernel.agents()[0].status, AgentStatus::Active);
    assert_eq!(event.kind, EventKind::AgentRegistered);
    assert_eq!(event.target_agent, Some(agent));
}
