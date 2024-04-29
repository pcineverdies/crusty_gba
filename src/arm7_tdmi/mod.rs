mod instruction;
mod register_file;

use crate::arm7_tdmi::instruction::{decode_arm, ArmInstructionType};
use crate::arm7_tdmi::register_file::{ConditionCodeFlag, RegisterFile};
use crate::bus::{BusCycle, BusSignal, MemoryRequest, MemoryResponse, TransferSize};
use crate::common::BitOperation;
use std::collections::VecDeque;

const NOP : u32 = 0xE1A00000_u32;

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

    next_to_write: u32,
    carry_output: bool,
}

impl ARM7TDMI {
    /// ARM7TDMI::new
    pub fn new() -> Self {
        Self {
            rf: RegisterFile::new(),
            arm_instruction_queue: VecDeque::from([]),
            arm_current_execute: NOP,
            data_is_fetch: true,
            data_is_reading: true,
            instruction_step: InstructionStep::STEP0,
            next_to_write: 0,
            carry_output: false,
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
            println!(
                "Puntting into queue instructiona at address {:#08x}",
                self.rf.get_register(15)
            );
            self.arm_current_execute = self.arm_instruction_queue.pop_front().unwrap();
            self.rf.write_register(15, self.rf.get_register(15) + 4);
        }

        next_request
    }

    fn arm_data_processing(&mut self, req: &mut MemoryRequest, rsp: MemoryResponse) {
        let rd = self.arm_current_execute.get_range(15, 12);
        let opcode = self.arm_current_execute.get_range(24, 21);
        let s_flag = self.arm_current_execute.get_range(20, 20);

        println!("we are in {:?}", self.instruction_step);

        if self.instruction_step == InstructionStep::STEP0 {
            // Decode instruction
            let i_flag = self.arm_current_execute.get_range(25, 25);
            let condition = self.arm_current_execute.get_range(31, 28);
            let rn = self.arm_current_execute.get_range(19, 16);
            let mut shift_amount = self.arm_current_execute.get_range(11, 7);
            let rs = self.arm_current_execute.get_range(11, 8);
            let shift_type = self.arm_current_execute.get_range(6, 5);
            let r_flag = self.arm_current_execute.get_range(4, 4);
            let rm = self.arm_current_execute.get_range(3, 0);
            let is = self.arm_current_execute.get_range(11, 8);
            let nn = self.arm_current_execute.get_range(7, 0);

            let mut operand2;
            let mut operand1 = self.rf.get_register(rn);
            let mut there_is_shift = false;

            if !self.rf.check_condition_code(condition) {
                return;
            }

            if i_flag == 1 {
                operand2 = nn;
                operand1 += if rn == 15 { 8 } else { 0 };
                if is != 0 {
                    operand2 = operand2.rotate_right(is * 2);
                    there_is_shift = true;
                }
            } else {
                there_is_shift = true;
                operand2 = self.rf.get_register(rm);
                if r_flag == 1 {
                    shift_amount = self.rf.get_register(rs).get_range(7, 0);
                    operand2 += if rm == 15 { 12 } else { 0 };
                    operand1 += if rn == 15 { 12 } else { 0 };
                } else {
                    operand2 += if rm == 15 { 12 } else { 0 };
                    operand1 += if rn == 15 { 12 } else { 0 };
                }

                if shift_type == 0b00 {
                    if shift_amount != 0 {
                        operand2 = operand2.wrapping_shl(shift_amount);
                    } else {
                        there_is_shift = false;
                    }
                } else if shift_type == 0b01 {
                    operand2 = if shift_amount == 0 {
                        0
                    } else {
                        operand2.wrapping_shr(shift_amount)
                    };
                } else if shift_type == 0b10 {
                    operand2 = if shift_amount == 0 {
                        0
                    } else {
                        (operand2 as i32).wrapping_shr(shift_amount) as u32
                    };
                } else {
                    shift_amount = if shift_amount == 0 { 1 } else { shift_amount };
                    operand2 = operand2.rotate_right(shift_amount);
                }
            }

            (self.next_to_write, self.carry_output) =
                self.alu_operation(operand1, operand2, opcode);
            if (opcode < 0x8 || opcode > 0xb) && !there_is_shift {
                self.rf.write_register(rd, self.next_to_write);
            }
            if ((opcode >= 0x8 && opcode <= 0xb) || s_flag == 1) && !there_is_shift {
                self.update_flags(self.next_to_write, opcode, rd, self.carry_output);
            }

            if there_is_shift {
                req.bus_cycle = BusCycle::INTERNAL;
                self.data_is_fetch = false;
                self.instruction_step = InstructionStep::STEP1;
            } else if rd == 15 {
                self.arm_instruction_queue.clear();
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.data_is_fetch = false;
                self.instruction_step = InstructionStep::STEP2;
            }
        } else if self.instruction_step == InstructionStep::STEP1 {
            if opcode < 0x8 || opcode > 0xb {
                self.rf.write_register(rd, self.next_to_write);
            }
            if (opcode >= 0x8 && opcode <= 0xb) || s_flag == 1 {
                self.update_flags(self.next_to_write, opcode, rd, self.carry_output);
            }

            if rd == 15 {
                self.arm_instruction_queue.clear();
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.data_is_fetch = false;
                self.instruction_step = InstructionStep::STEP2;
            } else {
                self.instruction_step = InstructionStep::STEP0;
            }
        } else if self.instruction_step == InstructionStep::STEP2 {
            req.address = self.rf.get_register(15);
            self.instruction_step = InstructionStep::STEP3;
        } else if self.instruction_step == InstructionStep::STEP3 {
            req.address = self.rf.get_register(15) + 4;
            self.instruction_step = InstructionStep::STEP0;
        } else {
            panic!("Wrong step for instructin type ARM_DATA_PROCESSING");
        }
    }

