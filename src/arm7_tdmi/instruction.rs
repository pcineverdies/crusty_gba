use crate::arm7_tdmi::register_file::ConditionCodeFlag;
use crate::arm7_tdmi::ARM7TDMI;
use crate::common::BitOperation;

/// instruction::ArmInstructionType
///
/// enum to represent the different categories of instructions which have to be handled while in
/// ARM mode. Using these categories, multiple instructions can be grouped together, taking into
/// account their similar behaviours.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[allow(dead_code)] // coprocessor instructions are not used
pub enum ArmInstructionType {
    DataProcessing,
    Multiply,
    SingleDataSwap,
    BranchAndExchange,
    HwTransfer,
    SingleDataTransfer,
    Undefined,
    BlockDataTransfer,
    Branch,
    CoprocessorDataTransfer,
    CoprocessorDataOperation,
    CoprocessorRegisterTransfer,
    SoftwareInterrupt,
    Unimplemented,
    PsrTransferMRS,
    PsrTransferMSR,
}

/// instruction::ArmAluOpcode
///
/// enum to represent the opcodes of ALU instructions in ARM mode
#[derive(Debug, PartialEq, Eq, Clone, Copy, FromPrimitive)]
#[repr(u32)]
pub enum ArmAluOpcode {
    AND = 0,
    EOR = 1,
    SUB = 2,
    RSB = 3,
    ADD = 4,
    ADC = 5,
    SBC = 6,
    RSC = 7,
    TST = 8,
    TEQ = 9,
    CMP = 10,
    CMN = 11,
    ORR = 12,
    MOV = 13,
    BIC = 14,
    MNV = 15,
}

impl ArmAluOpcode {
    /// ArmAluOpcode::is_test_opcode
    ///
    /// Some alu operations do not write back the result into the destination register, but they
    /// only update the flags.
    ///
    /// @param opcode [ArmAluOpcode]: opcode to check
    /// @return [bool]: true if the opcode is a test one
    pub fn is_test_opcode(opcode: ArmAluOpcode) -> bool {
        if opcode as u32 >= 8 && opcode as u32 <= 11 {
            return true;
        }
        false
    }

    /// ArmAluOpcode::is_logical
    ///
    /// Some alu operations are said to be "logical", not involving a sum or a subtraction. This
    /// characteristic affects the way flags are updated.
    ///
    /// @param opcode [ArmAluOpcode]: opcode to check
    /// @return [bool]: true if the opcode is a logical one
    pub fn is_logical(opcode: ArmAluOpcode) -> bool {
        if opcode as u32 <= 1
            || (opcode as u32 >= 8 && opcode as u32 <= 9)
            || (opcode as u32 >= 0xc)
        {
            return true;
        }
        false
    }

    /// ArmAluOpcode::is_arithmetic
    ///
    /// Some alu operations are said to be "arithmetic", involving a sum or a subtraction. This
    /// characteristic affects the way flags are updated.
    ///
    /// @param opcode [ArmAluOpcode]: opcode to check
    /// @return [bool]: true if the opcode is a logical one
    pub fn is_arithmetic(opcode: ArmAluOpcode) -> bool {
        !ArmAluOpcode::is_logical(opcode)
    }

    /// ArmAluOpcode::from_value
    ///
    /// return an instance of ArmAluOpcode given an opcode value.
    ///
    /// @param opcode [u32]: opcode to use
    /// @return [ArmAluOpcode]: associated opcode
    pub fn from_value(opcode: u32) -> ArmAluOpcode {
        if opcode == 0 {
            return ArmAluOpcode::AND;
        } else if opcode == 1 {
            return ArmAluOpcode::EOR;
        } else if opcode == 2 {
            return ArmAluOpcode::SUB;
        } else if opcode == 3 {
            return ArmAluOpcode::RSB;
        } else if opcode == 4 {
            return ArmAluOpcode::ADD;
        } else if opcode == 5 {
            return ArmAluOpcode::ADC;
        } else if opcode == 6 {
            return ArmAluOpcode::SBC;
        } else if opcode == 7 {
            return ArmAluOpcode::RSC;
        } else if opcode == 8 {
            return ArmAluOpcode::TST;
        } else if opcode == 9 {
            return ArmAluOpcode::TEQ;
        } else if opcode == 10 {
            return ArmAluOpcode::CMP;
        } else if opcode == 11 {
            return ArmAluOpcode::CMN;
        } else if opcode == 12 {
            return ArmAluOpcode::ORR;
        } else if opcode == 13 {
            return ArmAluOpcode::MOV;
        } else if opcode == 14 {
            return ArmAluOpcode::BIC;
        }
        ArmAluOpcode::MNV
    }
}

// instruction::ArmAluShiftCodes
//
// enum representing the different kinds of shift operations you might apply to operands.
#[derive(Debug, PartialEq, Eq, Clone, Copy, FromPrimitive)]
#[repr(u32)]
pub enum ArmAluShiftCodes {
    LSL = 0,
    LSR = 1,
    ASR = 2,
    ROR = 3,
}

