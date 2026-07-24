//! CRB locality, control-area, and TPM response-header constants.
//!
//! This child module keeps PTP-defined offsets and masks separate from the
//! transport state machine. Values are scoped to the parent CRB module.

pub(super) const LOCALITY_OFFSET: u64 = 0x40;
pub(super) const LOC_STATE: u64 = 0x00;
pub(super) const LOC_CTRL: u64 = 0x08;
pub(super) const LOC_STS: u64 = 0x0c;
pub(super) const INTF_ID: u64 = 0x30;

pub(super) const INTF_TYPE_MASK: u32 = 0x0f;
pub(super) const INTF_TYPE_CRB: u32 = 1;
pub(super) const INTF_VERSION_SHIFT: u32 = 4;
pub(super) const INTF_VERSION_MASK: u32 = 0x0f;
pub(super) const INTF_VERSION_MAX_SUPPORTED: u32 = 3;
pub(super) const INTF_CAP_CRB: u32 = 1 << 14;
pub(super) const INTF_RESERVED_MASK: u32 = (0b11 << 15) | (0b11 << 20);

pub(super) const CTRL_REQ: u64 = 0x00;
pub(super) const CTRL_STS: u64 = 0x04;
pub(super) const CTRL_CANCEL: u64 = 0x08;
pub(super) const CTRL_START: u64 = 0x0c;
pub(super) const CTRL_CMD_SIZE: u64 = 0x18;
pub(super) const CTRL_CMD_LADDR: u64 = 0x1c;
pub(super) const CTRL_CMD_HADDR: u64 = 0x20;
pub(super) const CTRL_RSP_SIZE: u64 = 0x24;
pub(super) const CTRL_RSP_LADDR: u64 = 0x28;
pub(super) const CTRL_RSP_HADDR: u64 = 0x2c;

pub(super) const LOC_STATE_VALID: u32 = 1 << 7;
pub(super) const LOC_STATE_ASSIGNED: u32 = 1 << 1;
pub(super) const LOC_STATE_ACTIVE_MASK: u32 = 0b111 << 2;
pub(super) const LOC_STS_GRANTED: u32 = 1;
pub(super) const LOC_STS_SEIZED: u32 = 1 << 1;
pub(super) const LOC_CTRL_REQUEST_ACCESS: u32 = 1;
pub(super) const LOC_CTRL_RELINQUISH: u32 = 1 << 1;

pub(super) const CTRL_REQ_CMD_READY: u32 = 1;
pub(super) const CTRL_REQ_GO_IDLE: u32 = 1 << 1;
pub(super) const CTRL_STS_FATAL: u32 = 1;
pub(super) const CTRL_STS_IDLE: u32 = 1 << 1;
pub(super) const CTRL_CANCEL_CANCEL: u32 = 1;
pub(super) const CTRL_START_START: u32 = 1;

pub(super) const TPM_HEADER_SIZE: usize = 10;
pub(super) const TPM_ST_NO_SESSIONS: u16 = 0x8001;
pub(super) const TPM_ST_SESSIONS: u16 = 0x8002;
