mod arm_instructions;
mod cpu_test;
mod instruction;
mod register_file;
mod thumb_instructions;

use crate::arm7_tdmi::instruction::{
    decode_arm, decode_thumb, ArmInstructionType, ThumbInstructionType,
};
use crate::arm7_tdmi::register_file::RegisterFile;
use crate::bus::{BusCycle, BusSignal, MemoryRequest, MemoryResponse, TransferSize};
use crate::common::BitOperation;
use std::collections::VecDeque;

/// Definition of a NOP instruction used to initialize the CPU
pub const NOP: u32 = 0xE1A00000_u32;
pub const NOP_THUMB: u32 = 0x000046c0_u32;

/// arm7_tdmi::InstructionStep
///
/// Many of the instructions require an execute stage which is longer than one cycle. Each
/// instruction is thus implemented using an FSM handling the different states using a variable of
/// this type.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InstructionStep {
    STEP0,
    STEP1,
    STEP2,
    STEP3,
    STEP4,
}

/// arm7_tdmi::OpeartingMode
///
/// enum to represent the different operating modes that the cpu can be into, with respect to
/// [manual, 2.7].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u32)]
pub enum OperatingMode {
    SYSTEM = 0b11111,
    USER = 0b10000,
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
    pub rf: register_file::RegisterFile,   // Register File
    arm_instruction_queue: VecDeque<u32>,  // Instruction queue
    pub arm_current_execute: u32,          // Current executed instruction
    pub instruction_step: InstructionStep, // Current instructions stpe for FSM handling
    data_is_fetch: bool,                   // Is next data a fetch?
    last_used_address: u32,                // Store the last address sent on the bus
    instruction_counter_step: u32,         // For instructions which require many iterations
    list_transfer_op: Vec<(u32, u32)>,     // List of operations to perform for ldm and stm
}

impl ARM7TDMI {
    /// ARM7TDMI::new
    ///
    /// Instantiates a cpu with default parameters
    pub fn new() -> Self {
        Self {
            rf: RegisterFile::new(),
            arm_instruction_queue: VecDeque::from([]),
            arm_current_execute: NOP,
            data_is_fetch: true,
            instruction_step: InstructionStep::STEP0,
            last_used_address: 0,
            instruction_counter_step: 0,
            list_transfer_op: Vec::new(),
        }
    }

