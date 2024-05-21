use crate::arm7_tdmi::instruction::barrel_shifter;
use crate::arm7_tdmi::instruction::ArmAluOpcode;
use crate::arm7_tdmi::register_file::ConditionCodeFlag;
use crate::arm7_tdmi::{InstructionStep, ARM7TDMI};
use crate::bus::{BusCycle, MemoryRequest, MemoryResponse, TransferSize};
use crate::common::BitOperation;

impl ARM7TDMI {
    /// arm7_tdmi::thumb_move_shifter_register
    ///
    /// Function to execute thumb.1 instructions. It works by translating the opcode to the
    /// corresponding arm one (move with shift)
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    pub fn thumb_move_shifter_register(&mut self, req: &mut MemoryRequest) {
        let opcode = self.arm_current_execute.get_range(12, 11);
        let offset = self.arm_current_execute.get_range(10, 6);
        let rs = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);

        let mut arm_instruction = 0b1110_0001_1011_0000_0000_0000_0000_0000;
        let current_instruction = self.arm_current_execute;

        arm_instruction |= rd << 12;
        arm_instruction |= offset << 7;
        arm_instruction |= opcode << 5;
        arm_instruction |= rs << 0;

        self.arm_current_execute = arm_instruction;
        self.arm_data_processing(req);
        self.arm_current_execute = current_instruction;
    }

    /// arm7_tdmi::thumb_add_subtract
    ///
    /// Function to execute thumb.2 instructions. It works by translating the opcode to the
    /// corresponding arm one (either add or subtract)
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    pub fn thumb_add_subtract(&mut self, req: &mut MemoryRequest) {
        let opcode = self.arm_current_execute.get_range(10, 9);
        let rn = self.arm_current_execute.get_range(8, 6);
        let rs = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);

        let mut arm_instruction = 0b1110_0000_0001_0000_0000_0000_0000_0000;
        let current_instruction = self.arm_current_execute;

        // Add operation
        if opcode & 1 == 0 {
            arm_instruction |= 0x4 << 21;

        // Sub operation
        } else {
            arm_instruction |= 0x2 << 21;
        }

        // Immediate operand, otherwise register operand (default)
        if opcode >= 2 {
            arm_instruction = arm_instruction.set_bit(25);
        }

        arm_instruction |= rs << 16;
        arm_instruction |= rd << 12;
        arm_instruction |= rn << 0;

        self.arm_current_execute = arm_instruction;
        self.arm_data_processing(req);
        self.arm_current_execute = current_instruction;
    }

    /// arm7_tdmi::thumb_alu_immediate
    ///
    /// Function to execute thumb.4 instructions. It works by translating the opcode to the
    /// corresponding arm one (ADD, SUB, MOV or CMP)
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).

    pub fn thumb_alu_immediate(&mut self, req: &mut MemoryRequest) {
        let opcode = self.arm_current_execute.get_range(12, 11);
        let rd = self.arm_current_execute.get_range(10, 8);
        let nn = self.arm_current_execute.get_range(7, 0);

        let mut arm_instruction = 0b1110_0010_0001_0000_0000_0000_0000_0000;
        let current_instruction = self.arm_current_execute;

        arm_instruction |= rd << 12;
        arm_instruction |= rd << 16;
        arm_instruction |= nn << 0;

        arm_instruction |= if opcode == 0 {
            ArmAluOpcode::MOV as u32
        } else if opcode == 1 {
            ArmAluOpcode::CMP as u32
        } else if opcode == 2 {
            ArmAluOpcode::ADD as u32
        } else {
            ArmAluOpcode::SUB as u32
        } << 21;

        self.arm_current_execute = arm_instruction;
        self.arm_data_processing(req);
        self.arm_current_execute = current_instruction;
    }

    /// arm7_tdmi::thumb_alu
    ///
    /// Function to execute thumb.3 instructions. It works by translating the opcode to the
    /// corresponding arm one (either alu opcode or mul opcode)
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    pub fn thumb_alu(&mut self, req: &mut MemoryRequest) {
        let opcode = self.arm_current_execute.get_range(9, 6);
        let rs = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);

        let mut arm_instruction = 0b1110_0000_0001_0000_0000_0000_0000_0000;
        let current_instruction = self.arm_current_execute;

        // mul case
        if opcode == 0xd {
            arm_instruction = 0b1110_0000_0001_0000_0000_0000_1001_0000;
            arm_instruction |= rd << 16;
            arm_instruction |= rd << 8;
            arm_instruction |= rs << 0;

        // shift case -> translated to a mov with shift operation
        } else if (opcode >= 0x2 && opcode <= 0x4) || opcode == 0x7 {
            arm_instruction |= 0xd << 21;
            arm_instruction |= rs << 8;
            if opcode == 7 {
                arm_instruction |= 3 << 5;
            } else {
                arm_instruction |= (opcode - 2) << 5;
            }
            arm_instruction = arm_instruction.set_bit(4);
            arm_instruction |= rd << 12;
            arm_instruction |= rd << 0;

        // neg case, implemented using RSBS
        } else if opcode == 0x9 {
            arm_instruction |= 0x3 << 21;
            arm_instruction |= rd << 12;
            arm_instruction |= rs << 16;
            arm_instruction = arm_instruction.set_bit(25);

        // All the other alu instructions have a 1 to 1 mapping with arm alu execution
        } else {
            arm_instruction |= opcode << 21;
            arm_instruction |= rd << 12;
            arm_instruction |= rs << 0;
            arm_instruction |= rd << 16;
        }

        self.arm_current_execute = arm_instruction;
        if opcode == 0x0d {
            self.arm_multiply(req);
        } else {
            self.arm_data_processing(req);
        }
        self.arm_current_execute = current_instruction;
    }

    /// arm7_tdmi::thumb_hi_register_bx
    ///
    /// Function to execute thumb.5 instructions. Since the execution is a bit different from the
    /// arm one, and different opcodes lead to different functions, the code has been
    /// re-implemented.
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    /// @param rsp [&MemoryResponse]: response from the memory
    pub fn thumb_hi_register_bx(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let opcode = self.arm_current_execute.get_range(9, 8);
        let msbd = self.arm_current_execute.get_range(7, 7);
        let msbs = self.arm_current_execute.get_range(6, 6);
        let rs = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);

        let full_rd = if msbd == 1 { rd | 0x8 } else { rd };
        let full_rs = if msbs == 1 { rs | 0x8 } else { rs };

        let current_instruction = self.arm_current_execute;

        if opcode <= 2 {
            let mut arm_instruction = 0b1110_0000_0000_0000_0000_0000_0000_0000;

            arm_instruction |= if opcode == 0 {
                ArmAluOpcode::ADD as u32
            } else if opcode == 1 {
                ArmAluOpcode::CMP as u32
            } else {
                ArmAluOpcode::MOV as u32
            } << 21;

            arm_instruction |= full_rd << 12;
            arm_instruction |= full_rd << 16;
            arm_instruction |= full_rs << 0;

            self.arm_current_execute = arm_instruction;
            self.arm_data_processing(req);
        } else {
            let mut arm_instruction = 0b1110_0001_0010_1111_1111_1111_0001_0000;
            arm_instruction |= full_rs << 0;

            self.arm_current_execute = arm_instruction;
            self.arm_branch_and_exchange(req, rsp);
        }

        self.arm_current_execute = current_instruction;
    }

    /// arm7_tdmi::thumb_pc_relative_load
    ///
    /// Function to execute thumb.6 instructions. It works by translating the opcode to the
    /// corresponding arm one (arm single data load)
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    /// @param rsp [&MemoryResponse]: response from the memory
    pub fn thumb_pc_relative_load(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let rd = self.arm_current_execute.get_range(10, 8);
        let nn = self.arm_current_execute.get_range(7, 0) << 2;

        let current_instruction = self.arm_current_execute;
        let mut arm_instruction = 0b1110_0101_1001_0000_0000_0000_0000_0000;

        arm_instruction |= 15 << 16;
        arm_instruction |= rd << 12;
        arm_instruction |= nn << 0;

        self.arm_current_execute = arm_instruction;
        self.arm_single_data_transfer(req, rsp);
        self.arm_current_execute = current_instruction;
    }

    /// arm7_tdmi::thumb_load_store_reg_offset
    ///
    /// Function to execute thumb.7 instructions. It works by translating the opcode to the
    /// corresponding arm one (arm single data load)
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    /// @param rsp [&MemoryResponse]: response from the memory
    pub fn thumb_load_store_reg_offset(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let opcode = self.arm_current_execute.get_range(11, 10);
        let ro = self.arm_current_execute.get_range(8, 6);
        let rb = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);
        let current_instruction = self.arm_current_execute;
        let mut arm_instruction = 0b1110_0111_1000_0000_0000_0000_0000_0000;

        // byte load (word load by default, corresponding to even opcodes)
        if opcode & 1 == 1 {
            arm_instruction = arm_instruction.set_bit(22);
        }

        // load operation (store by default)
        if opcode > 1 {
            arm_instruction = arm_instruction.set_bit(20);
        }

        arm_instruction |= rb << 16;
        arm_instruction |= rd << 12;
        arm_instruction |= ro << 0;

        self.arm_current_execute = arm_instruction;
        self.arm_single_data_transfer(req, rsp);
        self.arm_current_execute = current_instruction;
    }

    /// arm7_tdmi::thumb_load_store_sign_ext
    ///
    /// Function to execute thumb.8 instructions. It works by translating the opcode to the
    /// corresponding arm one (arm halfword transfer)
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    /// @param rsp [&MemoryResponse]: response from the memory
    pub fn thumb_load_store_sign_ext(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let opcode = self.arm_current_execute.get_range(11, 10);
        let ro = self.arm_current_execute.get_range(8, 6);
        let rb = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);
        let current_instruction = self.arm_current_execute;
        let mut arm_instruction = 0b1110_0001_1000_0000_0000_0000_1000_0000;

        // load (store by default)
        if opcode != 0 {
            arm_instruction = arm_instruction.set_bit(20);
        }

        arm_instruction |= rb << 16;
        arm_instruction |= rd << 12;
        arm_instruction |= ro << 0;

        arm_instruction |= if opcode == 1 {
            2
        } else if opcode == 3 {
            3
        } else {
            1
        } << 5;

        self.arm_current_execute = arm_instruction;
        self.arm_hw_transfer(req, rsp);
        self.arm_current_execute = current_instruction;
    }

    /// arm7_tdmi::thumb_load_store_imm_offset
    ///
    /// Function to execute thumb.9 instructions. It works by translating the opcode to the
    /// corresponding arm one (arm single data load)
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    /// @param rsp [&MemoryResponse]: response from the memory
    pub fn thumb_load_store_imm_offset(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let opcode = self.arm_current_execute.get_range(12, 11);

        let current_instruction = self.arm_current_execute;
        let mut arm_instruction = 0b1110_0101_1000_0000_0000_0000_0000_0000;

        // different offset depending on the size of teh transfer
        let offset = if opcode > 1 {
            arm_instruction = arm_instruction.set_bit(22);
            self.arm_current_execute.get_range(10, 6)
        } else {
            self.arm_current_execute.get_range(10, 6) * 4
        };
        let rb = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);

        // load (store by defalut)
        if opcode & 1 == 1 {
            arm_instruction = arm_instruction.set_bit(20);
        }

        arm_instruction |= rb << 16;
        arm_instruction |= rd << 12;
        arm_instruction |= offset << 0;

        self.arm_current_execute = arm_instruction;
        self.arm_single_data_transfer(req, rsp);
        self.arm_current_execute = current_instruction;
    }

    /// arm7_tdmi::thumb_load_store_halfword
    ///
    /// Function to execute thumb.10 instructions. It works by translating the opcode to the
    /// corresponding arm one (arm halfword transfer)
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    /// @param rsp [&MemoryResponse]: response from the memory
    pub fn thumb_load_store_halfword(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let opcode = self.arm_current_execute.get_range(11, 11);
        let nn = self.arm_current_execute.get_range(10, 6) << 1;
        let rb = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);

        let current_instruction = self.arm_current_execute;
        let mut arm_instruction = 0b1110_0001_1100_0000_0000_0000_1010_0000;

        arm_instruction |= opcode << 20;
        arm_instruction |= rb << 16;
        arm_instruction |= rd << 12;
        arm_instruction |= nn << 0;

        self.arm_current_execute = arm_instruction;
        self.arm_hw_transfer(req, rsp);
        self.arm_current_execute = current_instruction;
    }

    /// arm7_tdmi::thumb_sp_relative_load_store
    ///
    /// Function to execute thumb.11 instructions. It works by translating the opcode to the
    /// corresponding arm one (arm single data load)
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    /// @param rsp [&MemoryResponse]: response from the memory
    pub fn thumb_sp_relative_load_store(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let opcode = self.arm_current_execute.get_range(11, 11);
        let rd = self.arm_current_execute.get_range(10, 8);
        let nn = self.arm_current_execute.get_range(7, 0) << 2;

        let current_instruction = self.arm_current_execute;
        let mut arm_instruction = 0b1110_0101_1000_0000_0000_0000_0000_0000;
        arm_instruction |= opcode << 20;

        arm_instruction |= 13 << 16;
        arm_instruction |= rd << 12;
        arm_instruction |= nn << 0;

        self.arm_current_execute = arm_instruction;
        self.arm_single_data_transfer(req, rsp);
        self.arm_current_execute = current_instruction;
    }

    /// arm7_tdmi::thumb_sp_relative_load_store
    ///
    /// Function to execute thumb.12 instructions. As the operation is straightforward, it had been
    /// re-implemented.
    pub fn thumb_load_address(&mut self, req: &mut MemoryRequest) {
        let opcode = self.arm_current_execute.get_range(11, 11);
        let rd = self.arm_current_execute.get_range(10, 8);
        let nn = self.arm_current_execute.get_range(7, 0);

        let mut arm_instruction = 0b1110_0010_0000_0000_0000_1111_0000_0000;
        let current_instruction = self.arm_current_execute;

        arm_instruction |= rd << 12;
        arm_instruction |= if opcode == 0 { 15 } else { 13 } << 16;
        arm_instruction |= nn << 0;

        arm_instruction |= (ArmAluOpcode::ADD as u32) << 21;

        self.arm_current_execute = arm_instruction;
        self.arm_data_processing(req);
        self.arm_current_execute = current_instruction;
    }

    /// arm7_tdmi::thumb_sp_relative_load_store
    ///
    /// Function to execute thumb.13 instructions. As the operation is straightforward, it had been
    /// re-implemented.
    pub fn thumb_add_offset_to_sp(&mut self, req: &mut MemoryRequest) {
        let opcode = self.arm_current_execute.get_range(7, 7);
        let nn = self.arm_current_execute.get_range(6, 0);

        let mut arm_instruction = 0b1110_0010_0000_0000_0000_1111_0000_0000;
        let current_instruction = self.arm_current_execute;

        arm_instruction |= 13 << 12;
        arm_instruction |= 13 << 16;
        arm_instruction |= nn << 0;

        arm_instruction |= if opcode == 0 {
            ArmAluOpcode::ADD as u32
        } else {
            ArmAluOpcode::SUB as u32
        } << 21;

        self.arm_current_execute = arm_instruction;
        self.arm_data_processing(req);
        self.arm_current_execute = current_instruction;
    }

    /// arm7_tdmi::thumb_push_pop_register
    ///
    /// Function to execute thumb.14 instructions. It works by translating the opcode to the
    /// corresponding arm one (arm block data transfer)
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    /// @param rsp [&MemoryResponse]: response from the memory
    pub fn thumb_push_pop_register(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let opcode = self.arm_current_execute.get_range(11, 11);
        let pc_bit = self.arm_current_execute.get_range(8, 8);
        let r_list = self.arm_current_execute.get_range(7, 0);

        let current_instruction = self.arm_current_execute;
        let mut arm_instruction = 0b1110_1000_0010_0000_0000_0000_0000_0000;

        // push case
        if opcode == 0 {
            arm_instruction |= 1 << 24;

            // add lr to the transfered elements
            if pc_bit == 1 {
                arm_instruction |= 1 << 14;
            }

        // pop case
        } else {
            arm_instruction |= 1 << 23;

            // add pc to the transfered elements
            if pc_bit == 1 {
                arm_instruction |= 1 << 15;
            }
        }

        arm_instruction |= opcode << 20;
        arm_instruction |= 13 << 16;
        arm_instruction |= r_list << 0;

        self.arm_current_execute = arm_instruction;
        self.arm_block_data_transfer(req, rsp);
        self.arm_current_execute = current_instruction;
    }

    /// arm7_tdmi::thumb_multiple_load_store
    ///
    /// Function to execute thumb.15 instructions. It works by translating the opcode to the
    /// corresponding arm one (arm block data transfer)
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    /// @param rsp [&MemoryResponse]: response from the memory
    pub fn thumb_multiple_load_store(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let opcode = self.arm_current_execute.get_range(11, 11);
        let rb = self.arm_current_execute.get_range(10, 8);
        let r_list = self.arm_current_execute.get_range(7, 0);

        let current_instruction = self.arm_current_execute;
        let mut arm_instruction = 0b1110_1000_1010_0000_0000_0000_0000_0000;

        arm_instruction |= opcode << 20;
        arm_instruction |= rb << 16;
        arm_instruction |= r_list << 0;

        self.arm_current_execute = arm_instruction;
        self.arm_block_data_transfer(req, rsp);
        self.arm_current_execute = current_instruction;
    }

    /// arm7_tdmi::thumb_branch
    ///
    /// Function to execute thumb.16 and thumb.17 instructions.
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    /// @param rsp [&MemoryResponse]: response from the memory
    pub fn thumb_branch(&mut self, req: &mut MemoryRequest, cond_branch: bool) {
        let opcode = self.arm_current_execute.get_range(11, 8);
        let nn = self.arm_current_execute.get_range(10, 0);

        let current_instruction = self.arm_current_execute;
        let mut arm_instruction = 0b0000_1010_0000_0000_0000_0000_0000_0000;

        if cond_branch {
            arm_instruction |= opcode << 28;
        } else {
            arm_instruction |= 0xe << 28;
        }

        arm_instruction |= nn << 0;

        self.arm_current_execute = arm_instruction;
        self.arm_branch(req);
        self.arm_current_execute = current_instruction;
    }

    /// arm7_tdmi::thumb_software_interrupt
    ///
    /// Function to execute thumb.17 instructions. It works by translating the opcode to the
    /// corresponding arm one (arm swi). In this case, the systems switches back to arm state.
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    /// @param rsp [&MemoryResponse]: response from the memory
    pub fn thumb_software_interrupt(&mut self, req: &mut MemoryRequest) {
        let swi_arm_instruction = 0xe6000000;
        let current_instruction = self.arm_current_execute;

        self.arm_current_execute = swi_arm_instruction;
        self.arm_swi(req);
        self.arm_current_execute = current_instruction;
    }

    /// arm7_tdmi::thumb_long_branch_with_link
    ///
    /// Function to execute thumb.19 instructions. Due to the peculiarities of this thumb
    /// instructions compared to arm functionalities, some special operations are implemented
    /// within the `arm_undefined` opcode, to be used on in thumb mode.
    ///
    /// @param req [&mut MemoryRequest]: request to be sent to the bus for the current cycle (might
    /// be modified by the function depending on what the current instruction does).
    /// @param rsp [&MemoryResponse]: response from the memory
    pub fn thumb_long_branch_with_link(&mut self, req: &mut MemoryRequest) {
        let opcode = self.arm_current_execute.get_range(11, 11);
        let nn = self.arm_current_execute.get_range(10, 0);

        let current_instruction = self.arm_current_execute;
        let mut arm_instruction = 0b1110_0110_0000_0000_0000_0000_0001_0000;

        arm_instruction |= opcode << 20;
        arm_instruction |= nn << 8;

        self.arm_current_execute = arm_instruction;
        self.arm_undefined(req);
        self.arm_current_execute = current_instruction;
    }
}
