//! CPU boot lifecycle and online membership state.
//!
//! The registry owns deterministic transitions after firmware topology freezes.
//! Startup generations prevent stale AP handshakes, while quiesce checks keep
//! run ownership and active TLB targets away from CPUs leaving the online set.

use super::{CpuIndex, CpuMask, CpuTopology, MAX_CPU_COUNT};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CpuLifecycleState {
    Discovered,
    StartupRequested,
    Online,
    Failed,
    Quiescing,
    Offline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CpuRegistryError {
    UnknownCpu(CpuIndex),
    InvalidStartupGeneration,
    StaleStartupGeneration {
        expected: u64,
        actual: u64,
    },
    InvalidState {
        cpu: CpuIndex,
        state: CpuLifecycleState,
    },
    CpuOwnsRunContext(CpuIndex),
    CpuTargetsActiveShootdown(CpuIndex),
    BootProcessorCannotOffline,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CpuRegistry<const CAPACITY: usize = MAX_CPU_COUNT> {
    topology: CpuTopology<CAPACITY>,
    states: [CpuLifecycleState; CAPACITY],
    startup_generations: [u64; CAPACITY],
    online: CpuMask,
}

impl<const CAPACITY: usize> CpuRegistry<CAPACITY> {
    pub fn new(topology: CpuTopology<CAPACITY>) -> Self {
        let mut states = [CpuLifecycleState::Offline; CAPACITY];
        for (index, state) in states.iter_mut().enumerate().take(topology.len()) {
            *state = if index == 0 {
                CpuLifecycleState::Online
            } else {
                CpuLifecycleState::Discovered
            };
        }
        Self {
            topology,
            states,
            startup_generations: [0; CAPACITY],
            online: CpuMask::singleton(CpuIndex::BSP),
        }
    }

    pub const fn topology(&self) -> &CpuTopology<CAPACITY> {
        &self.topology
    }

    pub fn state(&self, cpu: CpuIndex) -> Option<CpuLifecycleState> {
        self.slot(cpu).map(|slot| self.states[slot])
    }

    pub fn startup_generation(&self, cpu: CpuIndex) -> Option<u64> {
        self.slot(cpu).map(|slot| self.startup_generations[slot])
    }

    pub const fn online_mask(&self) -> CpuMask {
        self.online
    }

    pub fn request_startup(
        &mut self,
        cpu: CpuIndex,
        generation: u64,
    ) -> Result<(), CpuRegistryError> {
        if generation == 0 {
            return Err(CpuRegistryError::InvalidStartupGeneration);
        }
        let slot = self.slot(cpu).ok_or(CpuRegistryError::UnknownCpu(cpu))?;
        self.require_state(cpu, slot, CpuLifecycleState::Discovered)?;
        self.startup_generations[slot] = generation;
        self.states[slot] = CpuLifecycleState::StartupRequested;
        Ok(())
    }

    pub fn acknowledge_online(
        &mut self,
        cpu: CpuIndex,
        generation: u64,
    ) -> Result<(), CpuRegistryError> {
        let slot = self.slot(cpu).ok_or(CpuRegistryError::UnknownCpu(cpu))?;
        self.require_state(cpu, slot, CpuLifecycleState::StartupRequested)?;
        self.require_generation(slot, generation)?;
        self.states[slot] = CpuLifecycleState::Online;
        self.online.insert(cpu);
        Ok(())
    }

    pub fn fail_startup(&mut self, cpu: CpuIndex, generation: u64) -> Result<(), CpuRegistryError> {
        let slot = self.slot(cpu).ok_or(CpuRegistryError::UnknownCpu(cpu))?;
        self.require_state(cpu, slot, CpuLifecycleState::StartupRequested)?;
        self.require_generation(slot, generation)?;
        self.states[slot] = CpuLifecycleState::Failed;
        Ok(())
    }

    pub fn begin_quiesce(
        &mut self,
        cpu: CpuIndex,
        run_owners: CpuMask,
        shootdown_targets: CpuMask,
    ) -> Result<(), CpuRegistryError> {
        if cpu == CpuIndex::BSP {
            return Err(CpuRegistryError::BootProcessorCannotOffline);
        }
        let slot = self.slot(cpu).ok_or(CpuRegistryError::UnknownCpu(cpu))?;
        self.require_state(cpu, slot, CpuLifecycleState::Online)?;
        if run_owners.contains(cpu) {
            return Err(CpuRegistryError::CpuOwnsRunContext(cpu));
        }
        if shootdown_targets.contains(cpu) {
            return Err(CpuRegistryError::CpuTargetsActiveShootdown(cpu));
        }
        self.states[slot] = CpuLifecycleState::Quiescing;
        self.online.remove(cpu);
        Ok(())
    }

    pub fn mark_offline(&mut self, cpu: CpuIndex) -> Result<(), CpuRegistryError> {
        if cpu == CpuIndex::BSP {
            return Err(CpuRegistryError::BootProcessorCannotOffline);
        }
        let slot = self.slot(cpu).ok_or(CpuRegistryError::UnknownCpu(cpu))?;
        self.require_state(cpu, slot, CpuLifecycleState::Quiescing)?;
        self.states[slot] = CpuLifecycleState::Offline;
        Ok(())
    }

    fn slot(&self, cpu: CpuIndex) -> Option<usize> {
        self.topology.get(cpu).map(|_| cpu.as_usize())
    }

    fn require_state(
        &self,
        cpu: CpuIndex,
        slot: usize,
        expected: CpuLifecycleState,
    ) -> Result<(), CpuRegistryError> {
        let state = self.states[slot];
        if state == expected {
            Ok(())
        } else {
            Err(CpuRegistryError::InvalidState { cpu, state })
        }
    }

    fn require_generation(&self, slot: usize, actual: u64) -> Result<(), CpuRegistryError> {
        let expected = self.startup_generations[slot];
        if expected == actual {
            Ok(())
        } else {
            Err(CpuRegistryError::StaleStartupGeneration { expected, actual })
        }
    }
}
