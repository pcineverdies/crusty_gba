/// instruction::ArmInstructionType
///
/// enum to represent the different categories of instructions
/// which have to be handled while in ARM mode. Using these
/// categories, multiple instructions can be grouped together,
/// taking into account their similar behaviour
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ArmInstructionType {
    DataProcessing,
    Multiply,
    MultiplyLong,
    SingleDataSwap,
    BranchAndExchange,
    HwTrasferReg,
    HwTransferImmediate,
    SingleDataTransfer,
    Undefined,
    BlockDataTransfer,
    Branch,
    CoprocessorDataTransfer,
    CoprocessorDataOperation,
    CoprocessorRegisterTransfer,
    SoftwareInterrupt,
    Unimplemented,
}

/// decode_arg
///
/// Get the type of ARM instruction given its opcode. This function
/// has been implemented thanks to [this](https://www.gregorygaines.com/blog/decoding-the-arm7tdmi-instruction-set-game-boy-advance/)
/// article by Gregory Gaines.
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
    let format_mask = 0b0000_1111_1100_0000_0000_0000_1111_0000;
    if (data & format_mask) == multiply_format {
        return ArmInstructionType::Multiply;
    }

    let multiply_long_format = 0b0000_0000_1000_0000_0000_0000_1001_0000;
    let format_mask = 0b0000_1111_1000_0000_0000_0000_1111_0000;
    if (data & format_mask) == multiply_long_format {
        return ArmInstructionType::MultiplyLong;
    }

    let halfword_data_transfer_register_format = 0b0000_0000_0000_0000_0000_0000_1001_0000;
    let format_mask = 0b0000_1110_0100_0000_0000_1111_1001_0000;
    if (data & format_mask) == halfword_data_transfer_register_format {
        return ArmInstructionType::HwTrasferReg;
    }

    let halfword_data_transfer_immediate_format = 0b0000_0000_0100_0000_0000_0000_1001_0000;
    let format_mask = 0b0000_1110_0100_0000_0000_0000_1001_0000;
    if (data & format_mask) == halfword_data_transfer_immediate_format {
        return ArmInstructionType::HwTransferImmediate;
    }

    let data_processing_format = 0b0000_0000_0000_0000_0000_0000_0000_0000;
    let format_mask = 0b0000_1100_0000_0000_0000_0000_0000_0000;
    if (data & format_mask) == data_processing_format {
        return ArmInstructionType::DataProcessing;
    }

    ArmInstructionType::Unimplemented
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
        assert_eq!(decode_arm(0xe0cbad9c), ArmInstructionType::MultiplyLong);
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
        // ldrh r3, [r8, #3]
        assert_eq!(
            decode_arm(0xe1d830b3),
            ArmInstructionType::HwTransferImmediate
        );
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
