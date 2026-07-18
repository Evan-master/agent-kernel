//! Read-only cleanup planning for live Agent runtime mappings.
//!
//! This bare-metal Agent-memory child snapshots the private compatibility-page
//! and region ledgers into architecture-library identity. The fault executor
//! owns authorization, semantic retirement, leaf removal, and frame release.

use agent_kernel_x86_64::runtime_reclamation::RuntimeReclamationPlan;

use super::PreparedAgentMemory;

impl PreparedAgentMemory {
    pub(crate) fn runtime_reclamation_plan(&self) -> Option<RuntimeReclamationPlan> {
        RuntimeReclamationPlan::new(self.runtime_page.binding(), self.runtime_regions.bindings())
    }
}
