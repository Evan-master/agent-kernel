//! HAL backend for the two V30 semantic network commands.

use agent_kernel_core::{
    DriverCommandKind, DriverCommandRequest, DriverCommandResult, NetworkFrameDescriptor,
    NetworkIpv4Address, NetworkMacAddress, NetworkUdpPort,
};
use agent_kernel_hal::{DriverBackend, DriverCommandOutcome};
use agent_kernel_x86_64::virtio_net::{
    build_arp_request, build_udp_ipv4_frame, decode_udp_ipv4_frame, is_expected_arp_reply,
    UdpIpv4Route, VirtioNetRxCompletion, ARP_FRAME_BYTES, QEMU_USER_GATEWAY_IPV4,
    QEMU_USER_GUEST_IPV4,
};
use sha2::{Digest, Sha256};

use crate::smp_boot::SmpBootstrap;

use super::{
    super::interrupts, platform::NativeUdpNet, DRIVER, NETWORK_COMMAND_EXCHANGE_UDP,
    NETWORK_COMMAND_RESOLVE_NEIGHBOR, NETWORK_RESULT_OK, UDP_DESTINATION_PORT, UDP_PAYLOAD,
    UDP_SOURCE_PORT,
};

const NETWORK_RESULT_FAILED: u16 = 1;
const UDP_FRAME_BYTES: usize = 60;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum BackendFailure {
    None,
    Command,
    ReceivePrepare,
    TransmitPrepare,
    ReceiveNotify,
    TransmitNotify,
    TransmitInterrupt,
    TransmitCompletion,
    TransmitEoi,
    ReceiveInterrupt,
    ReceiveCompletion,
    ReceiveEoi,
    ReceiveFrame,
    ArpReply,
    UdpEncode,
    UdpReply,
}

pub(super) struct NetworkDriverBackend<'a, 'dma> {
    net: &'a mut NativeUdpNet<'dma>,
    smp: &'a mut SmpBootstrap,
    guest_mac: NetworkMacAddress,
    gateway_mac: Option<NetworkMacAddress>,
    arp_receive: Option<NetworkFrameDescriptor>,
    udp_receive: Option<NetworkFrameDescriptor>,
    command_count: u8,
    failure: BackendFailure,
}

impl<'a, 'dma> NetworkDriverBackend<'a, 'dma> {
    pub(super) const fn new(
        net: &'a mut NativeUdpNet<'dma>,
        smp: &'a mut SmpBootstrap,
        guest_mac: NetworkMacAddress,
    ) -> Self {
        Self {
            net,
            smp,
            guest_mac,
            gateway_mac: None,
            arp_receive: None,
            udp_receive: None,
            command_count: 0,
            failure: BackendFailure::None,
        }
    }

    pub(super) const fn gateway_mac(&self) -> Option<NetworkMacAddress> {
        self.gateway_mac
    }

    pub(super) const fn arp_receive(&self) -> Option<NetworkFrameDescriptor> {
        self.arp_receive
    }

    pub(super) const fn udp_receive(&self) -> Option<NetworkFrameDescriptor> {
        self.udp_receive
    }

    pub(super) const fn command_count(&self) -> u8 {
        self.command_count
    }

