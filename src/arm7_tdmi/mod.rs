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

use self::instruction::barrel_shifter;

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
            ArmInstructionType::BranchAndExchange => {
                self.arm_branch_and_exchange(&mut next_request, rsp)
            }
            ArmInstructionType::HwTrasferReg => todo!(),
            ArmInstructionType::HwTransferImmediate => todo!(),
            ArmInstructionType::SingleDataTransfer => todo!(),
            ArmInstructionType::Undefined => todo!(),
            ArmInstructionType::BlockDataTransfer => todo!(),
            ArmInstructionType::Branch => self.arm_branch(&mut next_request, rsp),
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
            let shift_type = self.arm_current_execute.get_range(6, 5);
            let r_flag = self.arm_current_execute.get_range(4, 4);
            let rm = self.arm_current_execute.get_range(3, 0);
            let is = self.arm_current_execute.get_range(11, 8);
            let nn = self.arm_current_execute.get_range(7, 0);

            let mut operand2;

            let mut carry_shifter = self.rf.is_flag_set(&ConditionCodeFlag::C);
            let mut operand1 = self.rf.get_register(rn);
            let mut there_is_shift = false;

            if !self.rf.check_condition_code(condition) {
                return;
            }

            if i_flag == 1 {
                operand1 += if rn == 15 { 8 } else { 0 };
                operand2 = nn.rotate_right(is * 2);
            } else {
                there_is_shift = true;
                operand2 = self.rf.get_register(rm);
                if r_flag == 1 {
                    if rs == 15 {
                        panic!("Cannot use r15 as rs register in ALU operations");
                    }
                    shift_amount = self.rf.get_register(rs).get_range(7, 0);
                    operand1 += if rn == 15 { 12 } else { 0 };
                    operand2 += if rm == 15 { 12 } else { 0 };
                } else {
                    operand1 += if rn == 15 { 8 } else { 0 };
                    operand2 += if rm == 15 { 8 } else { 0 };
                }

                (operand2, carry_shifter, there_is_shift) = barrel_shifter(
                    operand2,
                    shift_type,
                    shift_amount,
                    self.rf.is_flag_set(&ConditionCodeFlag::C),
                );
            }

            let (next_to_write, c_output, v_output) =
                self.alu_operation(operand1, operand2, opcode);
            if !ArmAluOpcode::is_test_opcode(opcode) {
                self.rf.write_register(rd, next_to_write);
            }
            if ArmAluOpcode::is_test_opcode(opcode) || s_flag == 1 {
                self.update_flags(next_to_write, opcode, rd, c_output, carry_shifter, v_output);
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

    fn alu(&self, operand1: u32, operand2: u32, opcode: ArmAluOpcode) -> (u32, bool, bool) {
        use ArmAluOpcode::*;
        use ConditionCodeFlag::*;

        let (mut alu_result, mut v_output, mut c_output) = (0, false, false);
        let (mut op1, mut op2, mut c_in);

        if opcode == SUB || opcode == CMP {
            op1 = (operand1 as i32) as i64;
            op2 = (operand2 as i32) as i64;
            alu_result = op1 - op2;
            v_output =
                (op1 >= 0 && op2 < 0 && alu_result < 0) || (op1 < 0 && op2 >= 0 && alu_result >= 0);
        } else if opcode == RSB {
            op1 = (operand2 as i32) as i64;
            op2 = (operand1 as i32) as i64;
            alu_result = op1 - op2;
            v_output =
                (op1 >= 0 && op2 < 0 && alu_result < 0) || (op1 < 0 && op2 >= 0 && alu_result >= 0);
        } else if opcode == ADD || opcode == CMN {
            op1 = (operand2 as i32) as i64;
            op2 = (operand1 as i32) as i64;
            alu_result = op1 + op2;
            v_output =
                (op1 >= 0 && op2 >= 0 && alu_result < 0) || (op1 < 0 && op2 < 0 && alu_result >= 0);
        } else if opcode == ADC {
            op1 = (operand2 as i32) as i64;
            op2 = (operand1 as i32) as i64;
            c_in = (if self.rf.is_flag_set(&C) { 1 } else { 0 }) as i64;
            alu_result = op1 + op2 + c_in;
            v_output =
                (op1 >= 0 && op2 >= 0 && alu_result < 0) || (op1 < 0 && op2 < 0 && alu_result >= 0);
        } else if opcode == SBC {
            op1 = (operand2 as i32) as i64;
            op2 = (operand1 as i32) as i64;
            c_in = (if self.rf.is_flag_set(&C) { 1 } else { 0 }) as i64;
            alu_result = op1 - op2 + c_in - 1;
            v_output =
                (op1 >= 0 && op2 < 0 && alu_result < 0) || (op1 < 0 && op2 >= 0 && alu_result >= 0);
        } else if opcode == RSC {
            op1 = (operand2 as i32) as i64;
            op2 = (operand1 as i32) as i64;
            c_in = (if self.rf.is_flag_set(&C) { 1 } else { 0 }) as i64;
            alu_result = op2 - op1 + c_in - 1;
            v_output =
                (op1 >= 0 && op2 < 0 && alu_result < 0) || (op1 < 0 && op2 >= 0 && alu_result >= 0);
        } else {
            panic!("Wrong argument `opcode` for alu")
        }

        c_output = (alu_result as u64).is_bit_set(32);

        let alu_result = (alu_result as u64).get_range(31, 0) as u32;
        return (alu_result, c_output, v_output);
    }

    fn alu_operation(
        &self,
        operand1: u32,
        operand2: u32,
        opcode: ArmAluOpcode,
    ) -> (u32, bool, bool) {
        use ArmAluOpcode::*;
        match opcode {
            AND => (operand1 & operand2, false, false),
            EOR => (operand1 ^ operand2, false, false),
            SUB => self.alu(operand1, operand2, opcode),
            RSB => self.alu(operand1, operand2, opcode),
            ADD => self.alu(operand1, operand2, opcode),
            ADC => self.alu(operand1, operand2, opcode),
            SBC => self.alu(operand1, operand2, opcode),
            RSC => self.alu(operand1, operand2, opcode),
            TST => (operand1 & operand2, false, false),
            TEQ => (operand1 ^ operand2, false, false),
            CMP => self.alu(operand1, operand2, opcode),
            CMN => self.alu(operand1, operand2, opcode),
            ORR => (operand1 | operand2, false, false),
            MOV => (operand2, false, false),
            BIC => (operand1 & !operand2, false, false),
            MNV => (!operand2, false, false),
        }
    }

    fn update_flags(
        &mut self,
        alu_result: u32,
        opcode: ArmAluOpcode,
        rd: u32,
        carry_output: bool,
        carry_shifter: bool,
        v_output: bool,
    ) {
        if rd != 15 {
            self.rf.write_z(alu_result == 0);
            self.rf.write_n(alu_result.is_bit_set(31));
            if ArmAluOpcode::is_logical(opcode) {
                self.rf.write_c(carry_shifter);
            } else if ArmAluOpcode::is_arithmetic(opcode) {
                self.rf.write_c(carry_output);
                self.rf.write_v(v_output);
            }
        } else {
            let current_spsr = self.rf.get_spsr();
            let res = self.rf.write_cpsr(current_spsr);
            assert_ne!(res, Err(()));
        }

        println!("Flags after exuction of {:?}: V = {:?}; N = {:?}", opcode, v_output, alu_result.is_bit_set(31));
    }

    fn arm_branch_and_exchange(&mut self, req: &mut MemoryRequest, rsp: MemoryResponse) {
        let condition = self.arm_current_execute.get_range(31, 28);
        let opcode = self.arm_current_execute.get_range(7, 4);
        let rn = self.arm_current_execute.get_range(3, 0);

        if self.instruction_step == InstructionStep::STEP0 {
            if !self.rf.check_condition_code(condition) {
                return;
            }

            todo!("Switching to thumb mode behaviour");
        } else if self.instruction_step == InstructionStep::STEP1 {
        } else if self.instruction_step == InstructionStep::STEP2 {
        } else {
            panic!("Wrong step for instructin type ARM_BRANCH_AND_EXCHANGE");
        }
    }

    fn arm_branch(&mut self, req: &mut MemoryRequest, rsp: MemoryResponse) {
        let condition = self.arm_current_execute.get_range(31, 28);
        let opcode = self.arm_current_execute.get_range(24, 24);
        let mut nn = self.arm_current_execute.get_range(23, 0);
        let current_pc = self.rf.get_register(15);

        if self.instruction_step == InstructionStep::STEP0 {
            if !self.rf.check_condition_code(condition) {
                return;
            }

            nn |= if nn.is_bit_set(23) { 0xFF000000 } else { 0 };
            let offset: i32 = (nn as i32) << 2;

            self.arm_instruction_queue.clear();
            req.bus_cycle = BusCycle::NONSEQUENTIAL;
            self.data_is_fetch = false;
            self.instruction_step = InstructionStep::STEP1;
            if opcode == 1 {
                self.rf.write_register(14, current_pc + 4);
            }

            self.rf
                .write_register(15, (current_pc as i32 + 4 + offset) as u32);
        } else if self.instruction_step == InstructionStep::STEP1 {
            req.address = current_pc + 4;
            self.instruction_step = InstructionStep::STEP2;
        } else if self.instruction_step == InstructionStep::STEP2 {
            req.address = current_pc + 8;
            self.instruction_step = InstructionStep::STEP0;
        } else {
            panic!("Wrong step for instructin type ARM_BRANCH_AND_EXCHANGE");
        }
    }
}
