use agent_kernel_x86_64::edu::{
    EduDma, EduDmaError, EduRegisterIo, EDU_COMMAND_REG, EDU_COUNT_REG, EDU_DESTINATION_REG,
    EDU_DMA_INTERRUPT, EDU_IDENTITY_REG, EDU_INTERRUPT_ACK_REG, EDU_INTERRUPT_STATUS_REG,
    EDU_SOURCE_REG,
};

const IOVA: u64 = 0x0100_0000;

#[test]
fn edu_dma_encodes_both_transfer_directions() {
    let mut dma = EduDma::bind(ScriptedEdu::new(), 4).unwrap();

    dma.copy_memory_to_device(IOVA, 0x40000, 64).unwrap();
    dma.copy_device_to_memory(0x40000, IOVA, 64).unwrap();
    let io = dma.into_io();

    assert_eq!(
        io.writes,
        [
            Write::U64(EDU_SOURCE_REG, IOVA),
            Write::U64(EDU_DESTINATION_REG, 0x40000),
            Write::U64(EDU_COUNT_REG, 64),
            Write::U64(EDU_COMMAND_REG, 1),
            Write::U64(EDU_SOURCE_REG, 0x40000),
            Write::U64(EDU_DESTINATION_REG, IOVA),
            Write::U64(EDU_COUNT_REG, 64),
            Write::U64(EDU_COMMAND_REG, 3),
        ]
    );
}

#[test]
fn edu_dma_rejects_out_of_range_device_buffers_before_mmio() {
    let mut dma = EduDma::bind(ScriptedEdu::new(), 4).unwrap();
    assert_eq!(
        dma.copy_memory_to_device(IOVA, 0x3fff0, 64),
        Err(EduDmaError::DeviceBufferOutOfRange)
    );
    assert_eq!(dma.into_io().writes, []);
}

#[test]
fn edu_dma_requests_and_acknowledges_one_device_interrupt() {
    let mut dma = EduDma::bind(ScriptedEdu::new(), 4).unwrap();

    dma.copy_memory_to_device_interrupting(IOVA, 0x40000, 64)
        .unwrap();
    assert_eq!(dma.acknowledge_dma_interrupt().unwrap(), EDU_DMA_INTERRUPT);
    let io = dma.into_io();

    assert_eq!(
        io.writes,
        [
            Write::U64(EDU_SOURCE_REG, IOVA),
            Write::U64(EDU_DESTINATION_REG, 0x40000),
            Write::U64(EDU_COUNT_REG, 64),
            Write::U64(EDU_COMMAND_REG, 5),
            Write::U32(EDU_INTERRUPT_ACK_REG, EDU_DMA_INTERRUPT),
        ]
    );
    assert_eq!(io.interrupt_status, 0);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Write {
    U32(u16, u32),
    U64(u16, u64),
}

struct ScriptedEdu {
    writes: Vec<Write>,
    command: u64,
    interrupt_status: u32,
}

impl ScriptedEdu {
    fn new() -> Self {
        Self {
            writes: Vec::new(),
            command: 0,
            interrupt_status: 0,
        }
    }
}

impl EduRegisterIo for ScriptedEdu {
    fn read_u32(&mut self, offset: u16) -> u32 {
        match offset {
            EDU_IDENTITY_REG => 0x0100_00ed,
            EDU_INTERRUPT_STATUS_REG => self.interrupt_status,
            _ => 0,
        }
    }

    fn read_u64(&mut self, offset: u16) -> u64 {
        if offset == EDU_COMMAND_REG {
            let command = self.command;
            self.command = 0;
            command
        } else {
            0
        }
    }

    fn write_u64(&mut self, offset: u16, value: u64) {
        self.writes.push(Write::U64(offset, value));
        if offset == EDU_COMMAND_REG {
            self.command = value;
            if value & 4 != 0 {
                self.interrupt_status |= EDU_DMA_INTERRUPT;
            }
        }
    }

    fn write_u32(&mut self, offset: u16, value: u32) {
        self.writes.push(Write::U32(offset, value));
        if offset == EDU_INTERRUPT_ACK_REG {
            self.interrupt_status &= !value;
        }
    }
}
