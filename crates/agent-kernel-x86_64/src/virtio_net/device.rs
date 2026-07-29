//! Ordered modern Virtio network device owner.
//!
//! This x86 owner composes transport, MAC configuration, and one in-flight
//! request per Rx/Tx queue. MSI-X vector identity selects the completion queue.

use agent_kernel_core::NetworkMacAddress;

use crate::virtio_rng::{VirtioCommonConfigIo, VirtioIsrIo, VirtioNotifyIo};

use super::{
    VirtioNetDeviceConfigIo, VirtioNetInterrupt, VirtioNetQueueError, VirtioNetQueueLayout,
    VirtioNetRxCompletion, VirtioNetRxQueue, VirtioNetRxRequest, VirtioNetTransport,
    VirtioNetTransportError, VirtioNetTxCompletion, VirtioNetTxQueue, VirtioNetTxRequest,
};

pub struct VirtioNetDevice<'a, C, N, I, D> {
    transport: VirtioNetTransport<C, N, I>,
    config: D,
    rx: VirtioNetRxQueue<'a>,
    tx: VirtioNetTxQueue<'a>,
    mac: Option<NetworkMacAddress>,
    pending_rx: Option<PendingRx>,
    pending_tx: Option<PendingTx>,
}

impl<
        'a,
        C: VirtioCommonConfigIo,
        N: VirtioNotifyIo,
        I: VirtioIsrIo,
        D: VirtioNetDeviceConfigIo,
    > VirtioNetDevice<'a, C, N, I, D>
{
    #[allow(clippy::too_many_arguments)]
    pub fn bind(
        common: C,
        notify: N,
        isr: I,
        config: D,
        poll_budget: u32,
        rx_metadata: &'a mut [u8; 4096],
        rx_packet: &'a mut [u8; 4096],
        rx_layout: VirtioNetQueueLayout,
        tx_metadata: &'a mut [u8; 4096],
        tx_packet: &'a mut [u8; 4096],
        tx_layout: VirtioNetQueueLayout,
    ) -> Result<Self, VirtioNetDeviceError> {
        let rx_pages = [rx_layout.descriptor_iova(), rx_layout.packet_iova()];
        let tx_pages = [tx_layout.descriptor_iova(), tx_layout.packet_iova()];
        if rx_pages.iter().any(|page| tx_pages.contains(page)) {
            return Err(VirtioNetDeviceError::OverlappingQueuePages);
        }
        let transport = VirtioNetTransport::bind(common, notify, isr, poll_budget)
            .map_err(VirtioNetDeviceError::Transport)?;
        Ok(Self {
            transport,
            config,
            rx: VirtioNetRxQueue::bind(rx_metadata, rx_packet, rx_layout),
            tx: VirtioNetTxQueue::bind(tx_metadata, tx_packet, tx_layout),
            mac: None,
            pending_rx: None,
            pending_tx: None,
        })
    }

    pub fn initialize(
        &mut self,
        rx_msix_vector: u16,
        tx_msix_vector: u16,
    ) -> Result<NetworkMacAddress, VirtioNetDeviceError> {
        self.transport
            .initialize(
                self.rx.layout(),
                self.tx.layout(),
                rx_msix_vector,
                tx_msix_vector,
            )
            .map_err(VirtioNetDeviceError::Transport)?;
        let Some(mac) = NetworkMacAddress::new(self.config.read_mac()) else {
            let _ = self.transport.shutdown();
            return Err(VirtioNetDeviceError::InvalidMac);
        };
        self.mac = Some(mac);
        Ok(mac)
    }

    pub fn prepare_receive(&mut self) -> Result<(), VirtioNetDeviceError> {
        self.ensure_initialized()?;
        if self.pending_rx.is_some() {
            return Err(VirtioNetDeviceError::ReceivePending);
        }
        let request = self.rx.post_buffer().map_err(VirtioNetDeviceError::Queue)?;
        self.pending_rx = Some(PendingRx {
            request,
            notified: false,
        });
        Ok(())
    }

    pub fn notify_receive(&mut self) -> Result<(), VirtioNetDeviceError> {
        let pending = self
            .pending_rx
            .ok_or(VirtioNetDeviceError::NoReceivePending)?;
        if pending.notified {
            return Err(VirtioNetDeviceError::RequestAlreadyNotified);
        }
        self.transport
            .notify_receive()
            .map_err(VirtioNetDeviceError::Transport)?;
        self.pending_rx
            .as_mut()
            .expect("pending receive remains owned")
            .notified = true;
        Ok(())
    }

    pub fn complete_receive(&mut self) -> Result<VirtioNetRxCompletion, VirtioNetDeviceError> {
        let pending = self
            .pending_rx
            .ok_or(VirtioNetDeviceError::NoReceivePending)?;
        if !pending.notified {
            return Err(VirtioNetDeviceError::RequestNotNotified);
        }
        let completion = self
            .rx
            .complete_buffer(pending.request)
            .map_err(VirtioNetDeviceError::Queue)?;
        self.pending_rx = None;
        Ok(completion)
    }

    pub fn frame<'b>(
        &'b self,
        completion: &VirtioNetRxCompletion,
    ) -> Result<&'b [u8], VirtioNetDeviceError> {
        self.rx
            .frame(completion)
            .map_err(VirtioNetDeviceError::Queue)
    }

    pub fn prepare_transmit(&mut self, frame: &[u8]) -> Result<(), VirtioNetDeviceError> {
        self.ensure_initialized()?;
        if self.pending_tx.is_some() {
            return Err(VirtioNetDeviceError::TransmitPending);
        }
        let request = self
            .tx
            .prepare_frame(frame)
            .map_err(VirtioNetDeviceError::Queue)?;
        self.pending_tx = Some(PendingTx {
            request,
            notified: false,
        });
        Ok(())
    }

    pub fn notify_transmit(&mut self) -> Result<(), VirtioNetDeviceError> {
        let pending = self
            .pending_tx
            .ok_or(VirtioNetDeviceError::NoTransmitPending)?;
        if pending.notified {
            return Err(VirtioNetDeviceError::RequestAlreadyNotified);
        }
        self.transport
            .notify_transmit()
            .map_err(VirtioNetDeviceError::Transport)?;
        self.pending_tx
            .as_mut()
            .expect("pending transmit remains owned")
            .notified = true;
        Ok(())
    }

    pub fn complete_transmit(&mut self) -> Result<VirtioNetTxCompletion, VirtioNetDeviceError> {
        let pending = self
            .pending_tx
            .ok_or(VirtioNetDeviceError::NoTransmitPending)?;
        if !pending.notified {
            return Err(VirtioNetDeviceError::RequestNotNotified);
        }
        let completion = self
            .tx
            .complete_frame(pending.request)
            .map_err(VirtioNetDeviceError::Queue)?;
        self.pending_tx = None;
        Ok(completion)
    }

    pub fn acknowledge_shared_interrupt(
        &mut self,
    ) -> Result<VirtioNetInterrupt, VirtioNetDeviceError> {
        self.transport
            .acknowledge_interrupt()
            .map_err(VirtioNetDeviceError::Transport)
    }

    pub fn shutdown(&mut self) -> Result<(), VirtioNetDeviceError> {
        self.transport
            .shutdown()
            .map_err(VirtioNetDeviceError::Transport)?;
        self.rx.reset_after_device_reset();
        self.tx.reset_after_device_reset();
        self.mac = None;
        self.pending_rx = None;
        self.pending_tx = None;
        Ok(())
    }

    fn ensure_initialized(&self) -> Result<(), VirtioNetDeviceError> {
        if self.mac.is_some() {
            Ok(())
        } else {
            Err(VirtioNetDeviceError::NotInitialized)
        }
    }
}

#[derive(Copy, Clone)]
struct PendingRx {
    request: VirtioNetRxRequest,
    notified: bool,
}

#[derive(Copy, Clone)]
struct PendingTx {
    request: VirtioNetTxRequest,
    notified: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VirtioNetDeviceError {
    Transport(VirtioNetTransportError),
    Queue(VirtioNetQueueError),
    OverlappingQueuePages,
    InvalidMac,
    NotInitialized,
    ReceivePending,
    NoReceivePending,
    TransmitPending,
    NoTransmitPending,
    RequestAlreadyNotified,
    RequestNotNotified,
}