    /// ARM7TDMI::step
    ///
    /// Corresponds to one clock cycle for the cpu.
    ///
    /// @param [MemoryResponse]: response from the bus to a previous request of the cpu.
    /// @return [MemoryRequest]: request from the cpu towards the bus.
    pub fn step(&mut self, rsp: MemoryResponse) -> MemoryRequest {
        let thumb_mode_active = self.rf.is_thumb_mode();

        // Build request to fetch new instruction. If the current execute stage requires the usage
        // of the memory, then the data will be overridden, otherwise it will be used to access the
        // memory.
        let mut next_request = MemoryRequest {
            address: if thumb_mode_active {
                self.rf.get_register(15, 4)
            } else {
                self.rf.get_register(15, 8)
            },
            nr_w: BusSignal::LOW,                               // Read operation
            mas: if thumb_mode_active{                          // Size of transfer
                TransferSize::HALFWORD
            } else {
                TransferSize::WORD
            },
            n_opc: BusSignal::LOW,                              // Requires an opcode
            data: 0,
            n_trans:                                            // Whether we are priviliged
                if self.rf.get_mode() == OperatingMode::USER {
                    BusSignal::LOW
                } else {
                    BusSignal::HIGH
                },
            lock: BusSignal::LOW,                               // No swap opeartion
            t_bit: if thumb_mode_active {                       // Select current mode
                BusSignal::HIGH
            } else {
                BusSignal::LOW
            },
            bus_cycle: BusCycle::SEQUENTIAL,                    // bus cycle is sequential
        };

        // Memory request is not completed, and the cpu must stall
        if rsp.n_wait == BusSignal::LOW {
            return next_request;
        }

        // A fetch was in progress: add the data to the instruction queue
        if self.data_is_fetch {
            if !thumb_mode_active {
                self.arm_instruction_queue.push_back(rsp.data);
            } else {
                if self.last_used_address.is_bit_clear(1) {
                    self.arm_instruction_queue
                        .push_back(rsp.data.get_range(15, 0));
                } else {
                    self.arm_instruction_queue
                        .push_back(rsp.data.get_range(31, 16));
                }
            }
        }

        self.data_is_fetch = true;

        if !thumb_mode_active {
            match decode_arm(self.arm_current_execute) {
                ArmInstructionType::DataProcessing => self.arm_data_processing(&mut next_request),
                ArmInstructionType::BranchAndExchange => {
                    self.arm_branch_and_exchange(&mut next_request, &rsp)
                }
                ArmInstructionType::SingleDataTransfer => {
                    self.arm_single_data_transfer(&mut next_request, &rsp)
                }
                ArmInstructionType::Branch => self.arm_branch(&mut next_request),
                ArmInstructionType::HwTransfer => self.arm_hw_transfer(&mut next_request, &rsp),
                ArmInstructionType::SoftwareInterrupt => self.arm_swi(&mut next_request),
                ArmInstructionType::Undefined => self.arm_undefined(&mut next_request),
                ArmInstructionType::PsrTransferMRS => self.arm_psr_transfer_mrs(),
                ArmInstructionType::PsrTransferMSR => self.arm_psr_transfer_msr(),
                ArmInstructionType::SingleDataSwap => {
                    self.arm_single_data_swap(&mut next_request, &rsp)
                }
                ArmInstructionType::BlockDataTransfer => {
                    self.arm_block_data_transfer(&mut next_request, &rsp)
                }
                ArmInstructionType::Multiply => self.arm_multiply(&mut next_request),
                ArmInstructionType::Unimplemented => panic!(
                    "The instruction {:#08x} at address {:#08x} is not implemented and it should not be used",
                    self.arm_current_execute,
                    self.rf.get_register(15, 0)
                ),

                ArmInstructionType::CoprocessorDataTransfer => {
                    panic!("Coprocessor data transfer instructions are not implemented");
                }
                ArmInstructionType::CoprocessorDataOperation => {
                    panic!("Coprocessor data operation instructions are not implemented");
                }
                ArmInstructionType::CoprocessorRegisterTransfer => {
                    panic!("Coprocessor register transfer instructions are not implemented");
                }
            }
        } else {
            match decode_thumb(self.arm_current_execute) {
                ThumbInstructionType::MoveShiftedRegister => {
                    self.thumb_move_shifter_register(&mut next_request)
                }
                ThumbInstructionType::AddSubtract => self.thumb_add_subtract(&mut next_request),
                ThumbInstructionType::AluImmediate => self.thumb_alu_immediate(&mut next_request),
                ThumbInstructionType::Alu => self.thumb_alu(&mut next_request),
                ThumbInstructionType::HiRegisterBx => {
                    self.thumb_hi_register_bx(&mut next_request, &rsp)
                }
                ThumbInstructionType::PcRelativeLoad => {
                    self.thumb_pc_relative_load(&mut next_request, &rsp)
                }
                ThumbInstructionType::LoadStoreRegOffset => {
                    self.thumb_load_store_reg_offset(&mut next_request, &rsp)
                }
                ThumbInstructionType::LoadStoreSignExt => {
                    self.thumb_load_store_sign_ext(&mut next_request, &rsp)
                }
                ThumbInstructionType::LoadStoreImmOffset => {
                    self.thumb_load_store_imm_offset(&mut next_request, &rsp)
                }
                ThumbInstructionType::LoadStoreHalfWord => {
                    self.thumb_load_store_halfword(&mut next_request, &rsp)
                }
                ThumbInstructionType::SpRelativeLoadStore => {
                    self.thumb_sp_relative_load_store(&mut next_request, &rsp)
                }
                ThumbInstructionType::LoadAddress => self.thumb_load_address(&mut next_request),
                ThumbInstructionType::AddOffsetToSp => {
                    self.thumb_add_offset_to_sp(&mut next_request)
                }
                ThumbInstructionType::PushPopRegister => {
                    self.thumb_push_pop_register(&mut next_request, &rsp)
                }
                ThumbInstructionType::MultipleLoadStore => {
                    self.thumb_multiple_load_store(&mut next_request, &rsp)
                }
                ThumbInstructionType::ConditionalBranch => {
                    self.thumb_branch(&mut next_request, true)
                }
                ThumbInstructionType::SoftwareInterrupt => {
                    self.thumb_software_interrupt(&mut next_request)
                }
                ThumbInstructionType::UncoditionalBranch => {
                    self.thumb_branch(&mut next_request, false)
                }
                ThumbInstructionType::LongBranchWithLink => {
                    self.thumb_long_branch_with_link(&mut next_request)
                }
            }
        }

        // The current instruction is done executing: move to the next instruction by popping the
        // front of the queue and updating the program counter
        if self.instruction_step == InstructionStep::STEP0 {
            self.arm_current_execute = self.arm_instruction_queue.pop_front().unwrap();

            // Arm mode in the current value of cpsr
            if !self.rf.get_cpsr().is_bit_set(5) {
                self.rf.write_register(15, self.rf.get_register(15, 4));
            // Thumb mode in the current value of cpsr
            } else {
                self.rf.write_register(15, self.rf.get_register(15, 2));
            }
        }

        // Always remember the address which was used in the last bus transaction. This is useful
        // for the execution of many instructions handling memory.
        self.last_used_address = next_request.address;
        next_request
    }
}
