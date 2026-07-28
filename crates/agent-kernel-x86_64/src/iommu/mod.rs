//! Native x86_64 DMA remapping primitives.
//!
//! This architecture layer owns Intel VT-d table encodings and bounded MMIO
//! command execution. Agent authority and mapping lifecycle remain in
//! `agent-kernel-core`.

mod intel_vtd;
mod tables;

pub use intel_vtd::{
    IntelVtd, VolatileVtdMmio, VtdControllerError, VtdFaultRecord, VtdOperation, VtdRegisterIo,
    DMAR_CAP_REG, DMAR_CCMD_REG, DMAR_ECAP_REG, DMAR_FRCD_HIGH_REG, DMAR_FRCD_LOW_REG,
    DMAR_FSTS_REG, DMAR_GCMD_REG, DMAR_GSTS_REG, DMAR_IOTLB_REG, DMAR_RTADDR_REG, DMAR_VER_REG,
};
pub use tables::{
    VtdDomainId, VtdLegacyTableAddresses, VtdLegacyTablePages, VtdTableError, VTD_ADDRESS_WIDTH,
};
