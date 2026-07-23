//! Register-width boundary used by ATA PIO and host contract doubles.

pub trait AtaRegisterIo {
    fn read_u8(&mut self, port: u16) -> u8;

    fn write_u8(&mut self, port: u16, value: u8);

    fn read_u16(&mut self, port: u16) -> u16;

    fn write_u16(&mut self, port: u16, value: u16);
}
