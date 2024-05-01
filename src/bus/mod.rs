#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
#[repr(u8)]
pub enum TransferSize {
    BYTE = 0,
    HALFWORD = 1,
    #[default]
    WORD = 2,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
#[repr(u8)]
pub enum BusCycle {
    #[default]
    NONSEQUENTIAL = 0,
    SEQUENTIAL = 1,
    INTERNAL = 2,
    COPROCESSOR = 3,
}

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
