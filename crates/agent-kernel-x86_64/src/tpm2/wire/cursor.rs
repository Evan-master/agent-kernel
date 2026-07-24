//! Bounds-checked cursor for TPM big-endian response fields.
//!
//! This private wire helper advances only after a complete field is available
//! and never allocates or interprets policy.

use super::TpmWireError;

pub(super) struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    pub(super) const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    pub(super) fn u16(&mut self) -> Result<u16, TpmWireError> {
        let bytes = self.take(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    pub(super) fn u32(&mut self) -> Result<u32, TpmWireError> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub(super) fn take(&mut self, length: usize) -> Result<&'a [u8], TpmWireError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(TpmWireError::Truncated)?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or(TpmWireError::Truncated)?;
        self.offset = end;
        Ok(bytes)
    }

    pub(super) fn tpm2b(&mut self) -> Result<&'a [u8], TpmWireError> {
        let length = usize::from(self.u16()?);
        self.take(length)
    }

    pub(super) fn remaining(&self) -> &'a [u8] {
        &self.bytes[self.offset..]
    }

    pub(super) fn is_finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}
