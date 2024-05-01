mod cpu_test;
mod instruction;
mod register_file;

use crate::arm7_tdmi::instruction::{decode_arm, ArmAluOpcode, ArmInstructionType};
use crate::arm7_tdmi::register_file::{ConditionCodeFlag, RegisterFile};
use crate::bus::{BusCycle, BusSignal, MemoryRequest, MemoryResponse, TransferSize};
use crate::common::BitOperation;
use std::collections::VecDeque;

use self::instruction::barrel_shifter;

/// Definition of a NOP instruction used to initialize the CPU
const NOP: u32 = 0xE1A00000_u32;

/// arm7_tdmi::InstructionStep
///
/// Many of the instructions require an execute stage which is longer than one cycle. Each
/// instruction is thus implemented using an FSM handling the different states using a variable of
/// this type.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum InstructionStep {
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
    rf: register_file::RegisterFile,      // Register File
    arm_instruction_queue: VecDeque<u32>, // Instruction queue (arm)
    arm_current_execute: u32,             // Current executed instruction (arm)
    instruction_step: InstructionStep,    // Current instructions stpe for FSM handling
    data_is_fetch: bool,                  // Is next data a fetch?
    data_is_reading: bool,                // Is next data a reading?
    last_used_address: u32,               // Store the last address sent on the bus
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
            data_is_reading: true,
            instruction_step: InstructionStep::STEP0,
            last_used_address: 0,
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
            address: self.rf.get_register(15).wrapping_add(8),  // Implements only arm mode
            nr_w: BusSignal::LOW,                               // Read operation
            mas: TransferSize::WORD,                            // Reads 32 bits
            n_opc: BusSignal::LOW,                              // Requires an opcode
            data: 0,
            n_trans:                                            // Whether we are priviliged
                if self.rf.get_mode() == OperatingMode::USER {
                    BusSignal::LOW
                } else {
                    BusSignal::HIGH
                },
            lock: BusSignal::LOW,                               // No swap opeartion
            t_bit: BusSignal::LOW,                              // arm mode
            bus_cycle: BusCycle::SEQUENTIAL,                        // bus cycle is sequential
        };

        // Memory request is not completed, and the cpu must stall
        if rsp.n_wait == BusSignal::LOW {
            return next_request;
        }

        // A fetch was in progress: add the data to the instruction queue
        if self.data_is_reading && self.data_is_fetch {
            self.arm_instruction_queue.push_back(rsp.data);
        }

        self.data_is_reading = true;
        self.data_is_fetch = true;

        // In case of arm mode, decode the current executed function
        match decode_arm(self.arm_current_execute) {
            ArmInstructionType::DataProcessing => self.arm_data_processing(&mut next_request),
            ArmInstructionType::BranchAndExchange => {
                self.arm_branch_and_exchange(&mut next_request)
            }
            ArmInstructionType::SingleDataTransfer => {
                self.arm_single_data_transfer(&mut next_request, &rsp)
            }
            ArmInstructionType::Branch => self.arm_branch(&mut next_request),
            ArmInstructionType::HwTrasferReg => todo!(),
            ArmInstructionType::HwTransferImmediate => todo!(),
            ArmInstructionType::Undefined => todo!(),
            ArmInstructionType::BlockDataTransfer => todo!(),
            ArmInstructionType::Multiply => todo!(),
            ArmInstructionType::MultiplyLong => todo!(),
            ArmInstructionType::SingleDataSwap => todo!(),
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
            self.rf
                .write_register(15, self.rf.get_register(15).wrapping_add(4));
        }

        self.last_used_address = next_request.address;
        next_request
    }

    /// arm7_tdmi::arm_data_processing
    ///
    /// function to handle all the data processing instructions (MOV, ADD, AND...)
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    fn arm_data_processing(&mut self, req: &mut MemoryRequest) {
        // Destination address
        let rd = self.arm_current_execute.get_range(15, 12);

        if self.instruction_step == InstructionStep::STEP0 {
            // === Decode instruction ===

            // Opcode of the alu instruction
            let opcode = ArmAluOpcode::from_value(self.arm_current_execute.get_range(24, 21));
            // 1 if flags should be updated
            let s_flag = self.arm_current_execute.get_range(20, 20);
            // 1 if the operand to use is an immediate encoded in the msbs of the instruction
            let i_flag = self.arm_current_execute.get_range(25, 25);
            // Condition to be checked, otherwise instruction is skipped
            let condition = self.arm_current_execute.get_range(31, 28);
            // First operand
            let rn = self.arm_current_execute.get_range(19, 16);
            // Shift amount for register
            let mut shift_amount = self.arm_current_execute.get_range(11, 7);
            // Shift register
            let rs = self.arm_current_execute.get_range(11, 8);
            // What kind of shift
            let shift_type = self.arm_current_execute.get_range(6, 5);
            // Is shift done with a register or with an immediate
            let r_flag = self.arm_current_execute.get_range(4, 4);
            // Second operand
            let rm = self.arm_current_execute.get_range(3, 0);
            // Shift amount for immediate
            let is = self.arm_current_execute.get_range(11, 8);
            // Immediate value
            let nn = self.arm_current_execute.get_range(7, 0);

            let mut operand2;
            let mut carry_shifter = self.rf.is_flag_set(&ConditionCodeFlag::C);
            let mut operand1 = self.rf.get_register(rn);
            let mut there_is_shift = false;

            if !self.rf.check_condition_code(condition) {
                return;
            }

            // operand1 is rn, operand 2 is nn << (2 * is)
            if i_flag == 1 {
                operand1 = operand1.wrapping_add((rn == 15) as u32 * 8);
                operand2 = nn.rotate_right(is * 2);

            // operand 1 is rn, operand 2 can be either `rm OP rs` or `rm op imm`
            } else {
                operand2 = self.rf.get_register(rm);
                if r_flag == 1 {
                    if rs == 15 {
                        panic!("Cannot use r15 as rs register in ALU operations");
                    }
                    shift_amount = self.rf.get_register(rs).get_range(7, 0);

                    // if rn == 15 or rm == 15, operands should be incremented
                    operand1 = operand1.wrapping_add((rn == 15) as u32 * 12);
                    operand2 = operand2.wrapping_add((rm == 15) as u32 * 12);
                } else {
                    // if rn == 15 or rm == 15, operands should be incremented
                    operand1 = operand1.wrapping_add((rn == 15) as u32 * 8);
                    operand2 = operand2.wrapping_add((rm == 15) as u32 * 8);
                }

                (operand2, carry_shifter, there_is_shift) = barrel_shifter(
                    operand2,
                    shift_type,
                    shift_amount,
                    self.rf.is_flag_set(&ConditionCodeFlag::C),
                );
            }

            // Get result from alu, and next value of carry and overflow flag in case of arithmetic
            // operations
            let (next_to_write, c_output, v_output) =
                self.alu_operation(operand1, operand2, opcode);

            // Write the result back for all the instructions which are not test
            if !ArmAluOpcode::is_test_opcode(opcode) {
                self.rf.write_register(rd, next_to_write);
            }

            // Update flags if the instruction is a test one or if s_flag is set
            if ArmAluOpcode::is_test_opcode(opcode) || s_flag == 1 {
                self.update_flags(next_to_write, opcode, rd, c_output, carry_shifter, v_output);
            }

            // If there is a shift, one extra cycle
            if there_is_shift {
                req.bus_cycle = BusCycle::INTERNAL;
                self.data_is_fetch = false;
                self.instruction_step = InstructionStep::STEP1;

            // If rd == 15, 2 more extra cycles to refill the pipeline
            } else if rd == 15 {
                self.arm_instruction_queue.clear();
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.data_is_fetch = false;
                self.instruction_step = InstructionStep::STEP2;
            }
        } else if self.instruction_step == InstructionStep::STEP1 {
            // If rd == 15, 2 more extra cycles to refill the pipeline
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
            req.address = self.rf.get_register(15).wrapping_add(4);
            self.rf
                .write_register(15, self.rf.get_register(15).wrapping_sub(4));
            self.instruction_step = InstructionStep::STEP0;
        } else {
            panic!("Wrong step for instructin type ARM_DATA_PROCESSING");
        }
    }

    /// arm7_tdmi::alu
    ///
    /// Implement the arm alu for arithmetic instructions, by both computing the correct result and generating the two expected
    /// flags, carry and overflow.
    ///
    /// @param operand1 [u32]: first input of the alu
    /// @param operand2 [u32]: second input of the alu
    /// @param opcode [ArmAluOpcode]: opcode to use
    /// @return [u32]: result
    /// @return [u32]: value of the arithmetic carry flag
    /// @return [u32]: value of the arithemtic overflow flag
    fn alu(&self, operand1: u32, operand2: u32, opcode: ArmAluOpcode) -> (u32, bool, bool) {
        use ArmAluOpcode::*;
        use ConditionCodeFlag::*;

        let (alu_result, v_output, c_output);
        let (op1, op2, c_in);

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

    /// arm7_tdmi::alu_operation
    ///
    /// Implement the arm alu both for logical and arithmetic instructions. Arithemtic instructions
    /// rely on `arm7_tdmi::alu` to be exectued.
    ///
    /// @param operand1 [u32]: first input of the alu
    /// @param operand2 [u32]: second input of the alu
    /// @param opcode [ArmAluOpcode]: opcode to use
    /// @return [u32]: result
    /// @return [u32]: value of the arithmetic carry flag
    /// @return [u32]: value of the arithemtic overflow flag
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

    /// arm7_tdmi::update_flags
    ///
    /// Update the flags of the cpu depending on the instruction executed
    ///
    /// @param alu_result [u32]: result of the alu
    /// @param opcode [ArmAluOpcode]: kind of executed instruction
    /// @param rd [u32]: destination register
    /// @param carry_output [bool]: carry to use for arithmetic instructions
    /// @param carry_shifter [bool]: carry to use for logical instructions
    /// @param v_output [bool]: overflow to use for arithmetic instructions
    fn update_flags(
        &mut self,
        alu_result: u32,
        opcode: ArmAluOpcode,
        rd: u32,
        carry_output: bool,
        carry_shifter: bool,
        v_output: bool,
    ) {
        // if the destination register is not r15, just update the flags in the normal way
        if rd != 15 {
            self.rf.write_z(alu_result == 0);
            self.rf.write_n(alu_result.is_bit_set(31));
            if ArmAluOpcode::is_logical(opcode) {
                self.rf.write_c(carry_shifter);
            } else if ArmAluOpcode::is_arithmetic(opcode) {
                self.rf.write_c(carry_output);
                self.rf.write_v(v_output);
            }

        // otherwise, the instruction is a sort of return: move the current spsr into the cpsr
        } else {
            let current_spsr = self.rf.get_spsr();
            let res = self.rf.write_cpsr(current_spsr);
            assert_ne!(res, Err(()));
        }
    }

    /// arm7_tdmi::arm_branch_and_exchange
    ///
    /// TBD
    fn arm_branch_and_exchange(&mut self, req: &mut MemoryRequest) {
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

    /// arm7_tdmi::arm_branch
    ///
    /// Function to handle all the branch instructions
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    fn arm_branch(&mut self, req: &mut MemoryRequest) {
        let condition = self.arm_current_execute.get_range(31, 28);
        let opcode = self.arm_current_execute.get_range(24, 24);
        let mut nn = self.arm_current_execute.get_range(23, 0);
        let current_pc = self.rf.get_register(15);

        if !self.rf.check_condition_code(condition) {
            return;
        }

        if self.instruction_step == InstructionStep::STEP0 {
            // Sign extenstion of the 24 bits immediate. Offset is this value * 4
            nn |= if nn.is_bit_set(23) { 0xFF000000 } else { 0 };
            let offset: i32 = (nn as i32) << 2;

            self.arm_instruction_queue.clear();
            req.bus_cycle = BusCycle::NONSEQUENTIAL;
            self.data_is_fetch = false;
            self.instruction_step = InstructionStep::STEP1;

            // If the operation is branch and link, store the next instruction to be used in the
            // link register
            if opcode == 1 {
                self.rf.write_register(14, current_pc.wrapping_add(4));
            }

            // Increment only by 4 due to the automatic increase of the pc at the end of the
            // instruction
            self.rf
                .write_register(15, (current_pc as i32 + 4 + offset) as u32);

        // Refill the pipeline in the next two steps
        } else if self.instruction_step == InstructionStep::STEP1 {
            req.address = current_pc.wrapping_add(4);
            self.instruction_step = InstructionStep::STEP2;
        } else if self.instruction_step == InstructionStep::STEP2 {
            req.address = current_pc.wrapping_add(8);
            self.instruction_step = InstructionStep::STEP0;
        } else {
            panic!("Wrong step for instructin type ARM_BRANCH_AND_EXCHANGE");
        }
    }

    /// arm7_tdmi::arm_single_data_transfer
    ///
    /// Function to handle all the single data transfer instructions (LDR, STR, LDRB, STRB)
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    /// @param rsp [&MemoryResponse]: response from the memory
    fn arm_single_data_transfer(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        // condition to be checked for the instruction to be executed
        let condition = self.arm_current_execute.get_range(31, 28);

        // load flag, 1 for a load operation
        let l_flag = self.arm_current_execute.get_range(20, 20);

        // operand register
        let rn = self.arm_current_execute.get_range(19, 16);

        // destination register
        let rd = self.arm_current_execute.get_range(15, 12);

        // p flag, 1 for pre-increment
        let p_flag = self.arm_current_execute.get_range(24, 24);

        // when p is 1, w flag: 1 for increment the base register after the transaction
        // when p is 0, t flag: 1 to make the request non privileged on the bus
        let tw_flag = self.arm_current_execute.get_range(21, 21);

        // when b is 1, a byte is requested
        let b_flag = self.arm_current_execute.get_range(22, 22);

        let offset;
        let mut address_to_mem = self.last_used_address;

        if !self.rf.check_condition_code(condition) {
            return;
        }

        // Common between load and store: during step1, the bus transaction to store/load is
        // generated, so in this step we need the address to be used.
        if self.instruction_step == InstructionStep::STEP1 {
            let i_flag = self.arm_current_execute.get_range(25, 25);
            let u_flag = self.arm_current_execute.get_range(23, 23);
            let immediate = self.arm_current_execute.get_range(11, 0);
            let shift_amount = self.arm_current_execute.get_range(11, 7);
            let shift_type = self.arm_current_execute.get_range(6, 5);
            let rm = self.arm_current_execute.get_range(3, 0);

            address_to_mem = self.rf.get_register(rn);

            if i_flag == 0 {
                offset = immediate;
            } else {
                if rm == 15 {
                    panic!("Cannot use r15 as shift register in ARM_SINGLE_DATA_TRANSFER");
                }
                (offset, _, _) = barrel_shifter(
                    self.rf.get_register(rm),
                    shift_type,
                    shift_amount,
                    self.rf.is_flag_set(&ConditionCodeFlag::C),
                );
            }

            if p_flag == 1 {
                if u_flag == 1 {
                    req.address = address_to_mem.wrapping_add(offset);
                } else {
                    req.address = address_to_mem.wrapping_sub(offset);
                }
            } else {
                if tw_flag == 1 {
                    req.n_trans = BusSignal::LOW;
                }
            }

            if b_flag == 1 {
                req.mas = TransferSize::BYTE;
            }
        }

        if l_flag == 1 {
            if self.instruction_step == InstructionStep::STEP0 {
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.instruction_step = InstructionStep::STEP1;
            } else if self.instruction_step == InstructionStep::STEP1 {
                self.data_is_fetch = false;
                req.bus_cycle = BusCycle::INTERNAL;
                // Post increment of the base register
                if p_flag == 0 || tw_flag == 1 {
                    self.rf.write_register(rn, address_to_mem);
                }
                self.instruction_step = InstructionStep::STEP2;
            } else if self.instruction_step == InstructionStep::STEP2 {
                // Write data back to the destination register
                let mut data_to_write = rsp.data;
                let offset = self.last_used_address % 4;

                // If only one byte is requested, the correct byte must be extracted from the
                // received data, taking into account that we are only working in little endian
                // mode
                if b_flag == 1 {
                    data_to_write = data_to_write.get_range(offset * 8 + 7, offset * 8);

                // If the required address was not word aligned, a rotation should be applied
                } else {
                    data_to_write = data_to_write.rotate_right(offset * 8);
                }

                // Update the destination register
                self.rf.write_register(rd, data_to_write);
                self.data_is_fetch = false;

                // If destination is r15, then the pipeline is to be filled again
                if rd == 15 {
                    self.arm_instruction_queue.clear();
                    req.bus_cycle = BusCycle::NONSEQUENTIAL;
                    self.instruction_step = InstructionStep::STEP3;
                } else {
                    req.bus_cycle = BusCycle::SEQUENTIAL;
                    self.instruction_step = InstructionStep::STEP0;
                }
            } else if self.instruction_step == InstructionStep::STEP3 {
                req.address = self.rf.get_register(15);
                self.instruction_step = InstructionStep::STEP4;
            } else if self.instruction_step == InstructionStep::STEP4 {
                req.address = self.rf.get_register(15).wrapping_add(4);
                self.rf
                    .write_register(15, self.rf.get_register(15).wrapping_sub(4));
                self.instruction_step = InstructionStep::STEP0;
            } else if self.instruction_step == InstructionStep::STEP4 {
            } else {
                panic!("Wrong step for instructin type ARM_LOAD");
            }
        } else {
            if self.instruction_step == InstructionStep::STEP0 {
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.instruction_step = InstructionStep::STEP1;
            } else if self.instruction_step == InstructionStep::STEP1 {
                self.data_is_reading = false;
                req.data = self.rf.get_register(rd);
                // If only one byte is to be moved, copy the byte over all the 32 lines of the bus
                if b_flag == 1 {
                    let byte = req.data & 0xff;
                    req.data = byte | (byte << 8) | (byte << 16) | (byte << 24);
                }
                req.nr_w = BusSignal::HIGH;
                self.instruction_step = InstructionStep::STEP0;
            } else {
                panic!("Wrong step for instructin type ARM_STORE");
            }
        }
    }
}
