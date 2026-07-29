//! Architecture-neutral network authority records.
//!
//! This module keeps MAC identities, endpoint policy, and frame evidence in
//! fixed-width values. Packet bytes and device registers remain architecture
//! concerns.

use crate::{KernelError, NetworkDatagramDescriptor, NetworkTransferId, ResourceId};

pub const NETWORK_MIN_MTU: u16 = 68;
pub const NETWORK_MAX_MTU: u16 = 1500;
pub const ETHERNET_HEADER_BYTES: u16 = 14;
pub const ETHERNET_MAX_FRAME_BYTES: u16 = NETWORK_MAX_MTU + ETHERNET_HEADER_BYTES;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NetworkMacAddress([u8; 6]);

impl NetworkMacAddress {
    pub const fn new(bytes: [u8; 6]) -> Option<Self> {
        let all_zero = bytes[0] == 0
            && bytes[1] == 0
            && bytes[2] == 0
            && bytes[3] == 0
            && bytes[4] == 0
            && bytes[5] == 0;
        if all_zero || bytes[0] & 1 != 0 {
            None
        } else {
            Some(Self(bytes))
        }
    }

    pub const fn bytes(self) -> [u8; 6] {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NetworkEndpointConfig {
    mac: NetworkMacAddress,
    mtu: u16,
}

impl NetworkEndpointConfig {
    pub const fn new(mac: NetworkMacAddress, mtu: u16) -> Result<Self, KernelError> {
        if mtu < NETWORK_MIN_MTU || mtu > NETWORK_MAX_MTU {
            Err(KernelError::NetworkEndpointInvalid)
        } else {
            Ok(Self { mac, mtu })
        }
    }

    pub const fn mac(self) -> NetworkMacAddress {
        self.mac
    }

    pub const fn mtu(self) -> u16 {
        self.mtu
    }

    pub const fn accepts(self, frame: NetworkFrameDescriptor) -> bool {
        frame.length <= self.mtu + ETHERNET_HEADER_BYTES
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NetworkFrameDescriptor {
    length: u16,
    ether_type: u16,
    digest: [u8; 32],
}

impl NetworkFrameDescriptor {
    pub const fn new(length: u16, ether_type: u16, digest: [u8; 32]) -> Result<Self, KernelError> {
        if length < ETHERNET_HEADER_BYTES
            || length > ETHERNET_MAX_FRAME_BYTES
            || ether_type < 0x0600
        {
            Err(KernelError::NetworkFrameInvalid)
        } else {
            Ok(Self {
                length,
                ether_type,
                digest,
            })
        }
    }

    pub const fn length(self) -> u16 {
        self.length
    }

    pub const fn ether_type(self) -> u16 {
        self.ether_type
    }

    pub const fn digest(self) -> [u8; 32] {
        self.digest
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NetworkEndpointStatus {
    Reserved,
    Active,
    Revoking,
    Released,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NetworkEndpointRecord {
    resource: ResourceId,
    device: ResourceId,
    config: NetworkEndpointConfig,
    status: NetworkEndpointStatus,
}

impl NetworkEndpointRecord {
    pub const fn resource(self) -> ResourceId {
        self.resource
    }

    pub const fn device(self) -> ResourceId {
        self.device
    }

    pub const fn config(self) -> NetworkEndpointConfig {
        self.config
    }

    pub const fn status(self) -> NetworkEndpointStatus {
        self.status
    }

    pub const fn occupies_endpoint(self) -> bool {
        !matches!(self.status, NetworkEndpointStatus::Released)
    }

    pub(crate) const fn new(
        resource: ResourceId,
        device: ResourceId,
        config: NetworkEndpointConfig,
    ) -> Self {
        Self {
            resource,
            device,
            config,
            status: NetworkEndpointStatus::Reserved,
        }
    }

    pub(crate) const fn empty() -> Self {
        Self {
            resource: ResourceId::new(0),
            device: ResourceId::new(0),
            config: NetworkEndpointConfig {
                mac: NetworkMacAddress([0x02, 0, 0, 0, 0, 0]),
                mtu: NETWORK_MIN_MTU,
            },
            status: NetworkEndpointStatus::Released,
        }
    }

    pub(crate) fn set_status(&mut self, status: NetworkEndpointStatus) {
        self.status = status;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NetworkTransferDirection {
    Transmit,
    Receive,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NetworkTransferStatus {
    Prepared,
    Completed,
    Failed,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NetworkTransferRecord {
    id: NetworkTransferId,
    endpoint: ResourceId,
    direction: NetworkTransferDirection,
    frame: NetworkFrameDescriptor,
    datagram: Option<NetworkDatagramDescriptor>,
    status: NetworkTransferStatus,
}

impl NetworkTransferRecord {
    pub const fn id(self) -> NetworkTransferId {
        self.id
    }

    pub const fn endpoint(self) -> ResourceId {
        self.endpoint
    }

    pub const fn direction(self) -> NetworkTransferDirection {
        self.direction
    }

    pub const fn frame(self) -> NetworkFrameDescriptor {
        self.frame
    }

    pub const fn datagram(self) -> Option<NetworkDatagramDescriptor> {
        self.datagram
    }

    pub const fn status(self) -> NetworkTransferStatus {
        self.status
    }

    pub(crate) const fn new(
        id: NetworkTransferId,
        endpoint: ResourceId,
        direction: NetworkTransferDirection,
        frame: NetworkFrameDescriptor,
        datagram: Option<NetworkDatagramDescriptor>,
        status: NetworkTransferStatus,
    ) -> Self {
        Self {
            id,
            endpoint,
            direction,
            frame,
            datagram,
            status,
        }
    }

    pub(crate) const fn empty() -> Self {
        Self {
            id: NetworkTransferId::new(0),
            endpoint: ResourceId::new(0),
            direction: NetworkTransferDirection::Receive,
            frame: NetworkFrameDescriptor {
                length: ETHERNET_HEADER_BYTES,
                ether_type: 0x0600,
                digest: [0; 32],
            },
            datagram: None,
            status: NetworkTransferStatus::Failed,
        }
    }

    pub(crate) fn set_status(&mut self, status: NetworkTransferStatus) {
        self.status = status;
    }
}
