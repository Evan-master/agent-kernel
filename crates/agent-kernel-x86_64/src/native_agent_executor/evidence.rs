//! Exact phase counters for the deterministic native execution profile.
//!
//! This executor child owns read-only predicates over bounded runtime counters.
//! Changes must stay synchronized with the strict QEMU event contract.

use super::NativeRuntimeEvidence;

impl NativeRuntimeEvidence {
    pub(crate) const fn proves_runtime_admission_wait(self) -> bool {
        self.dispatches == 2
            && self.prepared == 1
            && self.preempted == 1
            && self.waiting == 0
            && self.yielded == 0
            && self.recovered_faults == 0
            && self.quantum_expiries == 1
            && self.returning_quantum_expiries == 0
            && self.returning_quantum_generation == 0
            && self.agent_faults == 0
    }

    pub(crate) const fn proves_resident_runtime_admission_flow(self) -> bool {
        self.dispatches == 7
            && self.prepared == 3
            && self.preempted == 3
            && self.waiting == 1
            && self.yielded == 0
            && self.recovered_faults == 0
            && self.quantum_expiries == 3
            && self.returning_quantum_expiries == 0
            && self.returning_quantum_generation == 0
            && self.agent_faults == 0
    }

    pub(crate) const fn proves_fault_containment_phase(self) -> bool {
        self.dispatches == 13
            && self.prepared == 5
            && self.preempted == 6
            && self.waiting == 1
            && self.yielded == 1
            && self.recovered_faults == 0
            && self.quantum_expiries == 6
            && self.returning_quantum_expiries == 1
            && self.returning_quantum_generation == 2
            && self.agent_faults == 1
    }

    pub(crate) const fn proves_current_boot(self) -> bool {
        self.dispatches == 21
            && self.prepared == 8
            && self.preempted == 9
            && self.waiting == 2
            && self.yielded == 1
            && self.recovered_faults == 1
            && self.quantum_expiries == 9
            && self.returning_quantum_expiries == 1
            && self.returning_quantum_generation == 2
            && self.agent_faults == 4
    }

    pub(crate) const fn proves_resource_manager_phase(self) -> bool {
        self.dispatches == 23
            && self.prepared == 9
            && self.preempted == 10
            && self.waiting == 2
            && self.yielded == 1
            && self.recovered_faults == 1
            && self.quantum_expiries == 10
            && self.returning_quantum_expiries == 1
            && self.returning_quantum_generation == 2
            && self.agent_faults == 4
    }

    pub(crate) const fn proves_general_protection_phase(self) -> bool {
        self.dispatches == 15
            && self.prepared == 6
            && self.preempted == 7
            && self.waiting == 1
            && self.yielded == 1
            && self.recovered_faults == 0
            && self.quantum_expiries == 7
            && self.returning_quantum_expiries == 1
            && self.returning_quantum_generation == 2
            && self.agent_faults == 2
    }

    pub(crate) const fn proves_page_fault_phase(self) -> bool {
        self.dispatches == 17
            && self.prepared == 7
            && self.preempted == 8
            && self.waiting == 1
            && self.yielded == 1
            && self.recovered_faults == 0
            && self.quantum_expiries == 8
            && self.returning_quantum_expiries == 1
            && self.returning_quantum_generation == 2
            && self.agent_faults == 3
    }

    pub(crate) const fn proves_lazy_page_fault_phase(self) -> bool {
        self.dispatches == 19
            && self.prepared == 8
            && self.preempted == 9
            && self.waiting == 1
            && self.yielded == 1
            && self.recovered_faults == 0
            && self.quantum_expiries == 9
            && self.returning_quantum_expiries == 1
            && self.returning_quantum_generation == 2
            && self.agent_faults == 4
    }

    pub(crate) const fn proves_fault_handler_decision_phase(self) -> bool {
        self.dispatches == 20
            && self.prepared == 8
            && self.preempted == 9
            && self.waiting == 2
            && self.yielded == 1
            && self.recovered_faults == 0
            && self.quantum_expiries == 9
            && self.returning_quantum_expiries == 1
            && self.returning_quantum_generation == 2
            && self.agent_faults == 4
    }
}
