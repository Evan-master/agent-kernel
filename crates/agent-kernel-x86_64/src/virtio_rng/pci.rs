//! Exact modern virtio-rng PCI identity and required capability set.
//!
//! This x86 architecture module selects one Common, Notify, and ISR vendor
//! capability from a previously validated conventional capability list.
//! Missing or duplicate required regions fail before BAR mapping or MMIO.

use crate::pci::{
    PciBar, PciBarIndex, PciBarKind, PciBarSet, PciCapabilityList, PciConfigAccess,
    PciFunctionAddress, PciFunctionSelector, VirtioPciBarRegion, VirtioPciCapability,
    VirtioPciCapabilityError, VirtioPciCapabilityKind, PCI_CAPABILITY_ID_VENDOR_SPECIFIC,
};

use super::{VIRTIO_RNG_DEVICE_ID, VIRTIO_RNG_VENDOR_ID};

pub fn virtio_rng_selector(address: PciFunctionAddress) -> PciFunctionSelector {
    PciFunctionSelector::new(address, VIRTIO_RNG_VENDOR_ID, VIRTIO_RNG_DEVICE_ID)
        .expect("fixed virtio-rng identity")
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VirtioRngPciCapabilities {
    common: VirtioPciCapability,
    notify: VirtioPciCapability,
    isr: VirtioPciCapability,
}

impl VirtioRngPciCapabilities {
    pub fn decode<A: PciConfigAccess, const CAPACITY: usize>(
        access: &mut A,
        address: PciFunctionAddress,
        list: &PciCapabilityList<CAPACITY>,
    ) -> Result<Self, VirtioRngPciCapabilityError> {
        let mut common = None;
        let mut notify = None;
        let mut isr = None;
        for record in list
            .all()
            .iter()
            .copied()
            .filter(|record| record.id() == PCI_CAPABILITY_ID_VENDOR_SPECIFIC)
        {
            let capability =
                VirtioPciCapability::decode(access, address, record).map_err(|error| {
                    VirtioRngPciCapabilityError::Decode {
                        offset: record.offset(),
                        error,
                    }
                })?;
            let (slot, required) = match capability.kind() {
                VirtioPciCapabilityKind::CommonConfiguration => {
                    (&mut common, VirtioRngRequiredCapability::Common)
                }
                VirtioPciCapabilityKind::Notify => {
                    (&mut notify, VirtioRngRequiredCapability::Notify)
                }
                VirtioPciCapabilityKind::Isr => (&mut isr, VirtioRngRequiredCapability::Isr),
                VirtioPciCapabilityKind::DeviceConfiguration
                | VirtioPciCapabilityKind::PciConfiguration => continue,
            };
            if slot.replace(capability).is_some() {
                return Err(VirtioRngPciCapabilityError::Duplicate(required));
            }
        }
        Ok(Self {
            common: common.ok_or(VirtioRngPciCapabilityError::Missing(
                VirtioRngRequiredCapability::Common,
            ))?,
            notify: notify.ok_or(VirtioRngPciCapabilityError::Missing(
                VirtioRngRequiredCapability::Notify,
            ))?,
            isr: isr.ok_or(VirtioRngPciCapabilityError::Missing(
                VirtioRngRequiredCapability::Isr,
            ))?,
        })
    }

    pub const fn common(self) -> VirtioPciCapability {
        self.common
    }

    pub const fn notify(self) -> VirtioPciCapability {
        self.notify
    }

    pub const fn isr(self) -> VirtioPciCapability {
        self.isr
    }

    pub fn resolve_regions(
        self,
        bars: PciBarSet,
    ) -> Result<VirtioRngPciRegions, VirtioRngPciRegionError> {
        Ok(VirtioRngPciRegions {
            common: resolve_region(self.common, bars, VirtioRngRequiredCapability::Common)?,
            notify: resolve_region(self.notify, bars, VirtioRngRequiredCapability::Notify)?,
            isr: resolve_region(self.isr, bars, VirtioRngRequiredCapability::Isr)?,
        })
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VirtioRngPciRegion {
    bar: PciBar,
    region: VirtioPciBarRegion,
}

impl VirtioRngPciRegion {
    pub const fn bar(self) -> PciBar {
        self.bar
    }

    pub const fn region(self) -> VirtioPciBarRegion {
        self.region
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VirtioRngPciRegions {
    common: VirtioRngPciRegion,
    notify: VirtioRngPciRegion,
    isr: VirtioRngPciRegion,
}

impl VirtioRngPciRegions {
    pub const fn common(self) -> VirtioRngPciRegion {
        self.common
    }

    pub const fn notify(self) -> VirtioRngPciRegion {
        self.notify
    }

    pub const fn isr(self) -> VirtioRngPciRegion {
        self.isr
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VirtioRngRequiredCapability {
    Common,
    Notify,
    Isr,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VirtioRngPciCapabilityError {
    Decode {
        offset: u8,
        error: VirtioPciCapabilityError,
    },
    Missing(VirtioRngRequiredCapability),
    Duplicate(VirtioRngRequiredCapability),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VirtioRngPciRegionError {
    MissingBar {
        capability: VirtioRngRequiredCapability,
        bar: PciBarIndex,
    },
    UnassignedBar {
        capability: VirtioRngRequiredCapability,
        bar: PciBarIndex,
    },
    IoBar {
        capability: VirtioRngRequiredCapability,
        bar: PciBarIndex,
    },
    InvalidRegion {
        capability: VirtioRngRequiredCapability,
        error: VirtioPciCapabilityError,
    },
}

fn resolve_region(
    capability: VirtioPciCapability,
    bars: PciBarSet,
    required: VirtioRngRequiredCapability,
) -> Result<VirtioRngPciRegion, VirtioRngPciRegionError> {
    let bar = bars
        .get(capability.bar())
        .ok_or(VirtioRngPciRegionError::MissingBar {
            capability: required,
            bar: capability.bar(),
        })?;
    if !bar.is_assigned() {
        return Err(VirtioRngPciRegionError::UnassignedBar {
            capability: required,
            bar: bar.index(),
        });
    }
    if bar.kind() == PciBarKind::Io {
        return Err(VirtioRngPciRegionError::IoBar {
            capability: required,
            bar: bar.index(),
        });
    }
    let region = capability
        .bar_region(bar.index(), bar.size())
        .map_err(|error| VirtioRngPciRegionError::InvalidRegion {
            capability: required,
            error,
        })?;
    Ok(VirtioRngPciRegion { bar, region })
}
