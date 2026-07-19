use agent_kernel::AgentKernel;

type DefaultKernel = AgentKernel<3, 2, 8, 40, 0, 0, 0, 2, 2, 2>;
type CapacityKernel =
    AgentKernel<3, 2, 8, 40, 0, 0, 0, 2, 2, 2, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 5>;

#[test]
fn facade_forwards_default_and_explicit_runtime_admission_capacities() {
    assert_eq!(DefaultKernel::new().runtime_admission_capacity(), 2);
    assert_eq!(CapacityKernel::new().runtime_admission_capacity(), 5);
}
