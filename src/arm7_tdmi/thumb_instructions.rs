use crate::arm7_tdmi::instruction::barrel_shifter;
use crate::arm7_tdmi::instruction::ArmAluOpcode;
use crate::arm7_tdmi::register_file::ConditionCodeFlag;
use crate::arm7_tdmi::{InstructionStep, ARM7TDMI};
use crate::bus::{BusCycle, MemoryRequest, MemoryResponse, TransferSize};
use crate::common::BitOperation;

impl ARM7TDMI {
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

    pub fn thumb_add_subtract(&mut self, req: &mut MemoryRequest) {
        let opcode = self.arm_current_execute.get_range(10, 9);
        let rn = self.arm_current_execute.get_range(8, 6);
        let rs = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);

        let mut arm_instruction = 0b1110_0000_0001_0000_0000_0000_0000_0000;
        let current_instruction = self.arm_current_execute;

        if opcode & 1 == 0 {
            arm_instruction |= 0x4 << 21;
        } else {
            arm_instruction |= 0x2 << 21;
        }

        if opcode < 2 {
            arm_instruction = arm_instruction.set_bit(4);
        } else {
            arm_instruction = arm_instruction.set_bit(25);
        }
        arm_instruction |= rs << 16;
        arm_instruction |= rd << 12;
        arm_instruction |= rn << 0;

