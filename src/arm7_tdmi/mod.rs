mod instruction;
mod register_file;

use crate::arm7_tdmi::instruction::{decode_arm, ArmInstructionType};
use crate::arm7_tdmi::register_file::{ConditionCodeFlag, RegisterFile};
use crate::bus::{BusCycle, BusSignal, MemoryRequest, MemoryResponse, TransferSize};
use crate::common::BitOperation;
use std::collections::VecDeque;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum InstructionStep {
    STEP0,
    STEP1,
    STEP2,
    STEP3,
    STEP4,
    STEP5,
    STEP6,
    STEP7,
}

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
    arm_instruction_queue: VecDeque<u32>,
    arm_current_execute: u32,
    instruction_step: InstructionStep,
    data_is_fetch: bool,
    data_is_reading: bool,
}

impl ARM7TDMI {
    /// ARM7TDMI::new
    pub fn new() -> Self {
        Self {
            rf: RegisterFile::new(),
            arm_instruction_queue: VecDeque::from([0xe1a00000]),
            arm_current_execute: 0xe1a00000,
            data_is_fetch: false,
            data_is_reading: false,
            instruction_step: InstructionStep::STEP0,
        }
    }

    /// ARM7TDMI::step
    ///
    /// Corresponds to one clock cycle for the cpu.
    ///
    /// @param [MemoryResponse]: response from the bus to a previous request of the cpu.
    /// @return [MemoryRequest]: request from the cpu towards the bus.
    pub fn step(&mut self, rsp: MemoryResponse) -> MemoryRequest {
        // Build request to fetch new instruction. If the current execute stage requires the usage
        // of the memory, then the data will be overridden, otherwise it will be used to access the
        // memory.
        let mut next_request = MemoryRequest {
            address: self.rf.get_register(15) + 8, // Implements only arm mode
            nr_w: BusSignal::LOW,                  // Read operation
            mas: TransferSize::WORD,               // Reads 32 bits
            n_opc: BusSignal::LOW,                 // Requires an opcode
            n_trans:                               // Whether we are priviliged
                if self.rf.get_mode() == OperatingMode::USER {
                    BusSignal::LOW
                } else {
                    BusSignal::HIGH
                },
            lock: BusSignal::LOW,            // No swap opeartion
            t_bit: BusSignal::LOW,           // arm mode
            bus_cycle: BusCycle::SEQUENTIAL, // bus cycle is sequential
        };

        // Memory request is not completed, and the cpu must stall
        if rsp.n_wait == BusSignal::LOW {
            return next_request;
        }

        if self.data_is_reading && self.data_is_fetch {
            self.arm_instruction_queue.push_back(rsp.data);
        }

        println!(
            "Current executed instruction: {:#08x}",
            self.arm_current_execute
        );

        self.data_is_reading = true;
        self.data_is_fetch = true;

        match decode_arm(self.arm_current_execute) {
            ArmInstructionType::DataProcessing => self.arm_data_processing(&mut next_request, rsp),
            ArmInstructionType::Multiply => todo!(),
            ArmInstructionType::MultiplyLong => todo!(),
            ArmInstructionType::SingleDataSwap => todo!(),
            ArmInstructionType::BranchAndExchange => todo!(),
            ArmInstructionType::HwTrasferReg => todo!(),
            ArmInstructionType::HwTransferImmediate => todo!(),
            ArmInstructionType::SingleDataTransfer => todo!(),
            ArmInstructionType::Undefined => todo!(),
            ArmInstructionType::BlockDataTransfer => todo!(),
            ArmInstructionType::Branch => todo!(),
            ArmInstructionType::CoprocessorDataTransfer => todo!(),
            ArmInstructionType::CoprocessorDataOperation => todo!(),
            ArmInstructionType::CoprocessorRegisterTransfer => todo!(),
            ArmInstructionType::SoftwareInterrupt => todo!(),
            ArmInstructionType::Unimplemented => todo!(),
        }

        // The current instruction is done executing: move to the next instruction by popping the
        // front of the queue and updating the program counter
        if self.instruction_step == InstructionStep::STEP0 {
            self.arm_current_execute = self.arm_instruction_queue.pop_front().unwrap();
            self.rf.write_register(15, self.rf.get_register(15) + 4);
        }

        next_request
    }

