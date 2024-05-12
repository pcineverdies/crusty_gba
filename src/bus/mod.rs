use crate::arm7_tdmi;
use crate::gpu;
use crate::io::keypad;
use crate::memory;

/// bus::TransferSize
///
/// enum to represent the size of the current transfer (BYTE = 8, HALFWORD = 16, WORD = 32).
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
#[repr(u8)]
pub enum TransferSize {
    BYTE = 0,
    HALFWORD = 1,
    #[default]
    WORD = 2,
}

/// bus::BusCycle
///
/// enum to represent the value of the type of bus cycle for the next operation (which is sent
/// together with the current request)
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
#[repr(u8)]
#[allow(dead_code)] // COPROCESSOR is not used
pub enum BusCycle {
    #[default]
    NONSEQUENTIAL = 0,
    SEQUENTIAL = 1,
    INTERNAL = 2,
    COPROCESSOR = 3,
}

/// bus::BusSignal
///
/// enum to represent the value of a one-bit bus signal
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
#[repr(u8)]
pub enum BusSignal {
    HIGH = 1,
    #[default]
    LOW = 0,
}

/// bus::MemoryRequest
///
/// structure to represent a request towards the bus
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct MemoryRequest {
    pub address: u32,
    pub data: u32,
    pub nr_w: BusSignal,
    pub mas: TransferSize,
    pub n_opc: BusSignal,
    pub n_trans: BusSignal,
    pub lock: BusSignal,
    pub t_bit: BusSignal,
    pub bus_cycle: BusCycle,
}

/// bus::MemoryResponse
///
/// structure to represent a response from the bus to a memory request
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct MemoryResponse {
    pub data: u32,
    pub n_wait: BusSignal,
}

pub struct Bus {
    pub cpu: arm7_tdmi::ARM7TDMI,
    pub gpu: gpu::Gpu,
    pub keypad: keypad::Keypad,
    pub gamepak: memory::Memory,
    pub gamepak_sram: memory::Memory,
    pub ewram: memory::Memory,
    pub iwram: memory::Memory,
    pub bios: memory::Memory,
    next_cpu_response: MemoryResponse,
    next_transaction: BusCycle,
    step_counter: u64,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            cpu: arm7_tdmi::ARM7TDMI::new(),
            gpu: gpu::Gpu::new(),
            keypad: keypad::Keypad::new(),
            gamepak: memory::Memory::new(0x08000000, 0x06000000, true, String::from("GAMEPAK")),
            gamepak_sram: memory::Memory::new(0x0e000000, 0x10000, false, String::from("GAMEPAK")),
            ewram: memory::Memory::new(0x02000000, 0x00040000, false, String::from("EWRAM")),
            iwram: memory::Memory::new(0x03000000, 0x00008000, false, String::from("IWRAM")),
            bios: memory::Memory::new(0x00000000, 0x00004000, true, String::from("BIOS")),
            next_cpu_response: MemoryResponse {
                data: arm7_tdmi::NOP,
                n_wait: BusSignal::HIGH,
            },
            next_transaction: BusCycle::SEQUENTIAL,
            step_counter: 0,
        }
    }

    pub fn step(&mut self) {
        let cpu_request = self.cpu.step(self.next_cpu_response);
        self.gpu.step();

        if self.step_counter % 279620 == 0 {
            self.keypad.step();
        }

        if self.next_transaction != BusCycle::INTERNAL {
            if cpu_request.nr_w == BusSignal::LOW {
                self.next_cpu_response = self.read(cpu_request);
            } else {
                self.next_cpu_response = self.write(cpu_request);
            }
        }
        self.next_transaction = cpu_request.bus_cycle;

        self.step_counter += 1;
    }

    fn read(&mut self, req: MemoryRequest) -> MemoryResponse {
        let mut rsp = MemoryResponse {
            data: 0,
            n_wait: BusSignal::HIGH,
        };

        if req.address <= 0x00003ffff {
            rsp.data = self.bios.read(req.address, req.mas)
        } else if req.address >= 0x02000000 && req.address <= 0x02ffffff {
            rsp.data = self.ewram.read(req.address & 0x0203ffff, req.mas)
        } else if req.address >= 0x03000000 && req.address <= 0x03ffffff {
            rsp.data = self.iwram.read(req.address & 0x03007fff, req.mas)
        } else if req.address >= 0x04000000 && req.address <= 0x04000058 {
            rsp.data = self.gpu.read(req.address, req.mas);
        } else if req.address >= 0x04000130 && req.address <= 0x04000133 {
            rsp.data = self.keypad.read(req.address, req.mas);
        } else if req.address >= 0x05000000 && req.address <= 0x05000400 {
            rsp.data = self.gpu.read(req.address, req.mas);
        } else if req.address >= 0x06000000 && req.address <= 0x06018000 {
            rsp.data = self.gpu.read(req.address, req.mas);
        } else if req.address >= 0x07000000 && req.address <= 0x07000400 {
            rsp.data = self.gpu.read(req.address, req.mas);
        } else if req.address >= 0x08000000 && req.address <= 0x0dffffff {
            rsp.data = self.gamepak.read(req.address, req.mas)
        } else if req.address >= 0x0e000000 {
            rsp.data = self
                .gamepak_sram
                .read(req.address & 0xffff | 0x0e000000, req.mas)
        } else {
            todo!("reading from {:#08x}", req.address);
        }

        return rsp;
    }

    fn write(&mut self, req: MemoryRequest) -> MemoryResponse {
        let rsp = MemoryResponse {
            data: 0,
            n_wait: BusSignal::HIGH,
        };

        if req.address >= 0x08000000 && req.address <= 0x0dffffff {
            self.gamepak.write(req.address, req.data, req.mas)
        } else if req.address <= 0x00003ffff {
            self.bios.write(req.address, req.data, req.mas)
        } else if req.address >= 0x02000000 && req.address <= 0x02ffffff {
            self.ewram
                .write(req.address & 0x0203ffff, req.data, req.mas)
        } else if req.address >= 0x03000000 && req.address <= 0x03ffffff {
            self.iwram
                .write(req.address & 0x03007fff, req.data, req.mas)
        } else if req.address >= 0x06000000 && req.address <= 0x06018000 {
            self.gpu.write(req.address, req.data, req.mas);
        } else if req.address >= 0x05000000 && req.address <= 0x05000400 {
            self.gpu.write(req.address, req.data, req.mas);
        } else if req.address >= 0x07000000 && req.address <= 0x07000400 {
            self.gpu.write(req.address, req.data, req.mas);
        } else if req.address >= 0x04000000 && req.address <= 0x04000058 {
            self.gpu.write(req.address, req.data, req.mas);
        } else if req.address >= 0x04000130 && req.address <= 0x04000133 {
            self.keypad.write(req.address, req.data, req.mas);
        } else if req.address >= 0x0e000000 {
            self.gamepak_sram
                .write(req.address & 0xffff | 0x0e000000, req.data, req.mas);
        } else {
            todo!("writing to {:#08x}", req.address);
        }

        return rsp;
    }
}