    pub(super) const fn failure_marker(&self) -> &'static str {
        match self.failure {
            BackendFailure::None => "AGENT_KERNEL_NATIVE_UDP_BACKEND_UNKNOWN_ERROR",
            BackendFailure::Command => "AGENT_KERNEL_NATIVE_UDP_BACKEND_COMMAND_ERROR",
            BackendFailure::ReceivePrepare => "AGENT_KERNEL_NATIVE_UDP_RX_PREPARE_ERROR",
            BackendFailure::TransmitPrepare => "AGENT_KERNEL_NATIVE_UDP_TX_PREPARE_ERROR",
            BackendFailure::ReceiveNotify => "AGENT_KERNEL_NATIVE_UDP_RX_NOTIFY_ERROR",
            BackendFailure::TransmitNotify => "AGENT_KERNEL_NATIVE_UDP_TX_NOTIFY_ERROR",
            BackendFailure::TransmitInterrupt => "AGENT_KERNEL_NATIVE_UDP_TX_INTERRUPT_ERROR",
            BackendFailure::TransmitCompletion => "AGENT_KERNEL_NATIVE_UDP_TX_COMPLETION_ERROR",
            BackendFailure::TransmitEoi => "AGENT_KERNEL_NATIVE_UDP_TX_EOI_ERROR",
            BackendFailure::ReceiveInterrupt => "AGENT_KERNEL_NATIVE_UDP_RX_INTERRUPT_ERROR",
            BackendFailure::ReceiveCompletion => "AGENT_KERNEL_NATIVE_UDP_RX_COMPLETION_ERROR",
            BackendFailure::ReceiveEoi => "AGENT_KERNEL_NATIVE_UDP_RX_EOI_ERROR",
            BackendFailure::ReceiveFrame => "AGENT_KERNEL_NATIVE_UDP_RX_FRAME_ERROR",
            BackendFailure::ArpReply => "AGENT_KERNEL_NATIVE_UDP_ARP_REPLY_ERROR",
            BackendFailure::UdpEncode => "AGENT_KERNEL_NATIVE_UDP_ENCODE_ERROR",
            BackendFailure::UdpReply => "AGENT_KERNEL_NATIVE_UDP_ECHO_ERROR",
        }
    }

    fn resolve_neighbor(&mut self) -> Result<(), BackendFailure> {
        let mut request = [0; ARP_FRAME_BYTES];
        build_arp_request(&mut request, self.guest_mac);
        let completion = self.exchange(&request)?;
        let (gateway_mac, descriptor) = {
            let reply = self
                .net
                .frame(&completion)
                .map_err(|_| BackendFailure::ReceiveFrame)?;
            if !is_expected_arp_reply(reply, self.guest_mac) {
                return Err(BackendFailure::ArpReply);
            }
            let gateway_mac = NetworkMacAddress::new(
                reply
                    .get(6..12)
                    .ok_or(BackendFailure::ArpReply)?
                    .try_into()
                    .map_err(|_| BackendFailure::ArpReply)?,
            )
            .ok_or(BackendFailure::ArpReply)?;
            let descriptor =
                normalized_descriptor(reply, ARP_FRAME_BYTES).ok_or(BackendFailure::ArpReply)?;
            (gateway_mac, descriptor)
        };
        self.gateway_mac = Some(gateway_mac);
        self.arp_receive = Some(descriptor);
        Ok(())
    }

    fn exchange_udp(&mut self) -> Result<(), BackendFailure> {
        let gateway_mac = self.gateway_mac.ok_or(BackendFailure::Command)?;
        let guest_ip = NetworkIpv4Address::new(QEMU_USER_GUEST_IPV4).expect("fixed guest IPv4");
        let gateway_ip =
            NetworkIpv4Address::new(QEMU_USER_GATEWAY_IPV4).expect("fixed gateway IPv4");
        let guest_port = NetworkUdpPort::new(UDP_SOURCE_PORT).expect("fixed guest port");
        let echo_port = NetworkUdpPort::new(UDP_DESTINATION_PORT).expect("fixed echo port");
        let outbound = UdpIpv4Route::new(
            self.guest_mac,
            gateway_mac,
            guest_ip,
            gateway_ip,
            guest_port,
            echo_port,
        );
        let inbound = UdpIpv4Route::new(
            gateway_mac,
            self.guest_mac,
            gateway_ip,
            guest_ip,
            echo_port,
            guest_port,
        );
        let mut request = [0; UDP_FRAME_BYTES];
        let length = build_udp_ipv4_frame(&mut request, outbound, 0x3002, UDP_PAYLOAD)
            .map_err(|_| BackendFailure::UdpEncode)?;
        let completion = self.exchange(&request[..length])?;
        let descriptor = {
            let reply = self
                .net
                .frame(&completion)
                .map_err(|_| BackendFailure::ReceiveFrame)?;
            let packet =
                decode_udp_ipv4_frame(reply, inbound).map_err(|_| BackendFailure::UdpReply)?;
            if packet.payload() != UDP_PAYLOAD {
                return Err(BackendFailure::UdpReply);
            }
            normalized_descriptor(reply, UDP_FRAME_BYTES).ok_or(BackendFailure::UdpReply)?
        };
        self.udp_receive = Some(descriptor);
        Ok(())
    }

    fn exchange(&mut self, frame: &[u8]) -> Result<VirtioNetRxCompletion, BackendFailure> {
        self.net
            .prepare_receive()
            .map_err(|_| BackendFailure::ReceivePrepare)?;
        self.net
            .prepare_transmit(frame)
            .map_err(|_| BackendFailure::TransmitPrepare)?;
        interrupts::reset();
        self.net
            .notify_receive()
            .map_err(|_| BackendFailure::ReceiveNotify)?;
        self.net
            .notify_transmit()
            .map_err(|_| BackendFailure::TransmitNotify)?;
        if !interrupts::wait_for_counts(0, 1) {
            return Err(BackendFailure::TransmitInterrupt);
        }
        self.net
            .complete_transmit()
            .map_err(|_| BackendFailure::TransmitCompletion)?;
        self.smp
            .complete_message_interrupt(true)
            .map_err(|_| BackendFailure::TransmitEoi)?;
        if !interrupts::wait_for_counts(1, 1) {
            return Err(BackendFailure::ReceiveInterrupt);
        }
        let completion = self
            .net
            .complete_receive()
            .map_err(|_| BackendFailure::ReceiveCompletion)?;
        self.smp
            .complete_message_interrupt(true)
            .map_err(|_| BackendFailure::ReceiveEoi)?;
        Ok(completion)
    }

    const fn success(value: u64) -> DriverCommandOutcome {
        DriverCommandOutcome::Completed(DriverCommandResult {
            code: NETWORK_RESULT_OK,
            value,
        })
    }

    const fn failure() -> DriverCommandOutcome {
        DriverCommandOutcome::Failed(DriverCommandResult {
            code: NETWORK_RESULT_FAILED,
            value: 0,
        })
    }
}