    fn arm_data_processing(&mut self, req: &mut MemoryRequest, rsp: MemoryResponse) {

        let rd = self.arm_current_execute.get_range(15, 12);
        println!("we are in {:?}", self.instruction_step);

        if self.instruction_step == InstructionStep::STEP0 {

            // Decode instruction
            let opcode = self.arm_current_execute.get_range(24, 21);
            let i_flag = self.arm_current_execute.get_range(25, 25);
            let condition = self.arm_current_execute.get_range(31, 28);
            let s_flag = self.arm_current_execute.get_range(20, 20);
            let rn = self.arm_current_execute.get_range(19, 16);
            let mut shift_amount = self.arm_current_execute.get_range(11, 7);
            let rs = self.arm_current_execute.get_range(11, 8);
            let shift_type = self.arm_current_execute.get_range(6, 5);
            let r_flag = self.arm_current_execute.get_range(4, 4);
            let rm = self.arm_current_execute.get_range(3, 0);
            let is = self.arm_current_execute.get_range(11, 8);
            let nn = self.arm_current_execute.get_range(7, 0);

            let mut operand2 = 0;
            let mut there_is_shift = false;

            if !self.rf.check_condition_code(condition) {
                return;
            }

            if i_flag == 1 {
                operand2 = nn;
                if is != 0 {
                    operand2 = operand2.rotate_right(is * 2);
                    there_is_shift = true;
                }
            } else {
                there_is_shift = true;
                if r_flag == 1 {
                    shift_amount = self.rf.get_register(rs).get_range(7, 0);
                }
                operand2 = self.rf.get_register(rm);

                if shift_type == 0b00 {
                    if shift_amount != 0 {
                        operand2 = operand2.wrapping_shl(shift_amount);
                    }
                    else{
                        there_is_shift = false;
                    }
                } else if shift_type == 0b01 {
                    if shift_amount == 0 {
                        operand2 = 0;
                    }
                    else {
                        operand2 = operand2.wrapping_shr(shift_amount);
                    }
                } else if shift_type == 0b10 {
                    if shift_amount == 0 {
                        operand2 = 0;
                    }
                    else {
                        operand2 = operand2.wrapping_shr(shift_amount);
                    }

                } else {

                }
            }

            let result_alu = self.alu_operation(self.rf.get_register(rn), operand2, opcode);
            if opcode < 0x8 && opcode > 0xb {
                self.rf.write_register(rd, result_alu);
            }
            if (opcode >= 0x8 && opcode <= 0xb) || s_flag == 1 {
                self.update_flags(result_alu);
            }

            if there_is_shift {
                req.bus_cycle = BusCycle::INTERNAL;
                self.data_is_fetch = false;
                self.instruction_step = InstructionStep::STEP1;
            } else if rd == 15 {
                self.arm_instruction_queue.clear();
                req.address = self.rf.get_register(15);
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.instruction_step = InstructionStep::STEP2;
            }
        } else if self.instruction_step == InstructionStep::STEP1 {
            if rd == 15 {
                req.address = self.rf.get_register(15);
                self.arm_instruction_queue.clear();
                self.instruction_step = InstructionStep::STEP2;
            } else {
                self.instruction_step = InstructionStep::STEP0;
            }
        } else if self.instruction_step == InstructionStep::STEP2 {
            req.address = self.rf.get_register(15) + 4;
            self.instruction_step = InstructionStep::STEP3;
        } else if self.instruction_step == InstructionStep::STEP3 {
            req.address = self.rf.get_register(15) + 8;
            self.instruction_step = InstructionStep::STEP0;
        } else {
            panic!("Wrong step for instructin type ARM_DATA_PROCESSING");
        }
    }

    fn alu_operation(&self, operand1: u32, operand2: u32, opcode: u32) -> u32 {
        use ConditionCodeFlag::*;
        match opcode {
            0x0 => operand1 & operand2,
            0x1 => operand1 ^ operand2,
            0x2 => operand1 - operand2,
            0x3 => operand2 - operand1,
            0x4 => operand1 + operand2,
            0x5 => operand1 + operand2 + self.rf.is_flag_set(&C) as u32,
            0x6 => operand1 - operand2 + self.rf.is_flag_set(&C) as u32,
            0x7 => operand2 - operand1 + self.rf.is_flag_set(&C) as u32,
            0x8 => operand1 & operand2,
            0x9 => operand1 ^ operand2,
            0xa => operand1 - operand2,
            0xb => operand1 + operand2,
            0xc => operand1 | operand2,
            0xd => operand2,
            0xe => operand1 & !operand2,
            0xf => !operand2,
            _ => panic!("Invalid opcode: got {:#01x}", opcode),
        }
    }

    fn update_flags(&self, alu_result: u32) {}
}

#[test]
fn test_cpu() {
    let mut cpu = ARM7TDMI::new();

    for _i in 0..5 {
        let response = MemoryResponse {
            data: 0xe0844005,
            n_wait: BusSignal::HIGH,
        };
        let req = cpu.step(response);
        println!("requiring: {:#08x}", req.address);
    }

    assert_eq!(1, 0);
}