/// instruction::decode_arg
///
/// Get the type of ARM instruction given its opcode. This function has been implemented thanks to
/// [this](https://www.gregorygaines.com/blog/decoding-the-arm7tdmi-instruction-set-game-boy-advance/) article by Gregory Gaines.
///
/// @param data [u32]: instruction to decode
/// @return [ArmInstructionType]: type of the instruction
pub fn decode_arm(data: u32) -> ArmInstructionType {
    let branch_and_exchange_format = 0b0000_0001_0010_1111_1111_1111_0001_0000;
    let format_mask = 0b0000_1111_1111_1111_1111_1111_1111_0000;
    if (data & format_mask) == branch_and_exchange_format {
        return ArmInstructionType::BranchAndExchange;
    }

    let block_data_transfer_format = 0b0000_1000_0000_0000_0000_0000_0000_0000;
    let format_mask = 0b0000_1110_0000_0000_0000_0000_0000_0000;
    if (data & format_mask) == block_data_transfer_format {
        return ArmInstructionType::BlockDataTransfer;
    }

    let branch_format = 0b0000_1010_0000_0000_0000_0000_0000_0000;
    let format_mask = 0b0000_1110_0000_0000_0000_0000_0000_0000;
    if (data & format_mask) == branch_format {
        return ArmInstructionType::Branch;
    }

    let software_interrupt_format = 0b0000_1111_0000_0000_0000_0000_0000_0000;
    let format_mask = 0b0000_1111_0000_0000_0000_0000_0000_0000;
    if (data & format_mask) == software_interrupt_format {
        return ArmInstructionType::SoftwareInterrupt;
    }

    let undefined_format = 0b0000_0110_0000_0000_0000_0000_0001_0000;
    let format_mask = 0b0000_1110_0000_0000_0000_0000_0001_0000;
    if (data & format_mask) == undefined_format {
        return ArmInstructionType::Undefined;
    }

    let single_data_transfer_format = 0b0000_0100_0000_0000_0000_0000_0000_0000;
    let format_mask = 0b0000_1100_0000_0000_0000_0000_0000_0000;
    if (data & format_mask) == single_data_transfer_format {
        return ArmInstructionType::SingleDataTransfer;
    }

    let single_data_swap_format = 0b0000_0001_0000_0000_0000_0000_1001_0000;
    let format_mask = 0b0000_1111_1000_0000_0000_1111_1111_0000;
    if (data & format_mask) == single_data_swap_format {
        return ArmInstructionType::SingleDataSwap;
    }

    let multiply_format = 0b0000_0000_0000_0000_0000_0000_1001_0000;
    let format_mask = 0b0000_1111_0000_0000_0000_0000_1111_0000;
    if (data & format_mask) == multiply_format {
        return ArmInstructionType::Multiply;
    }

    let halfword_data_transfer_format = 0b0000_0000_0000_0000_0000_0000_1001_0000;
    let format_mask = 0b0000_1110_0000_0000_0000_0000_1001_0000;
    if (data & format_mask) == halfword_data_transfer_format {
        return ArmInstructionType::HwTransfer;
    }

    let mrs_format = 0b0000_0001_0000_1111_0000_0000_0000_0000;
    let format_mask = 0b0000_1111_1011_1111_0000_0000_0000_0000;
    if (data & format_mask) == mrs_format {
        return ArmInstructionType::PsrTransferMRS;
    }

    let msr_format = 0b0000_0001_0010_0000_1111_0000_0000_0000;
    let format_mask = 0b0000_1101_1011_0000_1111_0000_0000_0000;
    if (data & format_mask) == msr_format {
        return ArmInstructionType::PsrTransferMSR;
    }

    let data_processing_format = 0b0000_0000_0000_0000_0000_0000_0000_0000;
    let format_mask = 0b0000_1100_0000_0000_0000_0000_0000_0000;
    if (data & format_mask) == data_processing_format {
        return ArmInstructionType::DataProcessing;
    }

    ArmInstructionType::Unimplemented
}