        self.arm_current_execute = arm_instruction;
        self.arm_data_processing(req);
        self.arm_current_execute = current_instruction;
    }

    pub fn thumb_alu_immediate(&mut self) {
        let opcode = self.arm_current_execute.get_range(12, 11);
        let rd = self.arm_current_execute.get_range(10, 8);
        let nn = self.arm_current_execute.get_range(7, 0);
        let (result, c_flag, v_flag): (u32, bool, bool);

        if opcode == 0 {
            (result, c_flag, v_flag) = self.alu_operation(0, nn, ArmAluOpcode::MOV);
            self.rf.write_register(rd, result);
            self.update_flags(result, ArmAluOpcode::MOV, rd, c_flag, false, v_flag);
        } else if opcode == 1 {
            (result, c_flag, v_flag) =
                self.alu_operation(self.rf.get_register(rd, 0), nn, ArmAluOpcode::CMP);
            self.update_flags(result, ArmAluOpcode::CMP, rd, c_flag, false, v_flag);
        } else if opcode == 2 {
            (result, c_flag, v_flag) =
                self.alu_operation(self.rf.get_register(rd, 0), nn, ArmAluOpcode::ADD);
            self.rf.write_register(rd, result);
            self.update_flags(result, ArmAluOpcode::ADD, rd, c_flag, false, v_flag);
        } else {
            (result, c_flag, v_flag) =
                self.alu_operation(self.rf.get_register(rd, 0), nn, ArmAluOpcode::SUB);
            self.rf.write_register(rd, result);
            self.update_flags(result, ArmAluOpcode::SUB, rd, c_flag, false, v_flag);
        }
    }
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

        // shift case
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

        // neg case
        } else if opcode == 0x9 {
            arm_instruction |= 0x3 << 21;
            arm_instruction |= rd << 12;
            arm_instruction |= rs << 16;
            arm_instruction = arm_instruction.set_bit(25);
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

    pub fn thumb_hi_register_bx(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let opcode = self.arm_current_execute.get_range(9, 8);
        let msbd = self.arm_current_execute.get_range(7, 7);
        let msbs = self.arm_current_execute.get_range(6, 6);
        let rs = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);

        let full_rd = if msbd == 1 { rd | 0x8 } else { rd };
        let full_rs = if msbs == 1 { rs | 0x8 } else { rs };

        let bx_address = self.rf.get_register(full_rs, 4);

        if self.instruction_step == InstructionStep::STEP0 {
            if opcode == 0 {
                let op1 = self.rf.get_register(full_rd, 4);
                let op2 = self.rf.get_register(full_rs, 4);
                let (result, _, _) = self.alu_operation(op1, op2, ArmAluOpcode::ADD);
                self.rf.write_register(full_rd, result);
                if full_rd == 15 {
                    self.rf
                        .write_register(full_rd, self.rf.get_register(15, 0) & !2);
                    self.arm_instruction_queue.clear();
                    req.bus_cycle = BusCycle::NONSEQUENTIAL;
                    self.data_is_fetch = false;
                    self.instruction_step = InstructionStep::STEP1;
                }
            } else if opcode == 1 {
                let op1 = self.rf.get_register(full_rd, 4);
                let op2 = self.rf.get_register(full_rs, 4);
                let (result, c_flag, v_flag) = self.alu_operation(op1, op2, ArmAluOpcode::CMP);
                self.rf.write_register(full_rd, result);
                self.update_flags(result, ArmAluOpcode::CMP, full_rd, c_flag, true, v_flag);
            } else if opcode == 2 {
                let op1 = self.rf.get_register(full_rd, 4);
                let op2 = self.rf.get_register(full_rs, 4);
                let (result, _, _) = self.alu_operation(op1, op2, ArmAluOpcode::MOV);
                self.rf.write_register(full_rd, result);
                if full_rd == 15 {
                    self.rf
                        .write_register(full_rd, self.rf.get_register(15, 0) & !2);
                    self.arm_instruction_queue.clear();
                    req.bus_cycle = BusCycle::NONSEQUENTIAL;
                    self.data_is_fetch = false;
                    self.instruction_step = InstructionStep::STEP1;
                }
            } else {
                self.arm_instruction_queue.clear();
                req.bus_cycle = BusCycle::NONSEQUENTIAL;
                self.data_is_fetch = false;
                self.instruction_step = InstructionStep::STEP3;
            }
        } else if self.instruction_step == InstructionStep::STEP1 {
            req.mas = TransferSize::WORD;
            req.address = self.rf.get_register(15, 0);
            req.bus_cycle = BusCycle::SEQUENTIAL;
            self.data_is_fetch = false;
            self.instruction_step = InstructionStep::STEP2;
        } else if self.instruction_step == InstructionStep::STEP2 {
            req.mas = TransferSize::WORD;
            req.address = self.rf.get_register(15, 4);
            self.arm_instruction_queue.push_back(rsp.data);
            self.rf
                .write_register(15, (self.rf.get_register(15, 0)).wrapping_sub(4));
            let _ = self.rf.write_cpsr(self.rf.get_cpsr().clear_bit(5));
        } else if self.instruction_step == InstructionStep::STEP3 {
            if bx_address.is_bit_set(0) {
                req.mas = TransferSize::HALFWORD;
            } else {
                req.mas = TransferSize::WORD;
            }
            req.address = bx_address;
            req.bus_cycle = BusCycle::SEQUENTIAL;
            self.data_is_fetch = false;
            self.instruction_step = InstructionStep::STEP4;
        } else if self.instruction_step == InstructionStep::STEP4 {
            if msbd == 1 {
                self.rf.write_register(14, self.rf.get_register(15, 0))
            }
            if bx_address.is_bit_set(0) {
                if self.last_used_address.is_bit_clear(1) {
                    self.arm_instruction_queue
                        .push_back(rsp.data.get_range(15, 0));
                } else {
                    self.arm_instruction_queue
                        .push_back(rsp.data.get_range(31, 16));
                }
                req.mas = TransferSize::HALFWORD;
                req.address = bx_address.wrapping_add(2);
                self.rf.write_register(15, bx_address.wrapping_sub(2));
                let _ = self.rf.write_cpsr(self.rf.get_cpsr().set_bit(5));
            } else {
                req.mas = TransferSize::WORD;
                req.address = bx_address.wrapping_add(4);
                self.arm_instruction_queue.push_back(rsp.data);
                self.rf.write_register(15, bx_address.wrapping_sub(4));
                let _ = self.rf.write_cpsr(self.rf.get_cpsr().clear_bit(5));
            }
            req.bus_cycle = BusCycle::SEQUENTIAL;
            self.instruction_step = InstructionStep::STEP0;
        } else {
            panic!("Wrong step for THUMB HI REGISTER BX");
        }
    }

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

    pub fn thumb_load_store_reg_offset(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let opcode = self.arm_current_execute.get_range(11, 10);
        let ro = self.arm_current_execute.get_range(8, 6);
        let rb = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);
        let current_instruction = self.arm_current_execute;
        let mut arm_instruction = 0b1110_0111_1000_0000_0000_0000_0000_0000;

        if opcode & 1 == 1 {
            arm_instruction = arm_instruction.set_bit(22);
        }

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

    pub fn thumb_load_store_sign_ext(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let opcode = self.arm_current_execute.get_range(11, 10);
        let ro = self.arm_current_execute.get_range(8, 6);
        let rb = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);
        let current_instruction = self.arm_current_execute;
        let mut arm_instruction = 0b1110_0001_1000_0000_0000_0000_1000_0000;

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

    pub fn thumb_load_store_imm_offset(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let opcode = self.arm_current_execute.get_range(12, 11);
        let offset = if opcode > 1 {
            self.arm_current_execute.get_range(10, 6)
        } else {
            self.arm_current_execute.get_range(10, 6) * 4
        };
        let rb = self.arm_current_execute.get_range(5, 3);
        let rd = self.arm_current_execute.get_range(2, 0);

        let current_instruction = self.arm_current_execute;
        let mut arm_instruction = 0b1110_0101_1000_0000_0000_0000_0000_0000;

        if opcode > 1 {
            arm_instruction = arm_instruction.set_bit(22);
        }

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

    pub fn thumb_load_address(&mut self) {
        let opcode = self.arm_current_execute.get_range(11, 11);
        let rd = self.arm_current_execute.get_range(10, 8);
        let nn = self.arm_current_execute.get_range(7, 0) << 2;

        if opcode == 0 {
            self.rf
                .write_register(rd, (self.rf.get_register(15, 4) & !2).wrapping_add(nn));
        } else {
            self.rf
                .write_register(rd, self.rf.get_register(13, 0).wrapping_add(nn));
        }
    }

    pub fn thumb_add_offset_to_sp(&mut self) {
        let opcode = self.arm_current_execute.get_range(7, 7);
        let nn = self.arm_current_execute.get_range(6, 0) << 2;

        if opcode == 0 {
            self.rf
                .write_register(13, self.rf.get_register(13, 0).wrapping_add(nn));
        } else {
            self.rf
                .write_register(13, self.rf.get_register(13, 0).wrapping_add(nn));
        }
    }

    pub fn thumb_push_pop_register(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        let opcode = self.arm_current_execute.get_range(11, 11);
        let pc_bit = self.arm_current_execute.get_range(8, 8);
        let r_list = self.arm_current_execute.get_range(7, 0);

        let current_instruction = self.arm_current_execute;
        let mut arm_instruction = 0b1110_1000_0010_0000_0000_0000_0000_0000;

        if opcode == 0 {
            arm_instruction |= 1 << 24;
            if pc_bit == 1 {
                arm_instruction |= 1 << 14;
            }
        } else {
            arm_instruction |= 1 << 23;
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

    pub fn thumb_branch(&mut self, req: &mut MemoryRequest, cond_branch: bool) {
        let opcode = self.arm_current_execute.get_range(11, 8);
        let offset = if cond_branch {
            let nn = self.arm_current_execute.get_range(7, 0);
            if nn.is_bit_set(7) {
                nn | 0xffffff00
            } else {
                nn
            }
        } else {
            let nn = self.arm_current_execute.get_range(10, 0);
            if nn.is_bit_set(10) {
                nn | 0xfffff100
            } else {
                nn
            }
        } << 1;

        if cond_branch && !self.rf.check_condition_code(opcode) {
            return;
        }

        if self.instruction_step == InstructionStep::STEP0 {
            // modify r15
            self.rf
                .write_register(15, self.rf.get_register(15, 4).wrapping_add(offset) & !1);
            self.arm_instruction_queue.clear();
            req.bus_cycle = BusCycle::NONSEQUENTIAL;
            self.data_is_fetch = false;
            self.instruction_step = InstructionStep::STEP1;
        } else if self.instruction_step == InstructionStep::STEP1 {
            req.address = self.rf.get_register(15, 0);
            self.instruction_step = InstructionStep::STEP2;
        } else if self.instruction_step == InstructionStep::STEP2 {
            req.address = self.rf.get_register(15, 2);
            self.rf
                .write_register(15, (self.rf.get_register(15, 0)).wrapping_sub(2));
            self.instruction_step = InstructionStep::STEP0;
        } else {
            panic!("Wrong instruction step for THUMB BRANCH")
        }
    }

    pub fn thumb_software_interrupt(&mut self, req: &mut MemoryRequest) {
        let swi_arm_instruction = 0xe6000000;
        let current_instruction = self.arm_current_execute;

        self.arm_current_execute = swi_arm_instruction;
        self.arm_swi(req);
        self.arm_current_execute = current_instruction;
    }
    // THUMB.19: long branch with link
    // This may be used to call (or jump) to a subroutine, return address is saved in LR (R14).
    // Unlike all other THUMB mode instructions, this instruction occupies 32bit of memory which are split into two 16bit THUMB opcodes.
    //  First Instruction - LR = PC+4+(nn SHL 12)
    //   15-11  Must be 11110b for BL/BLX type of instructions
    //   10-0   nn - Upper 11 bits of Target Address
    //  Second Instruction - PC = LR + (nn SHL 1), and LR = PC+2 OR 1 (and BLX: T=0)
    //   15-11  Opcode
    //           11111b: BL label   ;branch long with link
    //           11101b: BLX label  ;branch long with link switch to ARM mode (ARM9)
    //   10-0   nn - Lower 11 bits of Target Address (BLX: Bit0 Must be zero)
    // The destination address range is (PC+4)-400000h..+3FFFFEh, ie. PC+/-4M.
    // Target must be halfword-aligned. As Bit 0 in LR is set, it may be used to return by a BX LR instruction (keeping CPU in THUMB mode).
    // Return: No flags affected, PC adjusted, return address in LR.
    // Execution Time: 3S+1N (first opcode 1S, second opcode 2S+1N).
    // Note: Exceptions may or may not occur between first and second opcode, this is "implementation defined" (unknown how this is implemented in GBA and NDS).
    // Using only the 2nd half of BL as "BL LR+imm" is possible (for example, Mario Golf Advance Tour for GBA uses opcode F800h as "BL LR+0").
    pub fn thumb_long_branch_with_link(&mut self, req: &mut MemoryRequest, rsp: &MemoryResponse) {
        if self.instruction_step == InstructionStep::STEP0 {
            self.data_is_fetch = false;
            req.address = self.rf.get_register(15, 2);
            req.bus_cycle = BusCycle::NONSEQUENTIAL;
            self.instruction_step = InstructionStep::STEP1;
        } else if self.instruction_step == InstructionStep::STEP1 {
            let mut received_data = rsp.data;
            if self.last_used_address.is_bit_set(1) {
                received_data >>= 16;
            } else {
                received_data &= 0xffff;
            }
            let mut dest_address = (self.arm_current_execute.get_range(10, 0) << 12)
                + (received_data.get_range(10, 0) << 1)
                + self.rf.get_register(15, 4);

            if dest_address.is_bit_set(22) {
                dest_address |= 0xffc00000;
            }

            self.rf.write_register(14, self.rf.get_register(15, 2));
            self.rf.write_register(15, dest_address);

            self.data_is_fetch = false;
            self.arm_instruction_queue.clear();
            req.bus_cycle = BusCycle::NONSEQUENTIAL;
            self.instruction_step = InstructionStep::STEP2;
        } else if self.instruction_step == InstructionStep::STEP2 {
            req.address = self.rf.get_register(15, 0);
            self.instruction_step = InstructionStep::STEP3;
        } else if self.instruction_step == InstructionStep::STEP3 {
            req.address = self.rf.get_register(15, 2);
            self.rf
                .write_register(15, self.rf.get_register(15, 0).wrapping_sub(2));
            self.instruction_step = InstructionStep::STEP0;
        } else {
            panic!("Wrong step for instruction BL THUMB");
        }
    }
}
