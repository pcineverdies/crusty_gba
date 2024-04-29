mod cpu_test;
mod instruction;
mod register_file;

use crate::arm7_tdmi::instruction::{
    decode_arm, ArmAluOpcode, ArmAluShiftCodes, ArmInstructionType,
};
use crate::arm7_tdmi::register_file::{ConditionCodeFlag, RegisterFile};
use crate::bus::{BusCycle, BusSignal, MemoryRequest, MemoryResponse, TransferSize};
use crate::common::BitOperation;
use std::collections::VecDeque;

const NOP: u32 = 0xE1A00000_u32;

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
            arm_instruction_queue: VecDeque::from([]),
            arm_current_execute: NOP,
            data_is_fetch: true,
            data_is_reading: true,
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

        if self.instruction_step == InstructionStep::STEP0 {
            // Decode instruction
            let opcode = ArmAluOpcode::from_value(self.arm_current_execute.get_range(24, 21));
            let s_flag = self.arm_current_execute.get_range(20, 20);
            let i_flag = self.arm_current_execute.get_range(25, 25);
            let condition = self.arm_current_execute.get_range(31, 28);
            let rn = self.arm_current_execute.get_range(19, 16);
            let mut shift_amount = self.arm_current_execute.get_range(11, 7);
            let rs = self.arm_current_execute.get_range(11, 8);
            let shift_type = num::FromPrimitive::from_u32(self.arm_current_execute.get_range(6, 5));
            let r_flag = self.arm_current_execute.get_range(4, 4);
            let rm = self.arm_current_execute.get_range(3, 0);
            let is = self.arm_current_execute.get_range(11, 8);
            let nn = self.arm_current_execute.get_range(7, 0);

            let mut operand2;
            let mut next_to_write;
            let mut carry_output;
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

                match shift_type {
                    Some(ArmAluShiftCodes::LSL) => {
                        if shift_amount != 0 {
                            operand2 = operand2.wrapping_shl(shift_amount);
                        } else {
                            there_is_shift = false;
                        }
                    }
                    Some(ArmAluShiftCodes::LSR) => {
                        operand2 = operand2.wrapping_shr(shift_amount);
                        operand2 = if shift_amount == 0 { 0 } else { shift_amount };
                    }
                    Some(ArmAluShiftCodes::ASR) => {
                        operand2 = (operand2 as i32).wrapping_shr(shift_amount) as u32;
                        operand2 = if shift_amount == 0 { 0 } else { shift_amount };
                    }
                    Some(ArmAluShiftCodes::ROR) => {
                        shift_amount = if shift_amount == 0 { 1 } else { shift_amount };
                        operand2.rotate_right(shift_amount);
                    }
                    None => {
                        panic!("Invalid shift type");
                    }
                }
            }

            (next_to_write, carry_output) = self.alu_operation(operand1, operand2, opcode);
            if !ArmAluOpcode::is_test_opcode(opcode) {
                self.rf.write_register(rd, next_to_write);
            }
            if ArmAluOpcode::is_test_opcode(opcode) || s_flag == 1 {
                self.update_flags(next_to_write, opcode, rd, carry_output);
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
            self.rf.write_register(15, self.rf.get_register(15) - 4);
            self.instruction_step = InstructionStep::STEP0;
        } else {
            panic!("Wrong step for instructin type ARM_DATA_PROCESSING");
        }
    }

    fn alu_operation(&self, operand1: u32, operand2: u32, opcode: ArmAluOpcode) -> (u32, bool) {
        use ArmAluOpcode::*;
        use ConditionCodeFlag::*;
        match opcode {
            AND => (operand1 & operand2, false),
            EOR => (operand1 ^ operand2, false),
            SUB => operand1.overflowing_sub(operand2),
            RSB => operand2.overflowing_sub(operand1),
            ADD => operand1.overflowing_add(operand2),
            ADC => operand1.carrying_add(operand2, self.rf.is_flag_set(&C)),
            SBC => operand1.borrowing_sub(operand2, self.rf.is_flag_set(&C)),
            RSC => operand2.borrowing_sub(operand1, self.rf.is_flag_set(&C)),
            TST => (operand1 & operand2, false),
            TEQ => (operand1 ^ operand2, false),
            CMP => (operand1 - operand2, false),
            CMN => (operand1 + operand2, false),
            ORR => (operand1 | operand2, false),
            MOV => (operand2, false),
            BIC => (operand1 & !operand2, false),
            MNV => (!operand2, false),
        }
    }

    fn update_flags(&mut self, alu_result: u32, opcode: ArmAluOpcode, rd: u32, carry_output: bool) {
        if rd != 15 {
            self.rf.write_z(alu_result == 0);
            self.rf.write_n(alu_result.is_bit_set(31));
            if ArmAluOpcode::is_logical(opcode) {
                // TODO: Something about V flag
                // TODO: Something about C flag
            }
            if ArmAluOpcode::is_arithmetic(opcode) {
                // TODO: Something about V flag
                // TODO: Something about C flag
            }
        }
    }
}