/// instruction::barrel_shifter
///
/// Performs a a shift operation using the internal barrel shift of arm, taking into account all
/// the weird corner cases.
///
/// @param operand [u32]: opearand to shift
/// @param shift_type [u32]: what kind of shift to use (must be in range 0..3)
/// @param shift_amound [u32]: how much to shift
/// @param old_c [bool]: current value of the c_flag
/// @return [u32]: shifted operand
/// @return [bool]: in case of a logical alu operation, this tells whether the carry flag should be
/// set or not.
/// @return [bool]: depending on the operands, the shift operation might not be done. This affects
/// the timing of the current instruction
pub fn barrel_shifter(
    operand: u32,
    shift_type: u32,
    shift_amount: u32,
    old_c: bool,
) -> (u32, bool, bool) {
    // Results to use
    let mut there_is_shift = true;
    let mut result = operand;
    let mut carry = old_c;

    match num::FromPrimitive::from_u32(shift_type) {
        // Logical shift left
        Some(ArmAluShiftCodes::LSL) => {
            // If shift amount is 0, no shift is done
            if shift_amount == 0 {
                there_is_shift = false;

            // Normal shift
            } else if shift_amount < 32 {
                carry = operand.is_bit_set(32 - shift_amount);
                result = operand.wrapping_shl(shift_amount);

            // Result is 0, carry is the lsb of the operand
            } else if shift_amount == 32 {
                carry = operand.is_bit_set(0);
                result = 0;

            // In case the shift_amount is too large, result is 0, carry is false
            } else {
                carry = false;
                result = 0;
            }
        }

        // Logical shift right
        Some(ArmAluShiftCodes::LSR) => {
            // equivalent 0 and 32: result is 0, carry is the msb
            if shift_amount == 0 || shift_amount == 32 {
                carry = operand.is_bit_set(31);
                result = 0;

            // Normal shift operation
            } else if shift_amount < 32 {
                carry = operand.is_bit_set(shift_amount - 1);
                result = operand.wrapping_shr(shift_amount);

            // In case the shift_amount is too large, result is 0, carry is false
            } else {
                carry = false;
                result = 0;
            }
        }

        // Arithmetic shift right (shifted bits are filled with msb of operand)
        Some(ArmAluShiftCodes::ASR) => {
            // Case of shift_amount 0 or >= 31: result is related to the msb, which is also the
            // carry
            if shift_amount == 0 || shift_amount >= 32 {
                carry = operand.is_bit_set(31);
                result = if carry { 0xFFFFFFFF } else { 0 };
            } else {
                carry = operand.is_bit_set(shift_amount - 1);
                result = (operand as i32).wrapping_shr(shift_amount) as u32;
            }
        }
        Some(ArmAluShiftCodes::ROR) => {
            // Special ROR operation (RORX), in which the rotation is by 1 and the shifted bit is
            // the old carry of the system.
            if shift_amount == 0 {
                carry = operand.is_bit_set(0);
                result = operand.rotate_right(1);
                if old_c {
                    result.set_bit(31);
                } else {
                    result.clear_bit(31);
                }

            // Only the 5 msbs of shift_amount are used in this case
            } else {
                let shift_amount = shift_amount % 32;
                if shift_amount == 0 {
                    carry = operand.is_bit_set(31);
                } else {
                    carry = operand.is_bit_set(shift_amount - 1);
                    result = operand.rotate_right(shift_amount);
                }
            }
        }
        None => {
            panic!("Invalid shift type");
        }
    }

    return (result, carry, there_is_shift);
}

impl ARM7TDMI {
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
    pub fn alu(&self, operand1: u32, operand2: u32, opcode: ArmAluOpcode) -> (u32, bool, bool) {
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
    pub fn alu_operation(
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
    pub fn update_flags(
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
}

#[cfg(test)]
mod test_instructions {

    use crate::arm7_tdmi::instruction::{decode_arm, ArmInstructionType};

    #[test]
    fn test_arm_decode() {
        // add r1, r2, 0x10
        assert_eq!(decode_arm(0xe2821010), ArmInstructionType::DataProcessing);
        // mov r1, r2
        assert_eq!(decode_arm(0xe1a01002), ArmInstructionType::DataProcessing);
        // bics r1, r2
        assert_eq!(decode_arm(0xe1d11002), ArmInstructionType::DataProcessing);
        // mlaeq r10, r11, r12, r13
        assert_eq!(decode_arm(0x002adc9b), ArmInstructionType::Multiply);
        // smull r10, r11, r12, r13
        assert_eq!(decode_arm(0xe0cbad9c), ArmInstructionType::Multiply);
        // bleq 0x10
        assert_eq!(decode_arm(0x0b000002), ArmInstructionType::Branch);
        // bxmi r9
        assert_eq!(
            decode_arm(0x412fff19),
            ArmInstructionType::BranchAndExchange
        );
        // swp r10, r11, [r12]
        assert_eq!(decode_arm(0xe10ca09b), ArmInstructionType::SingleDataSwap);
        // ldrb r3, [r8, #3]
        assert_eq!(
            decode_arm(0xe5d83003),
            ArmInstructionType::SingleDataTransfer
        );

        // ldrh r3, [r0, #0xc1]
        assert_eq!(decode_arm(0xe1d0acb1), ArmInstructionType::HwTransfer);

        // ldrh r3, <same address>
        assert_eq!(decode_arm(0xe15fa0b8), ArmInstructionType::HwTransfer);

        // undefined
        assert_eq!(decode_arm(0xf7ffffff), ArmInstructionType::Undefined);
        // ldmia r0, {r5 - r8}
        assert_eq!(
            decode_arm(0xe89001e0),
            ArmInstructionType::BlockDataTransfer
        );
        // swi 0x30
        assert_eq!(
            decode_arm(0xef000030),
            ArmInstructionType::SoftwareInterrupt
        );
    }
}