    fn alu_operation(&self, operand1: u32, operand2: u32, opcode: u32) -> (u32, bool) {
        use ConditionCodeFlag::*;
        match opcode {
            0x0 => (operand1 & operand2, false),
            0x1 => (operand1 ^ operand2, false),
            0x2 => operand1.overflowing_sub(operand2),
            0x3 => operand2.overflowing_sub(operand1),
            0x4 => operand1.overflowing_add(operand2),
            0x5 => operand1.carrying_add(operand2, self.rf.is_flag_set(&C)),
            0x6 => operand1.borrowing_sub(operand2, self.rf.is_flag_set(&C)),
            0x7 => operand2.borrowing_sub(operand1, self.rf.is_flag_set(&C)),
            0x8 => (operand1 & operand2, false),
            0x9 => (operand1 ^ operand2, false),
            0xa => (operand1 - operand2, false),
            0xb => (operand1 + operand2, false),
            0xc => (operand1 | operand2, false),
            0xd => (operand2, false),
            0xe => (operand1 & !operand2, false),
            0xf => (!operand2, false),
            _ => panic!("Invalid opcode: got {:#01x}", opcode),
        }
    }

    fn update_flags(&mut self, alu_result: u32, opcode: u32, rd: u32, carry_output: bool) {
        if rd != 15 {
            self.rf.write_Z(alu_result == 0);
            self.rf.write_N(alu_result.is_bit_set(31));
            if (opcode <= 1) || (opcode >= 8 && opcode <= 9) || opcode >= 0xc {
                // TODO: Something about V flag
                // TODO: Something about C flag
            } else {
                // TODO: Something about V flag
                self.rf.write_C(carry_output);
            }
        }
    }
}

#[test]
fn data_processing_test() {
    use std::collections::HashMap;
    let mut cpu = ARM7TDMI::new();

    let instructions = HashMap::from([
        (0x08000000_u32, 0xe2821010_u32),
        (0x08000004_u32, 0xe1a02001_u32),
        (0x08000008_u32, 0xe3a03011_u32),
        (0x0800000c_u32, 0xe0033002_u32),
        (0x08000010_u32, 0xe083f001_u32),
        (0x00000020_u32, 0xe3a03003_u32),
    ]);

    let mut response = MemoryResponse {
        data: NOP,
        n_wait: BusSignal::HIGH,
    };

    for i in 0..100 {
        let req = cpu.step(response);
        println!("requiring: {:#08x}", req.address);
        response.data = *instructions.get(&req.address).unwrap_or(&NOP);
        println!("=======");
    }

    assert_eq!(cpu.rf.get_register(3), 3);
}
