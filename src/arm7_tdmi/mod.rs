mod instruction;
mod register_file;

use crate::bus::{MemoryRequest, MemoryResponse};

/// arm7_tdmi::OpeartingMode
///
/// enum to represent the different operating modes that the cpu
/// can be into, with respect to [manual, 2.7].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u32)]
enum OperatingMode {
    SYSTEM = 0b10000,
    USER = 0b11111,
    FIQ = 0b10001,
    IRQ = 0b10010,
    SUPERVISOR = 0b10011,
    ABORT = 0b10111,
    UND = 0b11011,
}

/// arm7_tdmi::ARM7TDMI
///
/// structure to represent the arm cpu
pub struct ARM7TDMI {
    rf: register_file::RegisterFile,
    is_requesting_data: bool,
}

impl ARM7TDMI {
    /// ARM7TDMI::step
    ///
    /// Corresponds to one clock cycle for the cpu.
    ///
    /// @param [Option<MemoryResponse>]: possible response from the bus
    /// to a previous request by the cpu. If the cpu is waiting for a response
    /// but this value is None, then the cpu stalls for one clock cycle
    ///
    /// @return [Option<MemoryRequest>]: at each clock cycle, the cpu might
    /// require a data from the memory which should be received in the following
    /// cycles. The information about this request are encoded in here.
    pub fn step(&self, rsp: Option<MemoryResponse>) -> Option<MemoryRequest> {
        None
    }
}