impl DriverBackend for NetworkDriverBackend<'_, '_> {
    fn execute(&mut self, request: DriverCommandRequest) -> DriverCommandOutcome {
        let accepted = match (
            self.command_count,
            request.driver,
            request.kind,
            request.payload.opcode,
            request.payload.value,
        ) {
            (0, DRIVER, DriverCommandKind::Configure, NETWORK_COMMAND_RESOLVE_NEIGHBOR, 0) => {
                self.resolve_neighbor()
            }
            (1, DRIVER, DriverCommandKind::Write, NETWORK_COMMAND_EXCHANGE_UDP, value)
                if value == UDP_PAYLOAD.len() as u64 =>
            {
                self.exchange_udp()
            }
            _ => Err(BackendFailure::Command),
        };
        if let Err(failure) = accepted {
            self.failure = failure;
            return Self::failure();
        }
        self.failure = BackendFailure::None;
        self.command_count += 1;
        Self::success(request.payload.value)
    }
}

fn normalized_descriptor(frame: &[u8], canonical_length: usize) -> Option<NetworkFrameDescriptor> {
    if frame.len() < 14
        || (frame.len() > canonical_length
            && frame[canonical_length..].iter().any(|byte| *byte != 0))
    {
        return None;
    }
    let mut hasher = Sha256::new();
    const ZERO_PADDING: [u8; UDP_FRAME_BYTES] = [0; UDP_FRAME_BYTES];
    if frame.len() >= canonical_length {
        hasher.update(&frame[..canonical_length]);
    } else {
        hasher.update(frame);
        hasher.update(&ZERO_PADDING[..canonical_length - frame.len()]);
    }
    let digest: [u8; 32] = hasher.finalize().into();
    NetworkFrameDescriptor::new(
        u16::try_from(canonical_length).ok()?,
        u16::from_be_bytes([frame[12], frame[13]]),
        digest,
    )
    .ok()
}
