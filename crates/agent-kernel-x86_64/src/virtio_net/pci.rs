//! Exact modern virtio-net PCI identity and required capability set.
//!
//! This x86 owner selects one Common, Notify, ISR, and Device Configuration
//! capability before any BAR mapping or MMIO access.

use crate::pci::{
    PciBar, PciBarIndex, PciBarKind, PciBarSet, PciCapabilityList, PciConfigAccess,
    PciFunctionAddress, PciFunctionSelector, VirtioPciBarRegion, VirtioPciCapability,
    VirtioPciCapabilityError, VirtioPciCapabilityKind, PCI_CAPABILITY_ID_VENDOR_SPECIFIC,
};

use super::{VIRTIO_NET_DEVICE_ID, VIRTIO_NET_VENDOR_ID};

pub fn virtio_net_selector(address: PciFunctionAddress) -> PciFunctionSelector {
    PciFunctionSelector::new(address, VIRTIO_NET_VENDOR_ID, VIRTIO_NET_DEVICE_ID)
        .expect("fixed virtio-net identity")
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VirtioNetPciCapabilities {
    common: VirtioPciCapability,
    notify: VirtioPciCapability,
    isr: VirtioPciCapability,
    device: VirtioPciCapability,
}

impl VirtioNetPciCapabilities {
    pub fn decode<A: PciConfigAccess, const CAPACITY: usize>(
        access: &mut A,
        address: PciFunctionAddress,
        list: &PciCapabilityList<CAPACITY>,
    ) -> Result<Self, VirtioNetPciCapabilityError> {
        let mut common = None;
        let mut notify = None;
        let mut isr = None;
        let mut device = None;
        for record in list
            .all()
            .iter()
            .copied()
            .filter(|record| record.id() == PCI_CAPABILITY_ID_VENDOR_SPECIFIC)
        {
            let capability =
                VirtioPciCapability::decode(access, address, record).map_err(|error| {
                    VirtioNetPciCapabilityError::Decode {
                        offset: record.offset(),
                        error,
                    }
                })?;
            let (slot, required) = match capability.kind() {
                VirtioPciCapabilityKind::CommonConfiguration => {
                    (&mut common, VirtioNetRequiredCapability::Common)
                }
                VirtioPciCapabilityKind::Notify => {
                    (&mut notify, VirtioNetRequiredCapability::Notify)
                }
                VirtioPciCapabilityKind::Isr => (&mut isr, VirtioNetRequiredCapability::Isr),
                VirtioPciCapabilityKind::DeviceConfiguration => {
                    (&mut device, VirtioNetRequiredCapability::Device)
                }
                VirtioPciCapabilityKind::PciConfiguration => continue,
            };
            if slot.replace(capability).is_some() {
                return Err(VirtioNetPciCapabilityError::Duplicate(required));
            }
        }
        Ok(Self {
            common: required(common, VirtioNetRequiredCapability::Common)?,
            notify: required(notify, VirtioNetRequiredCapability::Notify)?,
            isr: required(isr, VirtioNetRequiredCapability::Isr)?,
            device: required(device, VirtioNetRequiredCapability::Device)?,
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

    pub const fn device(self) -> VirtioPciCapability {
        self.device
    }

    pub fn resolve_regions(
        self,
        bars: PciBarSet,
    ) -> Result<VirtioNetPciRegions, VirtioNetPciRegionError> {
        Ok(VirtioNetPciRegions {
            common: resolve_region(self.common, bars, VirtioNetRequiredCapability::Common)?,
            notify: resolve_region(self.notify, bars, VirtioNetRequiredCapability::Notify)?,
            isr: resolve_region(self.isr, bars, VirtioNetRequiredCapability::Isr)?,
            device: resolve_region(self.device, bars, VirtioNetRequiredCapability::Device)?,
        })
    }
}

fn required(
    capability: Option<VirtioPciCapability>,
    kind: VirtioNetRequiredCapability,
) -> Result<VirtioPciCapability, VirtioNetPciCapabilityError> {
    capability.ok_or(VirtioNetPciCapabilityError::Missing(kind))
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VirtioNetPciRegion {
    bar: PciBar,
    region: VirtioPciBarRegion,
}

impl VirtioNetPciRegion {
    pub const fn bar(self) -> PciBar {
        self.bar
    }

    pub const fn region(self) -> VirtioPciBarRegion {
        self.region
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct VirtioNetPciRegions {
    common: VirtioNetPciRegion,
    notify: VirtioNetPciRegion,
    isr: VirtioNetPciRegion,
    device: VirtioNetPciRegion,
}

impl VirtioNetPciRegions {
    pub const fn common(self) -> VirtioNetPciRegion {
        self.common
    }

    pub const fn notify(self) -> VirtioNetPciRegion {
        self.notify
    }

    pub const fn isr(self) -> VirtioNetPciRegion {
        self.isr
    }

    pub const fn device(self) -> VirtioNetPciRegion {
        self.device
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VirtioNetRequiredCapability {
    Common,
    Notify,
    Isr,
    Device,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VirtioNetPciCapabilityError {
    Decode {
        offset: u8,
        error: VirtioPciCapabilityError,
    },
    Missing(VirtioNetRequiredCapability),
    Duplicate(VirtioNetRequiredCapability),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VirtioNetPciRegionError {
    MissingBar {
        capability: VirtioNetRequiredCapability,
        bar: PciBarIndex,
    },
    UnassignedBar {
        capability: VirtioNetRequiredCapability,
        bar: PciBarIndex,
    },
    IoBar {
        capability: VirtioNetRequiredCapability,
        bar: PciBarIndex,
    },
    InvalidRegion {
        capability: VirtioNetRequiredCapability,
        error: VirtioPciCapabilityError,
    },
}

fn resolve_region(
    capability: VirtioPciCapability,
    bars: PciBarSet,
    required: VirtioNetRequiredCapability,
) -> Result<VirtioNetPciRegion, VirtioNetPciRegionError> {
    let bar = bars
        .get(capability.bar())
        .ok_or(VirtioNetPciRegionError::MissingBar {
            capability: required,
            bar: capability.bar(),
        })?;
    if !bar.is_assigned() {
        return Err(VirtioNetPciRegionError::UnassignedBar {
            capability: required,
            bar: bar.index(),
        });
    }
    if bar.kind() == PciBarKind::Io {
        return Err(VirtioNetPciRegionError::IoBar {
            capability: required,
            bar: bar.index(),
        });
    }
    let region = capability
        .bar_region(bar.index(), bar.size())
        .map_err(|error| VirtioNetPciRegionError::InvalidRegion {
            capability: required,
            error,
        })?;
    Ok(VirtioNetPciRegion { bar, region })
}
